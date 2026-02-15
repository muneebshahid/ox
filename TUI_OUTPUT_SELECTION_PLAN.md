# TUI Output Selection and Copy Redesign Plan

## Context

Current behavior is unstable across terminals because mouse wheel is translated to plain `Up/Down` key events in some environments. This makes it impossible to reliably distinguish wheel scrolling from keyboard navigation using key timing heuristics alone.

Observed result:
- Output scroll and input cursor movement can both be triggered by wheel.
- Heuristic filtering is fragile and terminal-dependent.

This plan replaces heuristic wheel/key inference with explicit in-app output interaction and selection handling.

## Goals

- Keep `Up/Down` keyboard behavior dedicated to input navigation.
- Support output scrolling with mouse wheel.
- Support selecting output text with mouse drag.
- Auto-copy selected output text on selection completion.
- Keep module boundaries clean and public APIs minimal.

## Non-Goals (V1)

- Character-accurate rectangular text selection.
- Selection across wrapped visual fragments with per-cell precision.
- Multi-pane focus model changes.

V1 selection is line-based (robust and simpler). Char-level selection can be a later enhancement.

## Architecture Proposal

### 1. Data Model (Structs/Enums and Key Fields)

Add a dedicated output interaction model under TUI state.

Proposed types:

- `OutputSelectionState`
  - `Idle`
  - `Dragging { anchor_row: u16, current_row: u16 }`
  - `Selected { start_row: u16, end_row: u16 }`

- `OutputSnapshot`
  - `area: Rect`
  - `scroll_offset: u16`
  - `visible_rows: Vec<VisibleOutputRow>`

- `VisibleOutputRow`
  - `visual_row: u16` (row index inside output area)
  - `source_line_index: usize` (index into `plain_lines`)
  - `text: String`

- `OutputSelection`
  - `start_source_line: usize`
  - `end_source_line: usize`

- `CopyResult`
  - `Copied { chars: usize }`
  - `Empty`
  - `Error { message: String }`

State additions in `TuiState`:
- `output_selection_state: OutputSelectionState`
- `output_snapshot: Option<OutputSnapshot>`

### 2. Responsibility Split

`src/tui/ui_action/mod.rs` and `src/tui/ui_action/adapter.rs`
- Own conversion from crossterm events to semantic UI actions.
- Add explicit output mouse actions:
  - `OutputMouseDown { column, row }`
  - `OutputMouseDrag { column, row }`
  - `OutputMouseUp { column, row }`
  - Keep `ScrollUp/ScrollDown` for wheel.

`src/tui/render/output.rs`
- Build render output as today.
- Build and publish `OutputSnapshot` for current frame.
- Render selection highlight for selected rows.

`src/tui/state/mod.rs`
- Single owner of selection lifecycle and mutation.
- On mouse down/drag/up, resolve visual rows from snapshot and update selection state.
- On selection complete (mouse up), resolve selected text and invoke clipboard copy.
- Keep keyboard cursor behavior isolated from output selection.

`src/tui/clipboard.rs` (new)
- Encapsulate clipboard writing:
  - OSC52 write
  - platform fallback commands
- Return `Result<()>` with useful error string.

`src/tui/orchestrator.rs`
- Pure event loop and dispatch only.
- Remove wheel-arrow inference/dedupe heuristics after selection flow is in place.
- Keep optional debug logging only if still useful.

### 3. External API Surface (Public vs Private)

Public surface should remain minimal:

- Keep `TuiState::handle_ui_action(action) -> StateCommand` as the only top-level mutation entry.

Internal (crate-private) additions:
- `TuiState::set_output_snapshot(snapshot: OutputSnapshot)` (called from render path only).

Private helpers:
- Selection row/line mapping helpers.
- Selection-to-text extraction.
- Clipboard integration helpers.

No new cross-module public types should be exposed unless needed by rendering pipeline internals.

### 4. Event and Control Flow

