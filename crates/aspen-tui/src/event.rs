//! Event handling for the TUI.
//!
//! Provides async event loop for keyboard, mouse, and tick events.

use std::time::Duration;

use crossterm::event::Event as CrosstermEvent;
use crossterm::event::KeyEvent;
use crossterm::event::MouseEvent;
use crossterm::event::{self};
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

/// TUI events.
#[derive(Debug)]
pub enum Event {
    /// Periodic tick for refreshing.
    Tick,
    /// Keyboard event.
    Key(KeyEvent),
    /// Mouse event.
    Mouse(MouseEvent),
    /// Terminal resize.
    Resize(u16, u16),
}

/// Event handler that manages the event loop.
pub struct EventHandler {
    /// Receiver for events.
    rx: mpsc::Receiver<Event>,
    /// Cancellation token for graceful shutdown.
    cancel: CancellationToken,
    /// Tick generator task handle.
    tick_task: tokio::task::JoinHandle<()>,
    /// Input event handler task handle.
    input_task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate.
    ///
    /// Spawns background tasks for:
    /// - Periodic tick events
    /// - Terminal input events
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel(1000);
        let cancel = CancellationToken::new();

        // Spawn tick generator
        let tick_tx = tx.clone();
        let tick_cancel = cancel.clone();
        let tick_task = tokio::spawn(async move {
            let mut ticker = interval(tick_rate);
            loop {
                tokio::select! {
                    _ = tick_cancel.cancelled() => break,
                    _ = ticker.tick() => {
                        if tick_tx.try_send(Event::Tick).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // Spawn input event handler
        let input_tx = tx;
        let input_cancel = cancel.clone();
        let input_task = tokio::spawn(async move {
            loop {
                if input_cancel.is_cancelled() {
                    break;
                }
                // Poll for events with 50ms timeout
                if event::poll(Duration::from_millis(50)).unwrap_or(false)
                    && let Ok(evt) = event::read()
                {
                    let event = match evt {
                        CrosstermEvent::Key(key) => Event::Key(key),
                        CrosstermEvent::Mouse(mouse) => Event::Mouse(mouse),
                        CrosstermEvent::Resize(w, h) => Event::Resize(w, h),
                        _ => continue,
                    };
                    if input_tx.try_send(event).is_err() {
                        break;
                    }
                }
            }
        });

        Self {
            rx,
            cancel,
            tick_task,
            input_task,
        }
    }

    /// Get the next event.
    ///
    /// Returns None if the channel is closed.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    /// Shut down the event handler gracefully.
    ///
    /// Cancels both spawned tasks and awaits their completion.
    pub async fn shutdown(self) {
        self.cancel.cancel();
        let _ = self.tick_task.await;
        let _ = self.input_task.await;
    }
}
