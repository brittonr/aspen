//! Shared free-scroll state for TUI panels.
//!
//! Wraps a `Cell<u16>` offset with standard scroll operations.
//! Used by panels that need interior-mutable scroll state (e.g., for
//! `&self` draw methods).

use std::cell::Cell;

/// Free-scroll state backed by a `Cell<u16>`.
///
/// Provides scroll operations with interior mutability, suitable for
/// panels whose `draw()` takes `&self` but needs to clamp the scroll
/// offset to the rendered content height.
#[derive(Debug)]
pub struct FreeScroll {
    offset: Cell<u16>,
}

impl Default for FreeScroll {
    fn default() -> Self {
        Self { offset: Cell::new(0) }
    }
}

impl FreeScroll {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current raw offset (may exceed content height; clamp before use).
    pub fn get(&self) -> u16 {
        self.offset.get()
    }

    /// Set offset directly.
    pub fn set(&self, v: u16) {
        self.offset.set(v);
    }

    pub fn scroll_up(&self, n: u16) {
        self.offset.set(self.offset.get().saturating_sub(n));
    }

    pub fn scroll_down(&self, n: u16) {
        self.offset.set(self.offset.get().saturating_add(n));
    }

    pub fn scroll_to_top(&self) {
        self.offset.set(0);
    }

    pub fn scroll_to_bottom(&self) {
        self.offset.set(u16::MAX);
    }

    /// Clamp offset to `[0, max_scroll]` and return the clamped value.
    ///
    /// Call this in `draw()` to prevent over-scroll:
    /// ```ignore
    /// let scroll = self.scroll.clamp(total_lines.saturating_sub(visible_height));
    /// ```
    pub fn clamp(&self, max_scroll: u16) -> u16 {
        let clamped = self.offset.get().min(max_scroll);
        self.offset.set(clamped);
        clamped
    }
}