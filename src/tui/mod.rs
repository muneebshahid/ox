use crate::app_context;
use crate::session;

pub async fn run(
    _app_context: &app_context::AppContext,
    _session_manager: &mut session::SessionManager,
) -> anyhow::Result<()> {
    // Placeholder for TUI implementation
    Ok(())
}
