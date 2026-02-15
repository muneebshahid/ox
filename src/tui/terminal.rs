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
    render::{self, RenderMeta},
    state::TuiState,
};

const RUNNING_STATUS_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
const TRANSIENT_NOTICE_REFRESH_INTERVAL: Duration = Duration::from_millis(100);
const TRANSIENT_NOTICE_DURATION: Duration = Duration::from_millis(1400);

pub(super) struct UiRenderer {
    terminal: TerminalGuard,
    render_meta: RenderMeta,
    running_status_last_draw_at: Option<Instant>,
    transient_notice_last_draw_at: Option<Instant>,
}

impl UiRenderer {
    pub(super) fn new(render_meta: RenderMeta) -> Result<Self> {
        Ok(Self {
            terminal: TerminalGuard::new()?,
            render_meta,
            running_status_last_draw_at: None,
            transient_notice_last_draw_at: None,
        })
    }

    pub(super) fn draw_if_needed(&mut self, state: &mut TuiState) -> Result<()> {
        let now = Instant::now();
        state.clear_expired_transient_notice(now);
        let is_running = state.status_is_running();
        let has_transient_notice = state.has_transient_notice();
        let dirty = state.take_dirty();
        let (should_draw, next_running_last_draw, next_transient_notice_last_draw) = draw_decision(
            dirty,
            is_running,
            has_transient_notice,
            self.running_status_last_draw_at,
            self.transient_notice_last_draw_at,
            now,
        );

        if should_draw {
            self.terminal.draw(state, &self.render_meta)?;
            if let Some(text) = state.take_pending_copy_text() {
                let notice = if clipboard::copy_to_clipboard(&text).is_ok() {
                    "Copied to clipboard".to_string()
                } else {
                    "Copy failed".to_string()
                };
                state.set_transient_notice(notice, TRANSIENT_NOTICE_DURATION);
            }
        }
        self.running_status_last_draw_at = next_running_last_draw;
        self.transient_notice_last_draw_at = next_transient_notice_last_draw;

        Ok(())
    }
}

fn draw_decision(
    dirty: bool,
    is_running: bool,
    has_transient_notice: bool,
    running_status_last_draw_at: Option<Instant>,
    transient_notice_last_draw_at: Option<Instant>,
    now: Instant,
) -> (bool, Option<Instant>, Option<Instant>) {
    let running_refresh_due = is_running
        && running_status_last_draw_at
            .is_none_or(|last| now.duration_since(last) >= RUNNING_STATUS_REFRESH_INTERVAL);
    let transient_notice_refresh_due = has_transient_notice
        && transient_notice_last_draw_at
            .is_none_or(|last| now.duration_since(last) >= TRANSIENT_NOTICE_REFRESH_INTERVAL);
    let should_draw = dirty || running_refresh_due || transient_notice_refresh_due;

    let next_running_last_draw = if should_draw {
        is_running.then_some(now)
    } else {
        running_status_last_draw_at
    };
    let next_transient_notice_last_draw = if should_draw {
        has_transient_notice.then_some(now)
    } else if has_transient_notice {
        transient_notice_last_draw_at
    } else {
        None
    };
    (
        should_draw,
        next_running_last_draw,
        next_transient_notice_last_draw,
    )
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

    pub(super) fn draw(&mut self, state: &mut TuiState, meta: &RenderMeta) -> Result<()> {
        self.terminal
            .draw(|frame| render::draw(frame, state, meta))?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            DisableMouseCapture,
            LeaveAlternateScreen
        );
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RUNNING_STATUS_REFRESH_INTERVAL, TRANSIENT_NOTICE_REFRESH_INTERVAL, draw_decision,
    };
    use std::time::Instant;

    #[test]
    fn dirty_state_always_draws() {
        let now = Instant::now();
        let (should_draw, next_last_draw, next_notice_draw) =
            draw_decision(true, false, false, None, None, now);
        assert!(should_draw);
        assert_eq!(next_last_draw, None);
        assert_eq!(next_notice_draw, None);
    }

    #[test]
    fn running_state_draws_when_refresh_interval_elapsed() {
        let now = Instant::now();
        let old = now - RUNNING_STATUS_REFRESH_INTERVAL;
        let (should_draw, next_last_draw, next_notice_draw) =
            draw_decision(false, true, false, Some(old), None, now);
        assert!(should_draw);
        assert_eq!(next_last_draw, Some(now));
        assert_eq!(next_notice_draw, None);
    }

    #[test]
    fn running_state_skips_draw_before_refresh_interval() {
        let now = Instant::now();
        let recent = now - (RUNNING_STATUS_REFRESH_INTERVAL / 2);
        let (should_draw, next_last_draw, next_notice_draw) =
            draw_decision(false, true, false, Some(recent), None, now);
        assert!(!should_draw);
        assert_eq!(next_last_draw, Some(recent));
        assert_eq!(next_notice_draw, None);
    }

    #[test]
    fn first_running_frame_draws_without_prior_timestamp() {
        let now = Instant::now();
        let (should_draw, next_last_draw, next_notice_draw) =
            draw_decision(false, true, false, None, None, now);
        assert!(should_draw);
        assert_eq!(next_last_draw, Some(now));
        assert_eq!(next_notice_draw, None);
    }

    #[test]
    fn transient_notice_draws_on_refresh_interval() {
        let now = Instant::now();
        let recent = now - (TRANSIENT_NOTICE_REFRESH_INTERVAL / 2);
        let stale = now - TRANSIENT_NOTICE_REFRESH_INTERVAL;

        let (should_draw_recent, _, next_notice_recent) =
            draw_decision(false, false, true, None, Some(recent), now);
        assert!(!should_draw_recent);
        assert_eq!(next_notice_recent, Some(recent));

        let (should_draw_stale, _, next_notice_stale) =
            draw_decision(false, false, true, None, Some(stale), now);
        assert!(should_draw_stale);
        assert_eq!(next_notice_stale, Some(now));
    }
}
