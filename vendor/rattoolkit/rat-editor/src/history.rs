//! Command history and history navigation

use crate::{Editor, MAX_HISTORY};

impl Editor {
    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if self.history_index.is_none() {
            self.saved_input = Some(self.lines.clone());
            self.history_index = Some(self.history.len() - 1);
        } else if let Some(idx) = self.history_index
            && idx > 0
        {
            self.history_index = Some(idx - 1);
        }

        if let Some(idx) = self.history_index {
            let text = &self.history[idx];
            self.lines = text.lines().map(|s| s.to_string()).collect();
            if self.lines.is_empty() {
                self.lines.push(String::new());
            }
            self.cursor_line = self.lines.len() - 1;
            self.cursor_col = self.lines[self.cursor_line].len();
        }
    }

    pub fn history_down(&mut self) {
        if let Some(idx) = self.history_index {
            if idx < self.history.len() - 1 {
                self.history_index = Some(idx + 1);
                let text = &self.history[idx + 1];
                self.lines = text.lines().map(|s| s.to_string()).collect();
                if self.lines.is_empty() {
                    self.lines.push(String::new());
                }
            } else {
                self.history_index = None;
                if let Some(saved) = self.saved_input.take() {
                    self.lines = saved;
                }
            }
            self.cursor_line = self.lines.len() - 1;
            self.cursor_col = self.lines[self.cursor_line].len();
        }
    }

    pub fn submit(&mut self) -> Option<String> {
        let text = self.lines.join("\n");
        if text.trim().is_empty() {
            return None;
        }
        self.history.push_back(text.clone());
        if self.history.len() > MAX_HISTORY {
            self.history.pop_front();
        }
        self.clear();
        Some(text)
    }
}