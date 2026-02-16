use super::render::RenderMeta;
use crate::app_context::AppContext;

pub(super) fn build_render_meta(app: &AppContext) -> RenderMeta {
    let cwd = current_dir_for_banner();
    let git_branch = current_git_branch();
    RenderMeta::new(
        app.auth.model().to_string(),
        app.auth.reasoning_setting().to_string(),
        app.auth.mode_name().to_string(),
        cwd,
        git_branch,
    )
}

fn current_dir_for_banner() -> String {
    let path =
        std::env::current_dir().map_or_else(|_| ".".to_string(), |path| path.display().to_string());

    if let Ok(home) = std::env::var("HOME")
        && path.starts_with(&home)
    {
        return format!("~{}", &path[home.len()..]);
    }

    path
}

fn current_git_branch() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;

    let branch = String::from_utf8(output.stdout).ok()?.trim().to_string();
    Some(branch).filter(|b| !b.is_empty() && b != "HEAD")
}
