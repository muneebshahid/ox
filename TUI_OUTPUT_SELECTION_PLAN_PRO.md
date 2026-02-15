You can’t get terminal-native drag selection and receive wheel mouse events in a fullscreen TUI at the same time: as soon as you enable mouse reporting (EnableMouseCapture), the terminal stops doing its own selection by design. That’s exactly why your “wheel scroll works” vs “copy works” toggle keeps happening. (src/tui/terminal.rs)

So the only way to achieve your exact UX combo in the current architecture is:

Keep EnableMouseCapture ON so you always get wheel events → output scroll stays app-owned. (src/tui/terminal.rs, src/tui/ui_action/adapter.rs, src/tui/state/mod.rs)

Implement drag selection + copy inside ox (highlight the selected region and copy it to the clipboard yourself), so “normal mouse drag selection/copy works” without any Shift bypass.

Below is a concrete, minimal patch plan that fits your codebase.

1) Teach the input adapter to emit drag-selection actions

Add three new UI actions for left-click selection. (src/tui/ui_action/mod.rs)

// src/tui/ui_action/mod.rs
pub enum UiAction {
    // ... existing ...
    ScrollUp { lines: u16 },
    ScrollDown { lines: u16 },

    // NEW: output selection (absolute terminal coords)
    OutputSelectStart { col: u16, row: u16 },
    OutputSelectDrag  { col: u16, row: u16 },
    OutputSelectEnd   { col: u16, row: u16 },

    ViewportChanged,
    Quit,
    Ignore,
}

Now map mouse events into those actions. (src/tui/ui_action/adapter.rs)

// src/tui/ui_action/adapter.rs
use crossterm::event::{
    Event as CEvent, KeyCode, KeyEvent, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};

const fn to_ui_action_from_mouse(mouse: MouseEvent) -> UiAction {
    match mouse.kind {
        MouseEventKind::ScrollUp => UiAction::ScrollUp { lines: MOUSE_SCROLL_LINES },
        MouseEventKind::ScrollDown => UiAction::ScrollDown { lines: MOUSE_SCROLL_LINES },

        MouseEventKind::Down(MouseButton::Left) => UiAction::OutputSelectStart {
            col: mouse.column,
            row: mouse.row,
        },
        MouseEventKind::Drag(MouseButton::Left) => UiAction::OutputSelectDrag {
            col: mouse.column,
            row: mouse.row,
        },
        MouseEventKind::Up(MouseButton::Left) => UiAction::OutputSelectEnd {
            col: mouse.column,
            row: mouse.row,
        },

        _ => UiAction::Ignore,
    }
}

This keeps wheel scrolling intact (what you already have), and adds drag tracking. (src/tui/ui_action/adapter.rs)

1) Track the output viewport + selection state in TuiState

You need output-viewport geometry so you can interpret mouse coordinates as “inside output area.”
You already do something similar for input width (set_input_inner_width) in render::draw. (src/tui/render/mod.rs, src/tui/state/mod.rs)

Add these structs/fields to TuiState. (src/tui/state/mod.rs)

// src/tui/state/mod.rs
# [derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Viewport {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

# [derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CellPos {
    col: u16, // relative to output viewport
    row: u16, // relative to output viewport
}

# [derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutputSelection {
    anchor: CellPos,
    focus: CellPos,
    selecting: bool,
    pending_copy: bool,
}

pub struct TuiState {
    // ... existing fields ...
    output_scroll_lines_from_bottom: u16,

    // NEW:
    output_viewport: Option<Viewport>,
    output_cells: Vec<Vec<String>>, // snapshot of rendered output area (cells[y][x] -> symbol)
    output_selection: Option<OutputSelection>,
}

Initialize them in TuiState::new(). (src/tui/state/mod.rs)

output_viewport: None,
output_cells: Vec::new(),
output_selection: None,

Add setters (called from rendering):

impl TuiState {
    pub(in crate::tui) fn set_output_viewport(&mut self, vp: Viewport) {
        self.output_viewport = Some(vp);
    }

    pub(in crate::tui) fn set_output_cells(&mut self, cells: Vec<Vec<String>>) {
        self.output_cells = cells;
    }

    pub(in crate::tui) fn output_selection_range(&self) -> Option<(CellPos, CellPos)> {
        let sel = self.output_selection?;
        let a = sel.anchor;
        let b = sel.focus;
        let ordered = if (b.row, b.col) < (a.row, a.col) { (b, a) } else { (a, b) };
        Some(ordered)
    }

    pub(in crate::tui) fn take_pending_copy_text(&mut self) -> Option<String> {
        let sel = self.output_selection?;
        if !sel.pending_copy {
            return None;
        }
        if sel.anchor == sel.focus {
            // click without drag -> no copy
            self.output_selection = Some(OutputSelection { pending_copy: false, ..sel });
            return None;
        }

        let (start, end) = self.output_selection_range()?;
        let text = build_selected_text(&self.output_cells, start, end)?;

        // clear pending flag, keep selection highlight
        self.output_selection = Some(OutputSelection { pending_copy: false, ..sel });
        Some(text)
    }
}

