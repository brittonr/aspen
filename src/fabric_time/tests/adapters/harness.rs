// Owning command and receiving-boundary harness.
struct Harness {
    owner: std::sync::Mutex<SchedulerRuntime<Effects>>,
    worker: Option<std::thread::JoinHandle<()>>,
    control: std::sync::mpsc::SyncSender<ReceiverInput>,
    received: std::sync::mpsc::Receiver<ReceiverOutput>,
    consumed: std::cell::Cell<u64>,
    checkpoint: std::cell::Cell<u64>,
}
impl Harness {
    fn new(profile: &AdmittedTimeProfile, generation: u64, keys: &[RunnableKey]) -> Self {
        // Commands consume each delivery before any control wake. A checkpoint
        // and its post-unpark confirmation may coexist, so input needs two slots.
        let (control, input) = std::sync::mpsc::sync_channel(2);
        // Each delivery/checkpoint acknowledgement is consumed before the next.
        let (sender, received) = std::sync::mpsc::sync_channel(1);
        let concrete = match profile.kind {
            TimeProfileKind::Live => ConcreteEffects::Live(ThreadSchedulerWakeAdapter::default()),
            TimeProfileKind::DeterministicSimulation => ConcreteEffects::Simulation(
                SimulationSchedulerWakeAdapter::activate(profile).expect("simulation delivery allocation"),
            ),
        };
        let effects = Effects {
            concrete,
            live_receiver: (profile.kind == TimeProfileKind::Live).then(|| control.clone()),
            simulation_boundaries: std::collections::VecDeque::new(),
            preparations: 0,
            deliveries: 0,
        };
        let mut owner = SchedulerRuntime::activate(context(profile, generation), profile.clone(), effects)
            .expect("owning activation");
        let worker = if profile.kind == TimeProfileKind::Live {
            let worker = std::thread::spawn(move || receive_worker(input, sender));
            let ConcreteEffects::Live(port) = &mut owner.effects_mut().concrete else {
                unreachable!()
            };
            for key in keys {
                port.register(key.clone(), worker.thread().clone()).expect("register receiving worker");
            }
            Some(worker)
        } else {
            None
        };
        Self {
            owner: std::sync::Mutex::new(owner),
            worker,
            control,
            received,
            consumed: std::cell::Cell::new(0),
            checkpoint: std::cell::Cell::new(0),
        }
    }
    fn snapshot(&self) -> Snapshot {
        let owner = self.owner.lock().expect("owner lock");
        (owner.state().clone(), owner.capacity_state().clone())
    }
    fn command(&self, command: SchedulerCommand) -> Result<SchedulerAction, SchedulerRuntimeError> {
        let result = self.owner.lock().expect("owner lock").command(command);
        if !matches!(result, Ok(SchedulerAction::Woken | SchedulerAction::Yielded)) {
            self.no_delivery();
        }
        result
    }
    fn choose(&self, key: &RunnableKey) -> Result<SchedulerChoice, SchedulerRuntimeError> {
        let result = self.owner.lock().expect("owner lock").choose(Some(key));
        self.no_delivery();
        result
    }
    fn receive_boundary(&self) -> DeliveryBoundary {
        let boundary = if self.worker.is_some() {
            // No control unpark/checkpoint precedes this observation: actual
            // progress must come from the real ThreadSchedulerWakeAdapter path.
            match self.received.recv_timeout(WAKE_TIMEOUT).expect("real receiving worker progress") {
                ReceiverOutput::Delivery(boundary) => boundary,
                checkpoint => panic!("delivery replaced by control traffic: {checkpoint:?}"),
            }
        } else {
            let mut owner = self.owner.lock().expect("consumer lock");
            let effects = owner.effects_mut();
            let ConcreteEffects::Simulation(port) = &mut effects.concrete else {
                unreachable!()
            };
            let index = port.drain_next().expect("independently drained simulation delivery");
            let boundary = effects.simulation_boundaries.pop_front().expect("actual simulation delivery boundary");
            assert_eq!(u64::try_from(boundary.occurrence).expect("occurrence fits simulation index"), index);
            boundary
        };
        assert_eq!(boundary.sequence, self.consumed.get() + 1);
        self.consumed.set(boundary.sequence);
        boundary
    }
    fn consume(&self, key: &RunnableKey) {
        let boundary = self.receive_boundary();
        assert!(
            boundary_is_committed(&boundary, key),
            "effect preceded scheduler/capacity publication: {boundary:?}"
        );
        assert_eq!(boundary.snapshot, self.snapshot());
        self.no_delivery();
    }
    fn no_delivery(&self) {
        {
            let mut owner = self.owner.lock().expect("effect boundary lock");
            let effects = owner.effects_mut();
            assert_eq!(effects.preparations, self.consumed.get(), "unexpected effect preparation for this command");
            assert_eq!(effects.deliveries, self.consumed.get(), "unexpected effect delivery for this command");
            if let ConcreteEffects::Simulation(port) = &mut effects.concrete {
                assert_eq!(port.drain_next(), None);
                assert!(effects.simulation_boundaries.is_empty());
            }
        }
        if let Some(worker) = &self.worker {
            let sequence = self.checkpoint.get() + 1;
            self.checkpoint.set(sequence);
            self.control.send(ReceiverInput::Checkpoint(sequence)).expect("send receiving checkpoint");
            // Explicit control wake only after command-boundary accounting. FIFO
            // acknowledgement cannot overtake an erroneous or coalesced effect.
            worker.thread().unpark();
            self.control
                .send(ReceiverInput::CheckpointReleased(sequence))
                .expect("confirm completed control unpark");
            match self.received.recv_timeout(WAKE_TIMEOUT).expect("receiving checkpoint must acknowledge") {
                ReceiverOutput::Checkpoint {
                    sequence: observed,
                    deliveries,
                } => {
                    assert_eq!(observed, sequence);
                    assert_eq!(deliveries, self.consumed.get());
                }
                delivery => panic!("unaccounted delivery before checkpoint: {delivery:?}"),
            }
        }
    }
    fn wake(&self, key: &RunnableKey, priority: i32) {
        assert_eq!(
            self.command(SchedulerCommand::Wake {
                key: key.clone(),
                priority
            })
            .expect("admitted wake"),
            SchedulerAction::Woken
        );
        self.consume(key);
    }
    fn usage(&self, active: u64, ready: u64) {
        let snapshot = self.snapshot();
        assert_eq!((snapshot.1.runnable_usage, snapshot.1.queue_usage), (active, ready));
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        // Successful tests must checkpoint even their final command before stop;
        // never let termination swallow an outstanding incorrect delivery.
        if !std::thread::panicking() {
            self.no_delivery();
        }
        if let Some(worker) = self.worker.take() {
            let _ = self.control.send(ReceiverInput::Stop);
            worker.thread().unpark();
            let result = worker.join();
            if !std::thread::panicking() {
                result.expect("join receiving worker");
            }
        }
    }
}
