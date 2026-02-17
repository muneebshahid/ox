use std::{
    io,
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use super::{
    clipboard,
    output_surface::selected_text,
    render::{self, RenderMeta},
    state::TuiState,
};

const RUNNING_STATUS_REFRESH_INTERVAL: Duration = Duration::from_millis(250);

pub(super) struct UiRenderer {
    terminal: TerminalGuard,
    render_meta: RenderMeta,
    running_status_last_draw_at: Option<Instant>,
}

impl UiRenderer {
    pub(super) fn new(render_meta: RenderMeta) -> Result<Self> {
        Ok(Self {
            terminal: TerminalGuard::new()?,
            render_meta,
            running_status_last_draw_at: None,
        })
    }

    pub(super) fn draw_if_needed(&mut self, state: &mut TuiState) -> Result<()> {
        let now = Instant::now();
        let is_running = state.status_is_running();
        let dirty = state.take_dirty();
        let (should_draw, next_last_draw) =
            draw_decision(dirty, is_running, self.running_status_last_draw_at, now);

        if should_draw {
            let snapshot = self.terminal.draw(state, &self.render_meta)?;
            if let Some((start, end)) = state.take_pending_copy_range()
                && let Some(text) = selected_text(&snapshot, start, end)
            {
                let _ = clipboard::copy_to_clipboard(&text);
            }
        }
        self.running_status_last_draw_at = next_last_draw;

        Ok(())
    }

    pub(super) fn sync_layout_context(&self, state: &mut TuiState) -> Result<()> {
        let area = self.terminal.size()?;
        let layout = render::layout_context(state, &self.render_meta, area);
        state.set_layout_context(layout);
        Ok(())
    }
}

fn draw_decision(
    dirty: bool,
    is_running: bool,
    running_status_last_draw_at: Option<Instant>,
    now: Instant,
) -> (bool, Option<Instant>) {
    let refresh_due = is_running
        && running_status_last_draw_at
            .is_none_or(|last| now.duration_since(last) >= RUNNING_STATUS_REFRESH_INTERVAL);
    let should_draw = dirty || refresh_due;

    let next_last_draw = if should_draw {
        is_running.then_some(now)
    } else {
        running_status_last_draw_at
    };
    (should_draw, next_last_draw)
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    pub(super) fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub(super) fn draw(
        &mut self,
        state: &TuiState,
        meta: &RenderMeta,
    ) -> Result<super::output_surface::OutputRenderSnapshot> {
        let mut snapshot = None;
        self.terminal.draw(|frame| {
            snapshot = Some(render::draw(frame, state, meta));
        })?;
        Ok(snapshot.expect("frame draw always produces output snapshot"))
    }

    pub(super) fn size(&self) -> Result<ratatui::layout::Rect> {
        Ok(self.terminal.size()?.into())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::{RUNNING_STATUS_REFRESH_INTERVAL, draw_decision};
    use std::time::Instant;

    #[test]
    fn dirty_state_always_draws() {
        let now = Instant::now();
        let (should_draw, next_last_draw) = draw_decision(true, false, None, now);
        assert!(should_draw);
        assert_eq!(next_last_draw, None);
    }

    #[test]
    fn running_state_draws_when_refresh_interval_elapsed() {
        let now = Instant::now();
        let old = now - RUNNING_STATUS_REFRESH_INTERVAL;
        let (should_draw, next_last_draw) = draw_decision(false, true, Some(old), now);
        assert!(should_draw);
        assert_eq!(next_last_draw, Some(now));
    }

    #[test]
    fn running_state_skips_draw_before_refresh_interval() {
        let now = Instant::now();
        let recent = now - (RUNNING_STATUS_REFRESH_INTERVAL / 2);
        let (should_draw, next_last_draw) = draw_decision(false, true, Some(recent), now);
        assert!(!should_draw);
        assert_eq!(next_last_draw, Some(recent));
    }

    #[test]
    fn first_running_frame_draws_without_prior_timestamp() {
        let now = Instant::now();
        let (should_draw, next_last_draw) = draw_decision(false, true, None, now);
        assert!(should_draw);
        assert_eq!(next_last_draw, Some(now));
    }
}