Selection handling in handle_ui_action (add these match arms). (src/tui/state/mod.rs)

match action {
    // ... existing ...

    UiAction::OutputSelectStart { col, row } => {
        self.begin_output_selection(col, row);
        StateCommand::None
    }
    UiAction::OutputSelectDrag { col, row } => {
        self.update_output_selection(col, row);
        StateCommand::None
    }
    UiAction::OutputSelectEnd { col, row } => {
        self.end_output_selection(col, row);
        StateCommand::None
    }

    // ...
}

And implement the helpers:

impl TuiState {
    fn begin_output_selection(&mut self, col: u16, row: u16) {
        let Some(vp) = self.output_viewport else { return; };
        if col < vp.x || row < vp.y || col >= vp.x + vp.width || row >= vp.y + vp.height {
            // click outside output clears selection
            self.output_selection = None;
            self.mark_dirty();
            return;
        }
        let rel = CellPos { col: col - vp.x, row: row - vp.y };
        self.output_selection = Some(OutputSelection {
            anchor: rel,
            focus: rel,
            selecting: true,
            pending_copy: false,
        });
        self.mark_dirty();
    }

    fn update_output_selection(&mut self, col: u16, row: u16) {
        let Some(vp) = self.output_viewport else { return; };
        let Some(sel) = self.output_selection else { return; };
        if !sel.selecting { return; }

        // clamp drag into viewport
        let rel_col = if col < vp.x { 0 }
            else if col >= vp.x + vp.width { vp.width.saturating_sub(1) }
            else { col - vp.x };
        let rel_row = if row < vp.y { 0 }
            else if row >= vp.y + vp.height { vp.height.saturating_sub(1) }
            else { row - vp.y };

        self.output_selection = Some(OutputSelection {
            focus: CellPos { col: rel_col, row: rel_row },
            ..sel
        });
        self.mark_dirty();
    }

    fn end_output_selection(&mut self, col: u16, row: u16) {
        self.update_output_selection(col, row);
        let Some(sel) = self.output_selection else { return; };
        if !sel.selecting { return; }

        let pending_copy = sel.anchor != sel.focus;
        self.output_selection = Some(OutputSelection {
            selecting: false,
            pending_copy,
            ..sel
        });
        self.mark_dirty();
    }
}

Finally, implement build_selected_text() using the rendered cell snapshot (so you copy exactly what was on screen — closest to “native”). (src/tui/state/mod.rs)

fn build_selected_text(cells: &[Vec<String>], start: CellPos, end: CellPos) -> Option<String> {
    if cells.is_empty() { return None; }
    let h = cells.len() as u16;
    let w = cells[0].len() as u16;
    if h == 0 || w == 0 { return None; }

    let start_row = start.row.min(h - 1);
    let end_row = end.row.min(h - 1);
    let start_col = start.col.min(w - 1);
    let end_col = end.col.min(w - 1);

    let (sr, sc, er, ec) = if (end_row, end_col) < (start_row, start_col) {
        (end_row, end_col, start_row, start_col)
    } else {
        (start_row, start_col, end_row, end_col)
    };

    let mut out_lines = Vec::new();
    for row in sr..=er {
        let c0 = if row == sr { sc } else { 0 };
        let c1 = if row == er { ec } else { w - 1 };

        let mut line = String::new();
        for col in c0..=c1 {
            line.push_str(&cells[row as usize][col as usize]);
        }
        out_lines.push(line.trim_end().to_string());
    }

    let text = out_lines.join("\n").trim_end().to_string();
    (!text.is_empty()).then_some(text)
}

1) Highlight the selection in the output buffer at render time

Apply a reverse/invert style to selected cells after drawing the output paragraph. (src/tui/render/output.rs)

Inside draw_output(...) you already have frame, state, and area. Add:

Store viewport to state

Render paragraph (existing)

Apply selection highlighting

Capture cell snapshot into state for copying

Example sketch:

// src/tui/render/output.rs
use ratatui::style::Modifier;
// ...

