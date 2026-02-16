use super::input_cursor;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct InputBuffer {
    text: String,
    cursor_byte_offset: usize,
}

impl InputBuffer {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    #[cfg(test)]
    pub(super) const fn cursor_byte_offset(&self) -> usize {
        self.cursor_byte_offset
    }

    fn set_cursor_byte_offset(&mut self, cursor_byte_offset: usize) -> bool {
        if cursor_byte_offset == self.cursor_byte_offset
            || cursor_byte_offset > self.text.len()
            || !self.text.is_char_boundary(cursor_byte_offset)
        {
            return false;
        }
        self.cursor_byte_offset = cursor_byte_offset;
        true
    }

    fn set_cursor_target(&mut self, target: usize) -> bool {
        self.set_cursor_byte_offset(target)
    }

    fn set_cursor_target_if_some(&mut self, target: Option<usize>) -> bool {
        target.is_some_and(|byte| self.set_cursor_byte_offset(byte))
    }

    fn delete_range(&mut self, start: usize, end: usize) -> bool {
        if start == end {
            return false;
        }
        self.text.replace_range(start..end, "");
        true
    }

    fn delete_range_and_set_cursor(
        &mut self,
        start: usize,
        end: usize,
        cursor_byte_offset: usize,
    ) -> bool {
        if !self.delete_range(start, end) {
            return false;
        }
        self.cursor_byte_offset = cursor_byte_offset;
        true
    }

    pub(super) fn move_up(&mut self, width: u16) -> bool {
        self.move_vertical(width, input_cursor::VerticalDirection::Up)
    }

    pub(super) fn move_down(&mut self, width: u16) -> bool {
        self.move_vertical(width, input_cursor::VerticalDirection::Down)
    }

    fn move_vertical(&mut self, width: u16, direction: input_cursor::VerticalDirection) -> bool {
        let target = input_cursor::target_cursor_byte_offset_for_vertical_move(
            &self.text,
            self.cursor_byte_offset,
            width,
            direction,
        );
        self.set_cursor_target_if_some(target)
    }

    pub(super) fn cursor_text(&self) -> &str {
        &self.text[..self.cursor_byte_offset]
    }

    pub(super) fn clear(&mut self) {
        self.text.clear();
        self.cursor_byte_offset = 0;
    }

    pub(super) fn insert_char(&mut self, ch: char) {
        self.text.insert(self.cursor_byte_offset, ch);
        self.cursor_byte_offset += ch.len_utf8();
    }

    pub(super) fn paste(&mut self, pasted: &str) {
        if pasted.is_empty() {
            return;
        }
        self.text.insert_str(self.cursor_byte_offset, pasted);
        self.cursor_byte_offset += pasted.len();
    }

    pub(super) fn backspace(&mut self) -> bool {
        let Some(prev) = prev_char_boundary(&self.text, self.cursor_byte_offset) else {
            return false;
        };
        self.delete_range_and_set_cursor(prev, self.cursor_byte_offset, prev)
    }

    pub(super) fn delete_forward(&mut self) -> bool {
        let Some(next) = next_char_boundary(&self.text, self.cursor_byte_offset) else {
            return false;
        };
        self.delete_range(self.cursor_byte_offset, next)
    }

    pub(super) fn delete_to_line_start(&mut self) -> bool {
        let target = line_start_boundary(&self.text, self.cursor_byte_offset);
        self.delete_range_and_set_cursor(target, self.cursor_byte_offset, target)
    }

    pub(super) fn delete_to_line_end(&mut self) -> bool {
        let target = line_end_boundary(&self.text, self.cursor_byte_offset);
        self.delete_range(self.cursor_byte_offset, target)
    }

    pub(super) fn move_left(&mut self) -> bool {
        self.set_cursor_target_if_some(prev_char_boundary(&self.text, self.cursor_byte_offset))
    }

    pub(super) fn move_right(&mut self) -> bool {
        self.set_cursor_target_if_some(next_char_boundary(&self.text, self.cursor_byte_offset))
    }

    pub(super) fn move_word_left(&mut self) -> bool {
        let target = prev_word_boundary(&self.text, self.cursor_byte_offset);
        self.set_cursor_target(target)
    }

    pub(super) fn move_word_right(&mut self) -> bool {
        let target = next_word_boundary(&self.text, self.cursor_byte_offset);
        self.set_cursor_target(target)
    }

    pub(super) fn move_line_start(&mut self) -> bool {
        let target = line_start_boundary(&self.text, self.cursor_byte_offset);
        self.set_cursor_target(target)
    }

    pub(super) fn move_line_end(&mut self) -> bool {
        let target = line_end_boundary(&self.text, self.cursor_byte_offset);
        self.set_cursor_target(target)
    }

