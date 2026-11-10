//! Cancellable bounded stdin ownership for an explicitly owned serving session.

pub(super) enum Input {
    Frame(Vec<u8>),
    End,
    Error(&'static str),
}

enum ReaderEnd {
    Cancelled,
    ConsumerClosed,
}

struct Reader {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<Result<(), ReaderEnd>>>,
}

impl Reader {
    fn start(sender: tokio::sync::mpsc::Sender<Input>) -> molten::error::Result<Self> {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopped = std::sync::Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("node-service-host-stdin".into())
            .spawn(move || read_frames(sender, &stopped))
            .map_err(molten::error::Failure::from)?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }

    fn close(&mut self) -> molten::error::Result<()> {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        match thread.join() {
            // Closing the owned receiver or stop flag cancels input, not a protected operation.
            Ok(Ok(()) | Err(ReaderEnd::Cancelled | ReaderEnd::ConsumerClosed)) => Ok(()),
            Err(_) => Err(molten::error::Failure::invalid_harness("control input reader panicked")),
        }
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        if self.close().is_err() {
            eprintln!("control input reader cleanup failed");
        }
    }
}

/// No blocking_send: cancellation must also wake a reader whose bounded queue is full.
fn forward(
    sender: &tokio::sync::mpsc::Sender<Input>,
    stopped: &std::sync::atomic::AtomicBool,
    mut input: Input,
) -> Result<(), ReaderEnd> {
    loop {
        if stopped.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(ReaderEnd::Cancelled);
        }
        match sender.try_send(input) {
            Ok(()) => return Ok(()),
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => return Err(ReaderEnd::ConsumerClosed),
            Err(tokio::sync::mpsc::error::TrySendError::Full(pending)) => input = pending,
        }
        std::thread::park_timeout(std::time::Duration::from_millis(100));
    }
}

fn read_frames(
    sender: tokio::sync::mpsc::Sender<Input>,
    stopped: &std::sync::atomic::AtomicBool,
) -> Result<(), ReaderEnd> {
    let stdin = std::io::stdin();
    let timeout = rustix::event::Timespec {
        tv_sec: 0,
        tv_nsec: 100_000_000,
    };
    let mut frame = Vec::new();
    let mut bytes = [0_u8; 4096];
    while !stopped.load(std::sync::atomic::Ordering::Relaxed) {
        let mut fds = [rustix::event::PollFd::new(&stdin, rustix::event::PollFlags::IN)];
        match rustix::event::poll(&mut fds, Some(&timeout)) {
            Ok(0) | Err(rustix::io::Errno::INTR) => continue,
            Err(_) => return forward(&sender, stopped, Input::Error("stdin poll failed")),
            Ok(_) => {}
        }
        // Read the descriptor directly: Stdin's internal buffering would hide bytes from poll.
        let count = match rustix::io::read(&stdin, &mut bytes) {
            Ok(0) => {
                return forward(
                    &sender,
                    stopped,
                    if frame.is_empty() {
                        Input::End
                    } else {
                        Input::Error("unterminated control frame")
                    },
                );
            }
            Ok(count) => count,
            Err(rustix::io::Errno::INTR | rustix::io::Errno::AGAIN) => continue,
            Err(_) => return forward(&sender, stopped, Input::Error("stdin read failed")),
        };
        for &byte in &bytes[..count] {
            if byte == b'\n' {
                forward(&sender, stopped, Input::Frame(std::mem::take(&mut frame)))?;
            } else {
                if frame.len() >= super::hosting::control::MAX_FRAME_BYTES {
                    return forward(&sender, stopped, Input::Error("control frame exceeds byte bound"));
                }
                frame.push(byte);
            }
        }
    }
    Ok(())
}

pub(super) async fn serve(
    hosted: &mut super::hosting::Session,
    bound: molten::node_daemon::BoundControlLiveListener<'_>,
) -> molten::error::Result<Option<molten::node_daemon::ControlLiveServe>> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let mut reader = Reader::start(sender)?;
    let shutdown = tokio::sync::Notify::new();
    let listener = bound.serve_until(shutdown.notified());
    tokio::pin!(listener);
    let result = loop {
        tokio::select! {
            biased;
            result = &mut listener => break result,
            input = receiver.recv() => {
                match input {
                    Some(Input::Frame(frame)) => {
                        if let Err(error) = emit(&hosted.respond(&frame)) {
                            shutdown.notify_one();
                            break listener.await.and_then(|_| Err(error));
                        }
                    }
                    Some(Input::Error(detail)) => {
                        let response = emit(&super::hosting::control::error("invalid", 0, detail));
                        shutdown.notify_one();
                        let closed = listener.await;
                        break response.and(closed).and_then(|_| Err(
                            molten::error::Failure::invalid_harness(detail)));
                    }
                    Some(Input::End) => {
                        shutdown.notify_one();
                        break listener.await;
                    }
                    None => {
                        shutdown.notify_one();
                        break listener.await.and_then(|_| Err(
                            molten::error::Failure::invalid_harness("control reader ended without EOF")));
                    }
                }
            }
        }
    };
    drop(receiver);
    // Always join; a join failure takes precedence over a successfully closed listener.
    reader.close().and(result)
}

fn emit(value: &preserves::IOValue) -> molten::error::Result<()> {
    use std::io::Write;
    let text = molten::preserves_rail::to_text(value)?;
    if text.len() > super::hosting::control::MAX_FRAME_BYTES {
        return Err(molten::error::Failure::invalid_harness("control response exceeds byte bound"));
    }
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{text}").map_err(molten::error::Failure::from)?;
    stdout.flush().map_err(molten::error::Failure::from)
}

#[cfg(test)]
mod tests {
    #[test]
    fn full_input_queue_cancels_without_a_consumer_read() {
        const CANCELLATION_SECONDS: u64 = 10;
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        assert!(sender.try_send(super::Input::End).is_ok());
        let stopped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let signal = std::sync::Arc::clone(&stopped);
        let (completed, result) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || {
            completed.send(super::forward(&sender, &signal, super::Input::Frame(Vec::new())))
        });
        stopped.store(true, std::sync::atomic::Ordering::Relaxed);
        let observed = result.recv_timeout(std::time::Duration::from_secs(CANCELLATION_SECONDS));
        // Release the blocked producer even if a regression missed cancellation.
        drop(receiver);
        assert!(thread.join().expect("owned producer joins").is_ok());
        assert!(matches!(observed, Ok(Err(super::ReaderEnd::Cancelled))));
    }
}
