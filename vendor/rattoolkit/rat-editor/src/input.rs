//! Input handling, text editing, and cursor movement

use crate::Editor;

impl Editor {
    pub fn insert_char(&mut self, c: char) {
        if c == '\n' {
            let current = &self.lines[self.cursor_line];
            let remainder = current[self.cursor_col..].to_string();
            self.lines[self.cursor_line].truncate(self.cursor_col);
            self.lines.insert(self.cursor_line + 1, remainder);
            self.cursor_line += 1;
            self.cursor_col = 0;
        } else {
            self.lines[self.cursor_line].insert(self.cursor_col, c);
            self.cursor_col += c.len_utf8();
        }
        self.history_index = None;
    }

    /// Insert a string at the cursor position (used for paste operations)
    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            // Skip carriage returns – they are invisible control chars that
            // mess up rendering when pasted from Windows-style line endings.
            if c == '\r' {
                continue;
            }
            self.insert_char(c);
        }
    }

    pub fn delete_back(&mut self) {
        if self.cursor_col > 0 {
            // Find the previous char boundary
            let line = &self.lines[self.cursor_line];
            let prev = line[..self.cursor_col].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
            self.lines[self.cursor_line].remove(prev);
            self.cursor_col = prev;
        } else if self.cursor_line > 0 {
            let current = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&current);
        }
        self.history_index = None;
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_col < self.lines[self.cursor_line].len() {
            self.lines[self.cursor_line].remove(self.cursor_col);
        } else if self.cursor_line < self.lines.len() - 1 {
            let next = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next);
        }
        self.history_index = None;
    }

    pub fn delete_word_back(&mut self) {
        let line = &self.lines[self.cursor_line];
        let before_cursor = &line[..self.cursor_col];

        // Find start of word
        let mut new_pos = self.cursor_col;
        let mut found_word = false;
        for (i, c) in before_cursor.char_indices().rev() {
            if c.is_whitespace() {
                if found_word {
                    new_pos = i + 1;
                    break;
                }
            } else {
                found_word = true;
            }
            new_pos = i;
        }

        self.lines[self.cursor_line].replace_range(new_pos..self.cursor_col, "");
        self.cursor_col = new_pos;
        self.history_index = None;
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            let line = &self.lines[self.cursor_line];
            self.cursor_col = line[..self.cursor_col].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_col < self.lines[self.cursor_line].len() {
            let line = &self.lines[self.cursor_line];
            self.cursor_col = line[self.cursor_col..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_col + i)
                .unwrap_or(line.len());
        } else if self.cursor_line < self.lines.len() - 1 {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_line].len();
    }

    /// Move the cursor to the position corresponding to a mouse click
    /// inside the editor's inner area. `rel_col` and `rel_row` are relative
    /// to the inner area (after removing borders). `inner_width` is the
    /// available character width and `indicator_len` is the prompt prefix
    /// length on the first line.
    pub fn click_to_cursor(&mut self, rel_col: u16, rel_row: u16, inner_width: usize, indicator_len: usize) {
        if inner_width == 0 {
            return;
        }

        let target_visual_row = rel_row as usize;
        let target_col = rel_col as usize;
        let mut visual_row: usize = 0;

        for (line_idx, line) in self.lines.iter().enumerate() {
            let prefix_len = if line_idx == 0 { indicator_len } else { 2 };
            let content_len = prefix_len + line.len();
            let wrapped_rows = if content_len == 0 {
                1
            } else {
                content_len.div_ceil(inner_width)
            };

            if target_visual_row < visual_row + wrapped_rows {
                // The click is in this logical line
                let row_within_line = target_visual_row - visual_row;
                let abs_offset = row_within_line * inner_width + target_col;
                let char_offset = abs_offset.saturating_sub(prefix_len);
                self.cursor_line = line_idx;
                self.cursor_col = char_offset.min(line.len());
                // Snap to a char boundary
                while self.cursor_col > 0 && !line.is_char_boundary(self.cursor_col) {
                    self.cursor_col -= 1;
                }
                return;
            }
            visual_row += wrapped_rows;
        }

        // Click is past the last line — put cursor at the end
        self.cursor_line = self.lines.len() - 1;
        self.cursor_col = self.lines[self.cursor_line].len();
    }
}