    pub(super) fn delete_word_left(&mut self) -> bool {
        let target = prev_word_boundary(&self.text, self.cursor_byte_offset);
        self.delete_range_and_set_cursor(target, self.cursor_byte_offset, target)
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

fn line_start_boundary(value: &str, cursor: usize) -> usize {
    let cursor = cursor.min(value.len());
    value[..cursor].rfind('\n').map_or(0, |idx| idx + 1)
}

fn line_end_boundary(value: &str, cursor: usize) -> usize {
    let cursor = cursor.min(value.len());
    value[cursor..]
        .find('\n')
        .map_or(value.len(), |offset| cursor + offset)
}

fn prev_word_boundary(value: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(value.len());
    if cursor == 0 {
        return 0;
    }

    while let Some((idx, ch)) = prev_char(value, cursor) {
        if !ch.is_whitespace() {
            break;
        }
        cursor = idx;
    }

    if let Some((idx, ch)) = prev_char(value, cursor) {
        let class = char_class(ch);
        while let Some((prev_idx, prev_ch)) = prev_char(value, cursor) {
            if char_class(prev_ch) != class {
                break;
            }
            cursor = prev_idx;
        }
        return idx.min(cursor);
    }

    cursor
}

fn next_word_boundary(value: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(value.len());
    if cursor == value.len() {
        return cursor;
    }

    while let Some((_, ch)) = next_char(value, cursor) {
        if !ch.is_whitespace() {
            break;
        }
        cursor = next_char_boundary(value, cursor).unwrap_or(value.len());
        if cursor >= value.len() {
            return value.len();
        }
    }

    if let Some((_, ch)) = next_char(value, cursor) {
        let class = char_class(ch);
        while let Some((_, next_ch)) = next_char(value, cursor) {
            if char_class(next_ch) != class {
                break;
            }
            cursor = next_char_boundary(value, cursor).unwrap_or(value.len());
            if cursor >= value.len() {
                break;
            }
        }
    }

    cursor
}

fn prev_char(value: &str, at: usize) -> Option<(usize, char)> {
    if at == 0 {
        return None;
    }

    value[..at].char_indices().last()
}

fn next_char(value: &str, at: usize) -> Option<(usize, char)> {
    if at >= value.len() {
        return None;
    }

    value[at..]
        .char_indices()
        .next()
        .map(|(idx, ch)| (at + idx, ch))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Whitespace,
    Word,
    Symbol,
}

fn char_class(ch: char) -> CharClass {
    if ch.is_whitespace() {
        return CharClass::Whitespace;
    }
    if ch.is_alphanumeric() || ch == '_' {
        return CharClass::Word;
    }
    CharClass::Symbol
}

#[cfg(test)]
mod tests {
    use super::InputBuffer;

    #[test]
    fn new_state_is_empty_and_cursor_is_zero() {
        let input = InputBuffer::new();
        assert_eq!(input.text(), "");
        assert_eq!(input.cursor_byte_offset(), 0);
        assert_eq!(input.cursor_text(), "");
    }

    #[test]
    fn insert_char_appends_and_moves_cursor() {
        let mut input = InputBuffer::new();
        input.insert_char('h');
        input.insert_char('i');

        assert_eq!(input.text(), "hi");
        assert_eq!(input.cursor_byte_offset(), 2);
        assert_eq!(input.cursor_text(), "hi");
    }

    #[test]
    fn move_left_and_right_clamp_at_bounds() {
        let mut input = InputBuffer::new();
        input.paste("ab");

        assert!(input.move_left());
        assert!(input.move_left());
        assert!(!input.move_left());
        assert_eq!(input.cursor_byte_offset(), 0);

        assert!(input.move_right());
        assert!(input.move_right());
        assert!(!input.move_right());
        assert_eq!(input.cursor_byte_offset(), 2);
    }

    #[test]
    fn insert_char_respects_cursor_position() {
        let mut input = InputBuffer::new();
        input.paste("ac");
        input.move_left();
        input.insert_char('b');

        assert_eq!(input.text(), "abc");
        assert_eq!(input.cursor_text(), "ab");
    }

    #[test]
    fn backspace_removes_character_before_cursor() {
        let mut input = InputBuffer::new();
        input.paste("abc");
        input.move_left();

        assert!(input.backspace());
        assert_eq!(input.text(), "ac");
        assert_eq!(input.cursor_text(), "a");
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut input = InputBuffer::new();
        input.paste("abc");
        input.move_left();
        input.move_left();
        input.move_left();

        assert!(!input.backspace());
        assert_eq!(input.text(), "abc");
        assert_eq!(input.cursor_byte_offset(), 0);
    }

    #[test]
    fn paste_inserts_at_cursor() {
        let mut input = InputBuffer::new();
        input.paste("hello");
        input.move_left();
        input.move_left();
        input.paste("y ");

        assert_eq!(input.text(), "hely lo");
        assert_eq!(input.cursor_text(), "hely ");
    }

    #[test]
    fn unicode_navigation_and_backspace_follow_char_boundaries() {
        let mut input = InputBuffer::new();
        input.paste("🙂x");

        assert_eq!(input.cursor_byte_offset(), "🙂x".len());
        assert!(input.move_left());
        assert_eq!(input.cursor_text(), "🙂");
        assert!(input.backspace());
        assert_eq!(input.text(), "x");
        assert_eq!(input.cursor_byte_offset(), 0);
        assert_eq!(input.cursor_text(), "");
    }

    #[test]
    fn clear_resets_text_and_cursor() {
        let mut input = InputBuffer::new();
        input.paste("hello");
        input.move_left();
        input.clear();

        assert_eq!(input.text(), "");
        assert_eq!(input.cursor_byte_offset(), 0);
        assert_eq!(input.cursor_text(), "");
    }

    #[test]
    fn move_line_start_and_end_stay_within_current_line() {
        let mut input = InputBuffer::new();
        input.paste("ab\ncd\nef");

        assert!(input.move_line_start());
        assert_eq!(input.cursor_text(), "ab\ncd\n");
        assert!(!input.move_line_start());

        assert!(input.move_line_end());
        assert_eq!(input.cursor_text(), "ab\ncd\nef");
        assert!(!input.move_line_end());
    }

    #[test]
    fn move_up_and_down_follow_wrapped_rows_without_explicit_newlines() {
        let mut input = InputBuffer::new();
        input.paste("abcdefghij");
        let end = input.cursor_byte_offset();

        assert!(input.move_up(5));
        let middle = input.cursor_byte_offset();
        assert!(middle < end);

        assert!(input.move_down(5));
        assert_eq!(input.cursor_byte_offset(), end);
    }

    #[test]
    fn move_up_and_down_are_noops_when_single_visual_row() {
        let mut input = InputBuffer::new();
        input.paste("hello");

        assert!(!input.move_up(40));
        assert!(!input.move_down(40));
        assert_eq!(input.cursor_text(), "hello");
    }

    #[test]
    fn delete_forward_removes_character_at_cursor() {
        let mut input = InputBuffer::new();
        input.paste("abdc");
        input.move_left();
        input.move_left();

        assert!(input.delete_forward());
        assert_eq!(input.text(), "abc");
        assert_eq!(input.cursor_text(), "ab");
    }

    #[test]
    fn delete_forward_at_end_is_noop() {
        let mut input = InputBuffer::new();
        input.paste("abc");

        assert!(!input.delete_forward());
        assert_eq!(input.text(), "abc");
        assert_eq!(input.cursor_text(), "abc");
    }

    #[test]
    fn delete_to_line_start_and_end_only_affect_current_line() {
        let mut input = InputBuffer::new();
        input.paste("ab\ncd\nef");
        input.move_left();

        assert!(input.delete_to_line_start());
        assert_eq!(input.text(), "ab\ncd\nf");
        assert_eq!(input.cursor_text(), "ab\ncd\n");
        assert!(!input.delete_to_line_start());

        assert!(input.delete_to_line_end());
        assert_eq!(input.text(), "ab\ncd\n");
        assert_eq!(input.cursor_text(), "ab\ncd\n");
        assert!(!input.delete_to_line_end());
    }

    #[test]
    fn move_word_left_and_right_jump_by_word_boundaries() {
        let mut input = InputBuffer::new();
        input.paste("hello   world, test");
        assert_eq!(input.cursor_text(), "hello   world, test");

        assert!(input.move_word_left());
        assert_eq!(input.cursor_text(), "hello   world, ");
        assert!(input.move_word_left());
        assert_eq!(input.cursor_text(), "hello   world");
        assert!(input.move_word_left());
        assert_eq!(input.cursor_text(), "hello   ");
        assert!(input.move_word_left());
        assert_eq!(input.cursor_text(), "");
        assert!(!input.move_word_left());

        assert!(input.move_word_right());
        assert_eq!(input.cursor_text(), "hello");
        assert!(input.move_word_right());
        assert_eq!(input.cursor_text(), "hello   world");
        assert!(input.move_word_right());
        assert_eq!(input.cursor_text(), "hello   world,");
        assert!(input.move_word_right());
        assert_eq!(input.cursor_text(), "hello   world, test");
        assert!(!input.move_word_right());
    }

    #[test]
    fn delete_word_left_removes_previous_word_chunk() {
        let mut input = InputBuffer::new();
        input.paste("hello   world test");
        assert!(input.delete_word_left());
        assert_eq!(input.text(), "hello   world ");
        assert_eq!(input.cursor_text(), "hello   world ");
        assert!(input.delete_word_left());
        assert_eq!(input.text(), "hello   ");
        assert_eq!(input.cursor_text(), "hello   ");
        assert!(input.delete_word_left());
        assert_eq!(input.text(), "");
        assert_eq!(input.cursor_text(), "");
        assert!(!input.delete_word_left());
    }
}