Frame/update flow:
1. Render computes output lines and wrapping metadata.
2. Render creates `OutputSnapshot` and stores into state.
3. Input event arrives:
   - Wheel -> `ScrollUp/ScrollDown` -> output scroll only.
   - Mouse down/drag/up in output area -> selection lifecycle.
4. On mouse up:
   - Build selected text (line-joined).
   - Copy to clipboard.
   - Set status toast/message in transcript/status line.
   - Clear or keep selection (V1 default: clear after copy).

Keyboard flow:
- `Up/Down` always route to input cursor logic only.
- No special wheel-key inference logic.

## Implementation Plan (Incremental)

### Phase 0: Stabilize Baseline

- Keep current branch state but remove newly added heuristic code once new path is ready.
- Preserve existing tests as regression guard.

### Phase 1: Introduce Selection Domain Skeleton

- Add `src/tui/clipboard.rs` with no-op/test implementation first.
- Add selection structs/enums to state.
- Add new `UiAction` variants for output mouse selection.
- Compile with no behavior change (actions mapped but ignored).

### Phase 2: Wire Mouse Events to Semantic Actions

- Update adapter to emit `OutputMouseDown/Drag/Up` using raw mouse coordinates.
- Keep wheel mapped to `ScrollUp/ScrollDown`.
- Do not change keyboard mappings.

### Phase 3: Build Output Snapshot from Render

- Extend output render pipeline to build `OutputSnapshot` each frame.
- Store snapshot into state through one setter.
- Add tests for snapshot mapping correctness for wrapped lines.

### Phase 4: Implement Selection Lifecycle in State

- Handle down/drag/up actions:
  - Validate event is inside output area.
  - Resolve visual row -> source line.
  - Update `OutputSelectionState`.
- On mouse up:
  - Resolve selected source-line range.
  - Build text and copy via clipboard module.
  - Update status message.

### Phase 5: Render Selection Highlight

- In `draw_output`, apply style override to selected source lines.
- Ensure highlight tracks scroll changes.

### Phase 6: Remove Heuristic Input Filtering

- Remove wheel-arrow inference and synthetic dedupe logic from orchestrator.
- Remove now-obsolete debug paths tied to heuristics.
- Keep optional raw input logs behind env flag if desired.

### Phase 7: Final Hardening

- Add/refresh tests:
  - Mouse wheel only affects output scroll.
  - `Up/Down` only affects input cursor.
  - Mouse drag over output copies expected text.
  - Active turn still allows output scrolling and selection.
- Run full test suite.

## Testing Strategy

Unit tests:
- `ui_action::adapter` for mouse down/drag/up mapping.
- `state` tests for selection transitions and copy triggers.
- `render::output` tests for snapshot row mapping.

Behavior tests:
- Regression test for issue scenario:
  - Simulate sequence of `Up` keys and verify only input cursor moves.
  - Simulate wheel events and verify cursor does not move.

Clipboard tests:
- Mock clipboard module in tests; verify text payload and success/error handling.

Manual verification checklist:
- Drag selection on output area copies to system clipboard.
- Wheel scroll works in output.
- Keyboard `Up/Down` navigation in input remains intact.
- Works during active turn and idle state.

## Risks and Mitigations

Risk: Row-to-line mapping mismatches with wrapped text.
- Mitigation: Snapshot stores explicit visual-to-source mapping computed with same wrapping rules used by renderer.

Risk: Clipboard behavior varies by platform.
- Mitigation: OSC52 first, then OS-specific fallback; clear error status message when copy fails.

Risk: Mouse capture may affect terminal native selection semantics.
- Mitigation: Selection is now app-owned; expected behavior no longer depends on terminal native selection.

## Acceptance Criteria

- No input cursor movement from mouse wheel interactions.
- `Up/Down` keys move input cursor only.
- Output text can be selected via mouse drag and copied on mouse-up.
- Full test suite passes.

