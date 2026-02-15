#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct InputState {
    text: String,
    cursor_byte: usize,
}

impl InputState {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    #[cfg(test)]
    pub(super) const fn cursor_byte(&self) -> usize {
        self.cursor_byte
    }

    pub(super) fn cursor_text(&self) -> &str {
        &self.text[..self.cursor_byte]
    }

    pub(super) fn clear(&mut self) {
        self.text.clear();
        self.cursor_byte = 0;
    }

    pub(super) fn insert_char(&mut self, ch: char) {
        self.text.insert(self.cursor_byte, ch);
        self.cursor_byte += ch.len_utf8();
    }

    pub(super) fn paste(&mut self, pasted: &str) {
        if pasted.is_empty() {
            return;
        }
        self.text.insert_str(self.cursor_byte, pasted);
        self.cursor_byte += pasted.len();
    }

    pub(super) fn backspace(&mut self) -> bool {
        let Some(prev) = prev_char_boundary(&self.text, self.cursor_byte) else {
            return false;
        };
        self.text.replace_range(prev..self.cursor_byte, "");
        self.cursor_byte = prev;
        true
    }

    pub(super) fn delete_forward(&mut self) -> bool {
        let Some(next) = next_char_boundary(&self.text, self.cursor_byte) else {
            return false;
        };
        self.text.replace_range(self.cursor_byte..next, "");
        true
    }

    pub(super) fn delete_to_start(&mut self) -> bool {
        if self.cursor_byte == 0 {
            return false;
        }
        self.text.replace_range(0..self.cursor_byte, "");
        self.cursor_byte = 0;
        true
    }

    pub(super) fn delete_to_end(&mut self) -> bool {
        if self.cursor_byte == self.text.len() {
            return false;
        }
        self.text.truncate(self.cursor_byte);
        true
    }

    pub(super) fn move_left(&mut self) -> bool {
        let Some(prev) = prev_char_boundary(&self.text, self.cursor_byte) else {
            return false;
        };
        self.cursor_byte = prev;
        true
    }

    pub(super) fn move_right(&mut self) -> bool {
        let Some(next) = next_char_boundary(&self.text, self.cursor_byte) else {
            return false;
        };
        self.cursor_byte = next;
        true
    }

    pub(super) const fn move_home(&mut self) -> bool {
        if self.cursor_byte == 0 {
            return false;
        }
        self.cursor_byte = 0;
        true
    }

    pub(super) const fn move_end(&mut self) -> bool {
        if self.cursor_byte == self.text.len() {
            return false;
        }
        self.cursor_byte = self.text.len();
        true
    }
}

fn prev_char_boundary(value: &str, at: usize) -> Option<usize> {
    if at == 0 {
        return None;
    }

    value[..at].char_indices().last().map(|(idx, _)| idx)
}

fn next_char_boundary(value: &str, at: usize) -> Option<usize> {
    if at >= value.len() {
        return None;
    }

    value[at..]
        .chars()
        .next()
        .map(|ch| at.saturating_add(ch.len_utf8()))
}

#[cfg(test)]
mod tests {
    use super::InputState;

    #[test]
    fn new_state_is_empty_and_cursor_is_zero() {
        let input = InputState::new();
        assert_eq!(input.text(), "");
        assert_eq!(input.cursor_byte(), 0);
        assert_eq!(input.cursor_text(), "");
    }

    #[test]
    fn insert_char_appends_and_moves_cursor() {
        let mut input = InputState::new();
        input.insert_char('h');
        input.insert_char('i');

        assert_eq!(input.text(), "hi");
        assert_eq!(input.cursor_byte(), 2);
        assert_eq!(input.cursor_text(), "hi");
    }

