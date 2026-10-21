// Receiving-worker protocol: traffic alone is not a concrete adapter wake.
enum ReceiverInput {
    Delivery(Box<DeliveryBoundary>),
    Checkpoint(u64),
    FenceProbe(u64),
    CheckpointReleased(u64),
    Stop,
}
#[derive(Debug)]
enum ReceiverOutput {
    Delivery(Box<DeliveryBoundary>),
    Checkpoint { sequence: u64, deliveries: u64 },
    FenceProbe(u64),
}

fn receive_worker(
    input: std::sync::mpsc::Receiver<ReceiverInput>,
    sender: std::sync::mpsc::SyncSender<ReceiverOutput>,
) {
    let mut deliveries = 0_u64;
    let mut is_stopped = false;
    while !is_stopped {
        std::thread::park();
        // A spurious park return has no mailbox entry and makes no
        // observation. Checkpoints are FIFO behind prior real effects.
        while let Ok(message) = input.try_recv() {
            match message {
                ReceiverInput::Delivery(boundary) => {
                    deliveries += 1;
                    assert_eq!(boundary.sequence, deliveries);
                    sender.send(ReceiverOutput::Delivery(boundary)).expect("report real delivery");
                }
                ReceiverInput::FenceProbe(sequence) => {
                    sender.send(ReceiverOutput::FenceProbe(sequence)).expect("observe actual unpark probe");
                }
                ReceiverInput::Checkpoint(sequence) => {
                    // Confirmation is sent after the control unpark
                    // returns, so it cannot arrive after our ack and
                    // leave a permit masking a later missing wake.
                    assert!(
                        matches!(input.recv_timeout(WAKE_TIMEOUT).expect("control wake confirmation"), ReceiverInput::CheckpointReleased(released) if released == sequence)
                    );
                    // Drain an already pending control permit before
                    // acknowledging. The return value is not evidence
                    // of absence; sequence/packet accounting is. Always
                    // re-enter park before processing later commands.
                    std::thread::park_timeout(std::time::Duration::ZERO);
                    sender
                        .send(ReceiverOutput::Checkpoint { sequence, deliveries })
                        .expect("acknowledge receiving checkpoint");
                    break;
                }
                ReceiverInput::Stop => {
                    is_stopped = true;
                    break;
                }
                ReceiverInput::CheckpointReleased(_) => panic!("control confirmation without checkpoint"),
            }
        }
    }
}
