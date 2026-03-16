//! Multi-line input editor with history

use std::collections::VecDeque;

mod history;
mod input;
mod render;

pub use render::render_editor;

const MAX_HISTORY: usize = 100;

/// Multi-line text editor with cursor and history
#[derive(Debug, Clone)]
pub struct Editor {
    pub(crate) lines: Vec<String>,
    pub(crate) cursor_line: usize,
    pub(crate) cursor_col: usize,
    pub(crate) history: VecDeque<String>,
    pub(crate) history_index: Option<usize>,
    pub(crate) saved_input: Option<Vec<String>>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            history: VecDeque::new(),
            history_index: None,
            saved_input: None,
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn content(&self) -> &[String] {
        &self.lines
    }

    /// Check if the editor is empty (no meaningful content)
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.is_empty())
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.history_index = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_editor_empty() {
        let editor = Editor::new();
        assert_eq!(editor.line_count(), 1);
        assert_eq!(editor.cursor_line, 0);
        assert_eq!(editor.cursor_col, 0);
        assert_eq!(editor.content()[0], "");
    }

    #[test]
    fn test_insert_char_single_line() {
        let mut editor = Editor::new();
        editor.insert_char('h');
        editor.insert_char('i');
        assert_eq!(editor.content()[0], "hi");
        assert_eq!(editor.cursor_col, 2);
    }

    #[test]
    fn test_insert_newline() {
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.insert_char('b');
        editor.insert_char('\n');
        editor.insert_char('c');

        assert_eq!(editor.line_count(), 2);
        assert_eq!(editor.content()[0], "ab");
        assert_eq!(editor.content()[1], "c");
        assert_eq!(editor.cursor_line, 1);
        assert_eq!(editor.cursor_col, 1);
    }

    #[test]
    fn test_delete_back_single_line() {
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.insert_char('b');
        editor.delete_back();

        assert_eq!(editor.content()[0], "a");
        assert_eq!(editor.cursor_col, 1);
    }

    #[test]
    fn test_delete_back_empty() {
        let mut editor = Editor::new();
        editor.delete_back(); // Shouldn't crash
        assert_eq!(editor.cursor_col, 0);
    }

    #[test]
    fn test_delete_back_join_lines() {
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.insert_char('\n');
        editor.insert_char('b');

        // Cursor at start of line 1
        editor.cursor_line = 1;
        editor.cursor_col = 0;
        editor.delete_back();

        assert_eq!(editor.line_count(), 1);
        assert_eq!(editor.content()[0], "ab");
        assert_eq!(editor.cursor_line, 0);
    }

    #[test]
    fn test_delete_forward() {
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.insert_char('b');
        editor.insert_char('c');
        editor.cursor_col = 1; // Between 'a' and 'b'
        editor.delete_forward();

        assert_eq!(editor.content()[0], "ac");
        assert_eq!(editor.cursor_col, 1);
    }

    #[test]
    fn test_delete_forward_join_lines() {
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.insert_char('\n');
        editor.insert_char('b');

        // Move cursor to end of first line
        editor.cursor_line = 0;
        editor.cursor_col = 1;
        editor.delete_forward();

        assert_eq!(editor.line_count(), 1);
        assert_eq!(editor.content()[0], "ab");
    }

    #[test]
    fn test_move_left_right() {
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.insert_char('b');
        editor.insert_char('c');

        assert_eq!(editor.cursor_col, 3);
        editor.move_left();
        assert_eq!(editor.cursor_col, 2);
        editor.move_left();
        assert_eq!(editor.cursor_col, 1);
        editor.move_right();
        assert_eq!(editor.cursor_col, 2);
    }

    #[test]
    fn test_move_left_across_lines() {
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.insert_char('\n');
        editor.insert_char('b');

        // At position 1 of line 1
        assert_eq!(editor.cursor_line, 1);
        assert_eq!(editor.cursor_col, 1);

        editor.move_left();
        editor.move_left(); // Should go to end of previous line

        assert_eq!(editor.cursor_line, 0);
        assert_eq!(editor.cursor_col, 1);
    }

    #[test]
    fn test_move_right_across_lines() {
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.insert_char('\n');
        editor.insert_char('b');

        editor.cursor_line = 0;
        editor.cursor_col = 1; // End of first line
        editor.move_right();

        assert_eq!(editor.cursor_line, 1);
        assert_eq!(editor.cursor_col, 0);
    }

    #[test]
    fn test_move_home_end() {
        let mut editor = Editor::new();
        editor.insert_char('h');
        editor.insert_char('e');
        editor.insert_char('l');
        editor.insert_char('l');
        editor.insert_char('o');

        assert_eq!(editor.cursor_col, 5);

        editor.move_home();
        assert_eq!(editor.cursor_col, 0);

        editor.move_end();
        assert_eq!(editor.cursor_col, 5);
    }

    #[test]
    fn test_clear() {
        let mut editor = Editor::new();
        editor.insert_char('t');
        editor.insert_char('e');
        editor.insert_char('s');
        editor.insert_char('t');
        editor.clear();

        assert_eq!(editor.line_count(), 1);
        assert_eq!(editor.content()[0], "");
        assert_eq!(editor.cursor_line, 0);
        assert_eq!(editor.cursor_col, 0);
    }

    #[test]
    fn test_submit_returns_content() {
        let mut editor = Editor::new();
        editor.insert_char('h');
        editor.insert_char('i');
        editor.insert_char('\n');
        editor.insert_char('t');
        editor.insert_char('h');
        editor.insert_char('e');
        editor.insert_char('r');
        editor.insert_char('e');

        let result = editor.submit();
        assert!(result.is_some());

        // Should be cleared after submit
        assert_eq!(editor.line_count(), 1);
        assert_eq!(editor.content()[0], "");
    }

    #[test]
    fn test_submit_empty_returns_none() {
        let mut editor = Editor::new();
        let result = editor.submit();
        assert!(result.is_none());
    }

    #[test]
    fn test_submit_whitespace_only_returns_none() {
        let mut editor = Editor::new();
        editor.insert_char(' ');
        editor.insert_char('\n');
        editor.insert_char(' ');

        let result = editor.submit();
        assert!(result.is_none());
    }

    #[test]
    fn test_history_up_down() {
        let mut editor = Editor::new();

        // Submit first entry
        editor.insert_char('f');
        editor.insert_char('i');
        editor.insert_char('r');
        editor.insert_char('s');
        editor.insert_char('t');
        editor.submit();

        // Submit second entry
        editor.insert_char('s');
        editor.insert_char('e');
        editor.insert_char('c');
        editor.insert_char('o');
        editor.insert_char('n');
        editor.insert_char('d');
        editor.submit();

        // Navigate history
        editor.history_up();
        assert_eq!(editor.content()[0], "second");

        editor.history_up();
        assert_eq!(editor.content()[0], "first");

        editor.history_down();
        assert_eq!(editor.content()[0], "second");

        editor.history_down();
        assert_eq!(editor.content()[0], ""); // Back to empty
    }

    #[test]
    fn test_history_saves_current_input() {
        let mut editor = Editor::new();
        editor.insert_char('s');
        editor.insert_char('a');
        editor.insert_char('v');
        editor.insert_char('e');
        editor.insert_char('d');
        editor.submit();

        editor.insert_char('c');
        editor.insert_char('u');
        editor.insert_char('r');
        editor.insert_char('r');
        editor.insert_char('e');
        editor.insert_char('n');
        editor.insert_char('t');

        // Go up then down should restore current input
        editor.history_up();
        editor.history_down();

        assert_eq!(editor.content()[0], "current");
    }

    #[test]
    fn test_delete_word_back() {
        let mut editor = Editor::new();
        editor.insert_char('h');
        editor.insert_char('e');
        editor.insert_char('l');
        editor.insert_char('l');
        editor.insert_char('o');
        editor.insert_char(' ');
        editor.insert_char('w');
        editor.insert_char('o');
        editor.insert_char('r');
        editor.insert_char('l');
        editor.insert_char('d');

        editor.delete_word_back();
        assert_eq!(editor.content()[0], "hello ");

        editor.delete_word_back();
        assert_eq!(editor.content()[0], "");
    }

    #[test]
    fn test_history_max_size() {
        let mut editor = Editor::new();

        // Add more than MAX_HISTORY entries
        for _ in 0..150 {
            editor.insert_char('a');
            editor.submit();
        }

        assert!(editor.history.len() <= MAX_HISTORY);
    }

    #[test]
    fn test_visual_line_count_no_wrap() {
        let mut editor = Editor::new();
        editor.insert_char('h');
        editor.insert_char('i');
        // "  hi" with indicator "> " = "> hi" = 4 chars, width 80 = 1 visual line
        assert_eq!(editor.visual_line_count(80, 2), 1);
    }

    #[test]
    fn test_visual_line_count_wraps() {
        let mut editor = Editor::new();
        // Type 20 chars on first line: "> " + 20 chars = 22 chars total
        for _ in 0..20 {
            editor.insert_char('a');
        }
        // Width 10: ceil(22/10) = 3 visual lines
        assert_eq!(editor.visual_line_count(10, 2), 3);
    }

    #[test]
    fn test_visual_line_count_multiline_wraps() {
        let mut editor = Editor::new();
        // Line 0: "> " + 8 chars = 10 chars -> 1 visual line at width 10
        for _ in 0..8 {
            editor.insert_char('a');
        }
        editor.insert_char('\n');
        // Line 1: "  " + 15 chars = 17 chars -> 2 visual lines at width 10
        for _ in 0..15 {
            editor.insert_char('b');
        }
        assert_eq!(editor.visual_line_count(10, 2), 3);
    }

    #[test]
    fn test_visual_cursor_position_no_wrap() {
        let mut editor = Editor::new();
        editor.insert_char('h');
        editor.insert_char('i');
        // Cursor at col 2, with indicator "> " = offset 4 in visual line
        let (cx, cy) = editor.visual_cursor_position(80, 2);
        assert_eq!(cx, 4);
        assert_eq!(cy, 0);
    }

    #[test]
    fn test_visual_cursor_position_wrapped() {
        let mut editor = Editor::new();
        // Type 12 chars: "> " + 12 = 14 chars. At width 10: row 0 has 10, row 1 has 4
        for _ in 0..12 {
            editor.insert_char('x');
        }
        let (cx, cy) = editor.visual_cursor_position(10, 2);
        assert_eq!(cy, 1);
        assert_eq!(cx, 4);
    }

    #[test]
    fn test_visual_cursor_position_second_line() {
        let mut editor = Editor::new();
        editor.insert_char('a');
        editor.insert_char('\n');
        editor.insert_char('b');
        editor.insert_char('c');
        // Line 0: "> a" = 3 chars -> 1 visual line at width 80
        // Line 1: "  bc" cursor at col 2 -> offset = 2 + 2 = 4
        let (cx, cy) = editor.visual_cursor_position(80, 2);
        assert_eq!(cy, 1);
        assert_eq!(cx, 4);
    }

    #[test]
    fn test_click_to_cursor_first_line() {
        let mut editor = Editor::new();
        // "hello" with indicator "> " → "> hello"
        for c in "hello".chars() {
            editor.insert_char(c);
        }
        // Click at column 4 of visual row 0 → past the ">" prefix (len 2), col 2 in text
        editor.click_to_cursor(4, 0, 80, 2);
        assert_eq!(editor.cursor_line, 0);
        assert_eq!(editor.cursor_col, 2); // "he|llo"
    }

    #[test]
    fn test_click_to_cursor_second_line() {
        let mut editor = Editor::new();
        for c in "abc".chars() {
            editor.insert_char(c);
        }
        editor.insert_char('\n');
        for c in "defgh".chars() {
            editor.insert_char(c);
        }
        // Line 0: "> abc" (1 visual line at width 80)
        // Line 1: "  defgh" (prefix "  " = 2)
        // Click visual row 1, col 5 → offset 5 - 2 = 3 → "def|gh"
        editor.click_to_cursor(5, 1, 80, 2);
        assert_eq!(editor.cursor_line, 1);
        assert_eq!(editor.cursor_col, 3);
    }

    #[test]
    fn test_click_to_cursor_past_end() {
        let mut editor = Editor::new();
        for c in "hi".chars() {
            editor.insert_char(c);
        }
        // Click way past the content → cursor should be at the end
        editor.click_to_cursor(50, 5, 80, 2);
        assert_eq!(editor.cursor_line, 0);
        assert_eq!(editor.cursor_col, 2);
    }

    #[test]
    fn test_click_to_cursor_wrapped_line() {
        let mut editor = Editor::new();
        // Type 15 chars: "> " + 15 = 17 chars. Width 10: row 0 = 10 chars, row 1 = 7 chars
        for _ in 0..15 {
            editor.insert_char('a');
        }
        // Click at visual row 1, col 3 → abs offset = 1*10 + 3 = 13, minus prefix 2 = 11
        editor.click_to_cursor(3, 1, 10, 2);
        assert_eq!(editor.cursor_line, 0);
        assert_eq!(editor.cursor_col, 11);
    }
}