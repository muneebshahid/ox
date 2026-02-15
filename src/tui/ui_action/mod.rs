pub(super) mod adapter;

#[derive(Debug, PartialEq, Eq)]
pub enum UiAction {
    Insert(char),
    Backspace,
    Delete,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorWordLeft,
    MoveCursorWordRight,
    MoveCursorLineStart,
    MoveCursorLineEnd,
    MoveCursorHome,
    MoveCursorEnd,
    DeleteToLineStart,
    DeleteToLineEnd,
    DeleteWordLeft,
    Submit,
    Paste(String),
    ScrollUp { lines: u16 },
    ScrollDown { lines: u16 },
    ViewportChanged,
    Quit,
    Ignore,
}
