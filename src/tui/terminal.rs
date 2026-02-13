use std::{
    io,
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use super::{
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
        let refresh_due = is_running
            && self
                .running_status_last_draw_at
                .is_none_or(|last| now.duration_since(last) >= RUNNING_STATUS_REFRESH_INTERVAL);
        let dirty = state.take_dirty();

        if dirty || refresh_due {
            self.terminal.draw(state, &self.render_meta)?;
            self.running_status_last_draw_at = is_running.then_some(now);
        }

        Ok(())
    }
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    pub(super) fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub(super) fn draw(&mut self, state: &TuiState, meta: &RenderMeta) -> Result<()> {
        self.terminal
            .draw(|frame| render::draw(frame, state, meta))?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