pub(super) fn draw_output(
    frame: &mut Frame<'_>,
    state: &mut TuiState,
    output: &OutputView,
    area: Rect,
) {
    // keep state updated with current viewport geometry
    state.set_output_viewport(crate::tui::state::Viewport {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height,
    });

    // existing scroll math + render
    let max_scroll = viewport::max_scroll_offset(&output.plain_lines, area.width, area.height);
    state.clamp_output_scroll_lines_from_bottom(max_scroll);
    let scroll = viewport::scroll_offset(
        &output.plain_lines,
        area.width,
        area.height,
        state.output_scroll_lines_from_bottom(),
    );
    let output = Paragraph::new(output.text.clone())
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(output, area);

    // NEW: highlight selection
    if let Some((start, end)) = state.output_selection_range() {
        let buf = frame.buffer_mut();
        let w = area.width;
        let h = area.height;
        if w > 0 && h > 0 {
            let start_row = start.row.min(h - 1);
            let end_row = end.row.min(h - 1);
            let start_col = start.col.min(w - 1);
            let end_col = end.col.min(w - 1);
            let (sr, sc, er, ec) = if (end_row, end_col) < (start_row, start_col) {
                (end_row, end_col, start_row, start_col)
            } else {
                (start_row, start_col, end_row, end_col)
            };

            for r in sr..=er {
                let c0 = if r == sr { sc } else { 0 };
                let c1 = if r == er { ec } else { w - 1 };
                for c in c0..=c1 {
                    let x = area.x + c;
                    let y = area.y + r;
                    let cell = &mut buf[(x, y)];
                    // simplest: add reverse modifier (don’t fight existing colors)
                    cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
                }
            }
        }
    }

    // NEW: capture visible output cells for copy (what-you-see-is-what-you-copy)
    let mut cells: Vec<Vec<String>> = Vec::with_capacity(area.height as usize);
    let buf = frame.buffer_mut();
    for dy in 0..area.height {
        let mut row: Vec<String> = Vec::with_capacity(area.width as usize);
        for dx in 0..area.width {
            row.push(buf[(area.x + dx, area.y + dy)].symbol().to_string());
        }
        cells.push(row);
    }
    state.set_output_cells(cells);
}

That’s the core: selection highlight + snapshot for copy. (src/tui/render/output.rs, src/tui/state/mod.rs)

1) Copy to clipboard on mouse-up via OSC 52

Add a tiny OSC52 clipboard helper, then call it after a draw when pending_copy is set.

Create a new module src/tui/clipboard.rs and include it from src/tui/mod.rs. (src/tui/mod.rs)

// src/tui/mod.rs
mod clipboard; // NEW

Clipboard module (no extra deps; includes its own base64). (src/tui/clipboard.rs)

use std::io::{self, Write};

pub fn copy_osc52(text: &str) -> io::Result<()> {
    let b64 = base64_encode(text.as_bytes());
    // OSC 52 ; c ; <base64> BEL
    let osc = format!("\x1b]52;c;{}\x07", b64);
    let mut stdout = io::stdout();
    stdout.write_all(osc.as_bytes())?;
    stdout.flush()
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(((data.len() + 2) / 3) * 4);

    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };

        let n = (b0 << 16) | (b1 << 8) | b2;

        out.push(T[((n >> 18) & 0x3F) as usize] as char);
        out.push(T[((n >> 12) & 0x3F) as usize] as char);

        if i + 1 < data.len() {
            out.push(T[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        if i + 2 < data.len() {
            out.push(T[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        i += 3;
    }

    out
}

Now trigger it after draw in the renderer so you copy from the latest snapshot. The clean place is UiRenderer::draw_if_needed after it calls self.terminal.draw(...). (src/tui/terminal.rs)

// src/tui/terminal.rs
use super::clipboard;
// ...

pub(super) fn draw_if_needed(&mut self, state: &mut TuiState) -> Result<()> {
    // ... existing draw decision ...

    if should_draw {
        self.terminal.draw(state, &self.render_meta)?;

        if let Some(text) = state.take_pending_copy_text() {
            let _ = clipboard::copy_osc52(&text);
            // optional: set a status message here if you want user feedback
        }
    }

    // ...
    Ok(())
}

Now mouse-drag selection “copies” without relying on terminal selection, and wheel scroll stays app-owned. (src/tui/terminal.rs, src/tui/state/mod.rs, src/tui/render/output.rs)

1) Let selection work while the agent is running

Right now, during an active turn you only allow scroll + viewport actions. Selection actions will be ignored unless you add them to the allowlist. (src/tui/orchestrator.rs)

Update this check:

// src/tui/orchestrator.rs
if matches!(
    &ui_action,
    UiAction::ScrollUp { .. }
        | UiAction::ScrollDown { .. }
        | UiAction::ViewportChanged
        | UiAction::OutputSelectStart { .. }
        | UiAction::OutputSelectDrag { .. }
        | UiAction::OutputSelectEnd { .. }
) {
    let_ = state.handle_ui_action(ui_action);
}

(And update the test that asserts only scroll/viewport are allowed during active turn.) (src/tui/orchestrator.rs)

Resulting behavior (the combo you wanted)

With the above:

Mouse wheel still generates UiAction::ScrollUp/Down → only output scroll changes. (src/tui/ui_action/adapter.rs, src/tui/state/mod.rs)

Up/Down keys remain mapped to MoveCursorUp/Down → only input cursor moves. (src/tui/ui_action/adapter.rs, src/tui/state/input.rs)

Mouse drag selection/copy works again without Shift: you highlight in-app and ox copies via OSC52. (src/tui/render/output.rs, src/tui/state/mod.rs, src/tui/terminal.rs, src/tui/clipboard.rs)
