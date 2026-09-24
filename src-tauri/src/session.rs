//! Who is signed in.
//!
//! There is no sign-in flow to speak of: the identity is whoever [`crate::gh`]
//! is signed in as. Signing in and out is `gh auth login` and `gh auth logout`,
//! which is also why nothing is stored here — `gh` keeps its own credential in
//! the OS keyring.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::gh;

const USER_URL: &str = "https://api.github.com/user";
/// Private repositories are invisible to the search API without this.
const REQUIRED_SCOPE: &str = "repo";

#[derive(Serialize, Deserialize)]
pub struct User {
    login: String,
    name: Option<String>,
    avatar_url: String,
}

pub struct Session {
    http: reqwest::Client,
}

impl Session {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("jaguar/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("could not build an HTTP client"),
        }
    }

    pub fn http(&self) -> reqwest::Client {
        self.http.clone()
    }

    /// Asked of `gh` afresh each time rather than cached, so signing in or out
    /// in a terminal takes effect on the next refresh instead of at restart.
    pub fn token(&self) -> Result<String, String> {
        gh::token()
    }
}

/// Who `gh` is signed in as. The UI calls this on startup and on retry, so its
/// errors are what the user sees when something needs fixing.
#[tauri::command]
pub async fn current_user(state: State<'_, Session>) -> Result<User, String> {
    let token = state.token()?;
    fetch_user(&state.http(), &token).await
}

async fn fetch_user(http: &reqwest::Client, token: &str) -> Result<User, String> {
    let response = http
        .get(USER_URL)
        .header("Accept", "application/vnd.github+json")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("GitHub rejected the GitHub CLI's token. Run: gh auth login".into());
    }

    if !response.status().is_success() {
        return Err(format!(
            "GitHub rejected the access token ({}).",
            response.status()
        ));
    }

    // A `gh` login made without `repo` authenticates fine but sees no private
    // repositories, which would look like having no work in progress rather
    // than like a missing permission. Better to say so.
    if !scopes_are_sufficient(response.headers()) {
        return Err(format!(
            "The GitHub CLI's token does not include the {REQUIRED_SCOPE} scope, so private \
             repositories are invisible.\n\nRun: gh auth refresh -s {REQUIRED_SCOPE}"
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from GitHub: {e}"))
}

/// Whether the token carries [`REQUIRED_SCOPE`], per the `X-OAuth-Scopes`
/// header GitHub attaches to every REST response. When the header is absent we
/// cannot tell, and refusing to proceed on a guess would be worse than trying.
fn scopes_are_sufficient(headers: &reqwest::header::HeaderMap) -> bool {
    let Some(granted) = headers.get("x-oauth-scopes").and_then(|v| v.to_str().ok()) else {
        return true;
    };
    granted.split(',').any(|scope| scope.trim() == REQUIRED_SCOPE)
}
