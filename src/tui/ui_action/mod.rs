pub(super) mod adapter;

#[derive(Debug, PartialEq, Eq)]
pub enum UiAction {
    Insert(char),
    Backspace,
    Delete,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorWordLeft,
    MoveCursorWordRight,
    MoveCursorLineStart,
    MoveCursorLineEnd,
    DeleteToLineStart,
    DeleteToLineEnd,
    DeleteWordLeft,
    Submit,
    Paste(String),
    ScrollUp { lines: u16 },
    ScrollDown { lines: u16 },
    OutputSelectStart { col: u16, row: u16 },
    OutputSelectDrag { col: u16, row: u16 },
    OutputSelectEnd { col: u16, row: u16 },
    ViewportChanged,
    Quit,
    Ignore,
}
