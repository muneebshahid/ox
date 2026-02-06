mod manager;
mod naming;
mod store;

use anyhow::{Context, Result};

pub use manager::SessionManager;
pub use naming::create_session_name;

pub fn list_sessions() -> Result<()> {
    let sessions = store::list_sessions().context("unable to list sessions")?;
    if sessions.is_empty() {
        println!("No sessions found.");
    } else {
        for name in &sessions {
            println!("{name}");
        }
    }
    Ok(())
}

pub fn open_session(session_name: &str) -> Result<SessionManager> {
    eprintln!("Using session: {session_name}");
    SessionManager::open(session_name)
}
