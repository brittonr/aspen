//! Event handling for the TUI.
//!
//! Provides async event loop for keyboard, mouse, and tick events.

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use tokio::sync::mpsc;
use tokio::time::interval;

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
    rx: mpsc::UnboundedReceiver<Event>,
    /// Sender handle kept for cloning.
    #[allow(dead_code)]
    tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate.
    ///
    /// Spawns background tasks for:
    /// - Periodic tick events
    /// - Terminal input events
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Spawn tick generator
        let tick_tx = tx.clone();
        tokio::spawn(async move {
            let mut ticker = interval(tick_rate);
            loop {
                ticker.tick().await;
                if tick_tx.send(Event::Tick).is_err() {
                    break;
                }
            }
        });

        // Spawn input event handler
        let input_tx = tx.clone();
        tokio::spawn(async move {
            loop {
                // Poll for events with 50ms timeout
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    if let Ok(evt) = event::read() {
                        let event = match evt {
                            CrosstermEvent::Key(key) => Event::Key(key),
                            CrosstermEvent::Mouse(mouse) => Event::Mouse(mouse),
                            CrosstermEvent::Resize(w, h) => Event::Resize(w, h),
                            _ => continue,
                        };
                        if input_tx.send(event).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self { rx, tx }
    }

    /// Get the next event.
    ///
    /// Returns None if the channel is closed.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