    #[test]
    fn move_left_and_right_clamp_at_bounds() {
        let mut input = InputState::new();
        input.paste("ab");

        assert!(input.move_left());
        assert!(input.move_left());
        assert!(!input.move_left());
        assert_eq!(input.cursor_byte(), 0);

        assert!(input.move_right());
        assert!(input.move_right());
        assert!(!input.move_right());
        assert_eq!(input.cursor_byte(), 2);
    }

    #[test]
    fn insert_char_respects_cursor_position() {
        let mut input = InputState::new();
        input.paste("ac");
        input.move_left();
        input.insert_char('b');

        assert_eq!(input.text(), "abc");
        assert_eq!(input.cursor_text(), "ab");
    }

    #[test]
    fn backspace_removes_character_before_cursor() {
        let mut input = InputState::new();
        input.paste("abc");
        input.move_left();

        assert!(input.backspace());
        assert_eq!(input.text(), "ac");
        assert_eq!(input.cursor_text(), "a");
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut input = InputState::new();
        input.paste("abc");
        input.move_left();
        input.move_left();
        input.move_left();

        assert!(!input.backspace());
        assert_eq!(input.text(), "abc");
        assert_eq!(input.cursor_byte(), 0);
    }

    #[test]
    fn paste_inserts_at_cursor() {
        let mut input = InputState::new();
        input.paste("hello");
        input.move_left();
        input.move_left();
        input.paste("y ");

        assert_eq!(input.text(), "hely lo");
        assert_eq!(input.cursor_text(), "hely ");
    }

    #[test]
    fn unicode_navigation_and_backspace_follow_char_boundaries() {
        let mut input = InputState::new();
        input.paste("🙂x");

        assert_eq!(input.cursor_byte(), "🙂x".len());
        assert!(input.move_left());
        assert_eq!(input.cursor_text(), "🙂");
        assert!(input.backspace());
        assert_eq!(input.text(), "x");
        assert_eq!(input.cursor_byte(), 0);
        assert_eq!(input.cursor_text(), "");
    }

    #[test]
    fn clear_resets_text_and_cursor() {
        let mut input = InputState::new();
        input.paste("hello");
        input.move_left();
        input.clear();

        assert_eq!(input.text(), "");
        assert_eq!(input.cursor_byte(), 0);
        assert_eq!(input.cursor_text(), "");
    }

    #[test]
    fn move_home_and_end_clamp_to_boundaries() {
        let mut input = InputState::new();
        input.paste("hello");
        input.move_left();
        input.move_left();

        assert!(input.move_home());
        assert_eq!(input.cursor_text(), "");
        assert!(!input.move_home());

        assert!(input.move_end());
        assert_eq!(input.cursor_text(), "hello");
        assert!(!input.move_end());
    }

    #[test]
    fn delete_forward_removes_character_at_cursor() {
        let mut input = InputState::new();
        input.paste("abdc");
        input.move_left();
        input.move_left();

        assert!(input.delete_forward());
        assert_eq!(input.text(), "abc");
        assert_eq!(input.cursor_text(), "ab");
    }

    #[test]
    fn delete_forward_at_end_is_noop() {
        let mut input = InputState::new();
        input.paste("abc");

        assert!(!input.delete_forward());
        assert_eq!(input.text(), "abc");
        assert_eq!(input.cursor_text(), "abc");
    }

    #[test]
    fn delete_to_start_removes_all_text_before_cursor() {
        let mut input = InputState::new();
        input.paste("hello");
        input.move_left();
        input.move_left();

        assert!(input.delete_to_start());
        assert_eq!(input.text(), "lo");
        assert_eq!(input.cursor_text(), "");
        assert!(!input.delete_to_start());
    }

    #[test]
    fn delete_to_end_removes_all_text_after_cursor() {
        let mut input = InputState::new();
        input.paste("hello");
        input.move_left();
        input.move_left();

        assert!(input.delete_to_end());
        assert_eq!(input.text(), "hel");
        assert_eq!(input.cursor_text(), "hel");
        assert!(!input.delete_to_end());
    }
}
