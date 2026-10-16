//! Open pull requests for the signed-in user, as one list.
//!
//! GitHub has no "list my pull requests" endpoint, so this asks the issue
//! search API, which is the only way to reach across every repository in one
//! call. `author:@me` resolves to whoever the token belongs to, so we never
//! have to send the login ourselves.
//!
//! Search does not include merge state. A second query, against the GraphQL
//! API, fills that in for the same rows. Callers only see the combined list.
//!
//! Search returns private repositories as well, provided the token carries the
//! `repo` scope — see `REQUIRED_SCOPE` in [`crate::session`].

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use chrono_humanize::HumanTime;
use serde::{Deserialize, Serialize};

const SEARCH_URL: &str = "https://api.github.com/search/issues";
const GRAPHQL_URL: &str = "https://api.github.com/graphql";
const QUERY: &str = "is:pr is:open archived:false author:@me";
const PER_PAGE: &str = "50";

/// `mergeStateStatus` is GitHub's own verdict: `CLEAN` is the only state in
/// which every requirement is already met.
const MERGE_STATES_QUERY: &str = "\
    query($ids: [ID!]!) { \
      nodes(ids: $ids) { ... on PullRequest { id mergeStateStatus } } \
    }";

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
    /// GraphQL global id, which is how the follow-up query refers to this PR.
    node_id: String,
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
    #[serde(rename = "updatedAgo")]
    updated_ago: String,
    /// Whether GitHub will currently accept a merge, or why it will not.
    #[serde(rename = "mergeStatus")]
    merge_status: MergeStatus,
}

/// Serialized in camelCase for the frontend.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum MergeStatus {
    /// `CLEAN` — every requirement is already met.
    Ready,
    /// `BLOCKED` — a required review, check, or another branch rule is still in the way.
    Blocked,
    /// `DIRTY` — the branch cannot be merged cleanly.
    Conflicts,
    /// `BEHIND` — the branch is behind its base.
    Behind,
    /// `UNSTABLE` — mergeable, but a commit status is not passing.
    Checks,
    /// `DRAFT` — still a draft, so merging is refused.
    Draft,
    /// `HAS_HOOKS` — requirements are met, but pre-receive hooks have not finished.
    Pending,
    /// No usable state came back, or GitHub returned one we do not recognise.
    Unknown,
}

/// The signed-in user's open pull requests, each already carrying its merge status.
pub async fn list_open_pull_requests(
    http: &reqwest::Client,
    token: &str,
) -> Result<Vec<PullRequest>, String> {
    let items = search_pull_requests(http, token).await?;
    let ids: Vec<String> = items.iter().map(|item| item.node_id.clone()).collect();
    let states = merge_states(http, token, &ids).await?;

    Ok(items
        .into_iter()
        .map(|item| {
            let merge_status = states
                .get(&item.node_id)
                .copied()
                .unwrap_or(MergeStatus::Unknown);
            PullRequest {
                number: item.number,
                title: item.title,
                repository: repository_name(&item.repository_url),
                url: item.html_url,
                updated_ago: relative_time(&item.updated_at),
                merge_status,
            }
        })
        .collect())
}

async fn search_pull_requests(
    http: &reqwest::Client,
    token: &str,
) -> Result<Vec<SearchItem>, String> {
    let response = http
        .get(SEARCH_URL)
        .header("Accept", "application/vnd.github+json")
        .bearer_auth(token)
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

    Ok(reply.items)
}

#[derive(Serialize)]
struct GraphqlRequest<'a> {
    query: &'a str,
    variables: GraphqlVariables<'a>,
}

#[derive(Serialize)]
struct GraphqlVariables<'a> {
    ids: &'a [String],
}

#[derive(Deserialize)]
struct GraphqlReply {
    data: Option<GraphqlData>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Deserialize)]
struct GraphqlData {
    nodes: Vec<Option<MergeNode>>,
}

#[derive(Deserialize)]
struct MergeNode {
    id: Option<String>,
    #[serde(rename = "mergeStateStatus")]
    merge_state_status: Option<String>,
}

async fn merge_states(
    http: &reqwest::Client,
    token: &str,
    ids: &[String],
) -> Result<HashMap<String, MergeStatus>, String> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let response = http
        .post(GRAPHQL_URL)
        .header("Accept", "application/vnd.github+json")
        .bearer_auth(token)
        .json(&GraphqlRequest {
            query: MERGE_STATES_QUERY,
            variables: GraphqlVariables { ids },
        })
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub rejected the merge status lookup ({}).",
            response.status()
        ));
    }

    let reply: GraphqlReply = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from GitHub: {e}"))?;

    let Some(data) = reply.data else {
        let reason = reply
            .errors
            .first()
            .map(|error| error.message.as_str())
            .unwrap_or("no merge status");
        return Err(format!(
            "GitHub rejected the merge status lookup: {reason}."
        ));
    };

    Ok(data
        .nodes
        .into_iter()
        .flatten()
        .filter_map(|node| Some((node.id?, merge_status(node.merge_state_status.as_deref()?))))
        .collect())
}

fn merge_status(github: &str) -> MergeStatus {
    match github {
        "CLEAN" => MergeStatus::Ready,
        "BLOCKED" => MergeStatus::Blocked,
        "DIRTY" => MergeStatus::Conflicts,
        "BEHIND" => MergeStatus::Behind,
        "UNSTABLE" => MergeStatus::Checks,
        "DRAFT" => MergeStatus::Draft,
        "HAS_HOOKS" => MergeStatus::Pending,
        _ => MergeStatus::Unknown,
    }
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
