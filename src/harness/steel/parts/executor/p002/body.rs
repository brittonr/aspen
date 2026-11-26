#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MeterState {
    fuel_remaining: u64,
    allocated_bytes: u64,
    hostcall_count: u64,
    exhaustion: Option<Exhaustion>,
}

/// The deterministic resource meter shared by every primitive of one run.
#[derive(Clone)]
struct Meter {
    state: Shared<Lock<MeterState>>,
    interrupt: Shared<dyn Fn() + Send + Sync>,
}

impl Meter {
    fn exhaust(&self, state: &mut MeterState, exhaustion: Exhaustion) -> SteelError {
        if state.exhaustion.is_none() {
            state.exhaustion = Some(exhaustion);
        }
        // Stop the VM at its next instruction as well, so no script path can continue past the ceiling.
        (self.interrupt)();
        sandbox_error(format!("Steel sandbox {} limit exhausted", exhaustion.as_str()))
    }

    fn charge(&self, fuel: u64, bytes: u64) -> SteelResult<()> {
        self.charge_allocation(fuel, Some(bytes))
    }

    /// Charges `fuel` and an allocation of `bytes`. `None` is an allocation too large to represent as a
    /// byte count; once its fuel is charged it exhausts memory like any allocation past the limit.
    fn charge_allocation(&self, fuel: u64, bytes: Option<u64>) -> SteelResult<()> {
        let mut state = self.state.lock().map_err(|_| sandbox_error("Steel sandbox meter is unavailable"))?;
        if let Some(exhaustion) = state.exhaustion {
            return Err(self.exhaust(&mut state, exhaustion));
        }
        let Some(fuel_remaining) = state.fuel_remaining.checked_sub(fuel) else {
            state.fuel_remaining = 0;
            return Err(self.exhaust(&mut state, Exhaustion::Fuel));
        };
        let Some(allocated_bytes) = bytes
            .map(|bytes| state.allocated_bytes.saturating_add(bytes))
            .filter(|allocated_bytes| *allocated_bytes <= STEEL_MEMORY_LIMIT_BYTES)
        else {
            return Err(self.exhaust(&mut state, Exhaustion::Memory));
        };
        state.fuel_remaining = fuel_remaining;
        state.allocated_bytes = allocated_bytes;
        Ok(())
    }

    /// Admits one hostcall before it is dispatched; the call past the limit is refused and never
    /// dispatched.
    // r[impl molten.bounded_execution.steel_sandbox_and_real_bounds]
    fn admit_hostcall(&self) -> SteelResult<()> {
        let mut state = self.state.lock().map_err(|_| sandbox_error("Steel sandbox meter is unavailable"))?;
        if let Some(exhaustion) = state.exhaustion {
            return Err(self.exhaust(&mut state, exhaustion));
        }
        if state.hostcall_count >= STEEL_MAX_HOSTCALLS {
            return Err(self.exhaust(&mut state, Exhaustion::Hostcalls));
        }
        state.hostcall_count += 1;
        Ok(())
    }

    /// Charges a produced string, refusing any string larger than the single-string ceiling.
    fn charge_string(&self, bytes: usize) -> SteelResult<()> {
        if bytes > STEEL_MAX_STRING_BYTES {
            let mut state = self.state.lock().map_err(|_| sandbox_error("Steel sandbox meter is unavailable"))?;
            return Err(self.exhaust(&mut state, Exhaustion::Memory));
        }
        self.charge_allocation(1, u64::try_from(bytes).ok())
    }

    fn charge_scalar(&self) -> SteelResult<()> {
        self.charge(1, STEEL_SCALAR_BYTES)
    }
}

fn sandbox_error(message: impl Into<String>) -> SteelError {
    SteelError::new(steel::rerrs::ErrorKind::Generic, message.into())
}

fn integer_overflow() -> SteelError {
    sandbox_error("Steel sandbox integer arithmetic overflowed")
}

/// Runs `work` on a dedicated thread whose stack does not depend on the caller's.
fn on_sandbox_thread<T: Send>(work: impl FnOnce() -> T + Send) -> std::result::Result<T, String> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("molten-steel-sandbox".to_string())
            .stack_size(STEEL_VM_STACK_BYTES)
            .spawn_scoped(scope, work)
            .map_err(|error| format!("Steel sandbox thread could not start: {error}"))?
            .join()
            .map_err(|_| "Steel sandbox thread panicked".to_string())
    })
}

/// Runs one actor step on the sandbox thread. The engine, every value it built, and their
/// (recursive) drops all stay on that thread.
// r[impl molten.bounded_execution.steel_sandbox_and_real_bounds]
fn run_vm(prepared: &Prepared, expected_operation: &str, allowed_hostcalls: &[String]) -> SteelRun {
    on_sandbox_thread(|| run_vm_on_current_thread(prepared, expected_operation, allowed_hostcalls)).unwrap_or_else(
        |detail| SteelRun {
            outcome: Err(detail),
            meter: MeterState {
                fuel_remaining: 0,
                allocated_bytes: 0,
                hostcall_count: 0,
                exhaustion: None,
            },
        },
    )
}

fn run_vm_on_current_thread(prepared: &Prepared, expected_operation: &str, allowed_hostcalls: &[String]) -> SteelRun {
    // A raw engine registers no prelude, modules, or ambient primitives; the sandbox table below is all
    // it has.
    let mut engine = Engine::new_raw_no_kernel();
    let controller = engine.get_thread_state_controller();
    let meter = Meter {
        state: Shared::new(Lock::new(MeterState {
            fuel_remaining: STEEL_FUEL_LIMIT,
            allocated_bytes: 0,
            hostcall_count: 0,
            exhaustion: None,
        })),
        interrupt: Shared::new(move || controller.interrupt()),
    };
    register_sandbox(&mut engine, &meter, expected_operation, allowed_hostcalls);
    let outcome = meter
        .charge(0, prepared.input_text.len() as u64)
        .and_then(|()| engine.run(prepared.program.clone()))
        .and_then(|_| prepared.input_text.clone().into_steelval())
        .and_then(|input| engine.call_function_by_name_with_args(&prepared.entry, vec![input]))
        .map(|value| value.to_string())
        .map_err(|error| error.to_string());
    match meter.state.lock() {
        Ok(state) => SteelRun { outcome, meter: *state },
        Err(_) => SteelRun {
            outcome: Err("Steel sandbox meter is unavailable".to_string()),
            meter: MeterState {
                fuel_remaining: 0,
                allocated_bytes: 0,
                hostcall_count: 0,
                exhaustion: None,
            },
        },
    }
}
