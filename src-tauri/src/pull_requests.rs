//! The signed-in user's open pull requests, as the UI asks for them.
//!
//! The two GitHub calls behind this live in [`crate::github`] and come back
//! already combined.

use tauri::State;

use crate::github;
use crate::session::Session;

#[tauri::command]
pub async fn list_my_pull_requests(
    state: State<'_, Session>,
) -> Result<Vec<github::PullRequest>, String> {
    let token = state.token()?;
    github::list_open_pull_requests(&state.http(), &token).await
}
