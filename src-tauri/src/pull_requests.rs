//! The signed-in user's open pull requests.
//!
//! GitHub has no "list my pull requests" endpoint, so we go through the issue
//! search API, which is the only way to reach across every repository in one
//! call. `author:@me` resolves to whoever the token belongs to, so we never
//! have to send the login ourselves.
//!
//! Search returns private repositories as well, provided the token carries the
//! `repo` scope — see `REQUIRED_SCOPE` in [`crate::session`].

use chrono::{DateTime, Utc};
use chrono_humanize::HumanTime;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::session::Session;

const SEARCH_URL: &str = "https://api.github.com/search/issues";
const QUERY: &str = "is:pr is:open archived:false author:@me";
const PER_PAGE: &str = "50";

/// One row of the search response. Search returns issues, so the pull-request
/// specific fields are the ones GitHub bolts on when the issue is a PR.
#[derive(Deserialize)]
struct SearchItem {
    number: u64,
    title: String,
    html_url: String,
    updated_at: String,
    /// `https://api.github.com/repos/{owner}/{repo}` — the only place the
    /// search response names the repository.
    repository_url: String,
    #[serde(default)]
    draft: bool,
}

#[derive(Deserialize)]
struct SearchReply {
    items: Vec<SearchItem>,
}

#[derive(Serialize)]
pub struct PullRequest {
    number: u64,
    title: String,
    url: String,
    repository: String,
    /// Already worded — "3 hours ago" — rather than a timestamp for the
    /// frontend to format.
    updated: String,
    draft: bool,
}

#[tauri::command]
pub async fn list_my_pull_requests(state: State<'_, Session>) -> Result<Vec<PullRequest>, String> {
    let token = state.token()?;
    let http = state.http();

    let response = http
        .get(SEARCH_URL)
        .header("Accept", "application/vnd.github+json")
        .bearer_auth(&token)
        .query(&[
            ("q", QUERY),
            // The legacy search syntax is on its way out; opting in keeps the
            // query working once GitHub makes advanced search the only mode.
            ("advanced_search", "true"),
            ("sort", "updated"),
            ("order", "desc"),
            ("per_page", PER_PAGE),
        ])
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub rejected the search ({}).",
            response.status()
        ));
    }

    let reply: SearchReply = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from GitHub: {e}"))?;

    Ok(reply
        .items
        .into_iter()
        .map(|item| PullRequest {
            number: item.number,
            title: item.title,
            repository: repository_name(&item.repository_url),
            url: item.html_url,
            updated: relative_time(&item.updated_at),
            draft: item.draft,
        })
        .collect())
}

/// "3 hours ago" for a GitHub timestamp, in the largest unit that fits. A
/// timestamp we cannot parse is passed through as it arrived rather than
/// costing the caller the whole list.
fn relative_time(timestamp: &str) -> String {
    match DateTime::parse_from_rfc3339(timestamp) {
        Ok(at) => HumanTime::from(at.with_timezone(&Utc) - Utc::now()).to_string(),
        Err(_) => timestamp.to_string(),
    }
}

/// `owner/repo` out of a repository API URL, falling back to the whole URL if
/// GitHub ever changes its shape.
fn repository_name(url: &str) -> String {
    url.split_once("/repos/")
        .map(|(_, name)| name)
        .unwrap_or(url)
        .to_string()
}
