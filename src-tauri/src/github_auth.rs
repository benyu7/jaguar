//! Sign in with GitHub using the OAuth device flow.
//!
//! The device flow is the only GitHub OAuth App flow a desktop app can use
//! without shipping a client secret: we send our public client ID, GitHub hands
//! back a short user code, the user approves it in a browser, and we poll until
//! GitHub returns an access token. The token is then handed to [`token_store`],
//! which keeps it in the operating system's own secret store, so a restart
//! picks the session back up instead of asking the user to sign in again.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::token_store;

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const USER_URL: &str = "https://api.github.com/user";
const SCOPE: &str = "read:user";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const LOCK_POISONED: &str = "Sign-in state is unusable; restart the app.";

/// GitHub's reply to the device code request. `device_code` is the half the
/// user never sees and is deliberately kept on the Rust side.
#[derive(Deserialize)]
struct DeviceCode {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

/// The half of the device code the UI shows while it waits for approval.
#[derive(Serialize)]
pub struct DevicePrompt {
    user_code: String,
    verification_uri: String,
    expires_in: u64,
}

/// The token endpoint answers with HTTP 200 either way, so success and the
/// "still waiting" errors both arrive in the body.
#[derive(Deserialize)]
struct TokenReply {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    login: String,
    name: Option<String>,
    avatar_url: String,
}

pub struct Auth {
    http: reqwest::Client,
    pending: Mutex<Option<DeviceCode>>,
    token: Mutex<Option<String>>,
}

impl Auth {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("jaguar/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("could not build an HTTP client"),
            pending: Mutex::new(None),
            // Whatever the last run left behind. It is not validated here —
            // `current_user` does that on startup and discards it if GitHub has
            // since revoked it.
            token: Mutex::new(token_store::load()),
        }
    }

    /// The access token, if anyone is signed in. Lets other modules call the
    /// API without the token ever leaving the Rust side.
    pub fn token(&self) -> Result<Option<String>, String> {
        Ok(self
            .token
            .lock()
            .map_err(|_| LOCK_POISONED.to_string())?
            .clone())
    }

    /// The shared HTTP client, so callers inherit the user agent GitHub wants.
    pub fn http(&self) -> reqwest::Client {
        self.http.clone()
    }

    /// Hold on to a freshly issued token, here and on disk.
    fn remember(&self, token: String) -> Result<(), String> {
        token_store::save(&token);
        *self.token.lock().map_err(|_| LOCK_POISONED.to_string())? = Some(token);
        Ok(())
    }

    /// Drop the session everywhere, so a restart does not resurrect it.
    fn forget(&self) {
        token_store::clear();
        if let Ok(mut token) = self.token.lock() {
            *token = None;
        }
        if let Ok(mut pending) = self.pending.lock() {
            *pending = None;
        }
    }
}

fn client_id() -> Result<String, String> {
    std::env::var("CLIENT_ID")
        .map_err(|_| "CLIENT_ID is not set. Add it to src-tauri/.env.".to_string())
}

/// Step one: ask GitHub for a device code and give the UI the part to display.
#[tauri::command]
pub async fn start_device_auth(state: State<'_, Auth>) -> Result<DevicePrompt, String> {
    let client_id = client_id()?;
    let http = state.http.clone();

    let response = http
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", client_id.as_str()), ("scope", SCOPE)])
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub rejected the sign-in request ({}). Check that CLIENT_ID is correct and that \
             device flow is enabled on the OAuth app.",
            response.status()
        ));
    }

    let code: DeviceCode = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from GitHub: {e}"))?;

    let prompt = DevicePrompt {
        user_code: code.user_code.clone(),
        verification_uri: code.verification_uri.clone(),
        expires_in: code.expires_in,
    };

    *state
        .pending
        .lock()
        .map_err(|_| LOCK_POISONED.to_string())? = Some(code);

    Ok(prompt)
}

/// Step two: poll until the user approves, then store the token and report who
/// signed in. Resolves only once GitHub gives a final answer.
#[tauri::command]
pub async fn complete_device_auth(state: State<'_, Auth>) -> Result<User, String> {
    let pending = {
        let mut slot = state
            .pending
            .lock()
            .map_err(|_| LOCK_POISONED.to_string())?;
        slot.take()
    }
    .ok_or("No sign-in is in progress.")?;

    let client_id = client_id()?;
    let http = state.http.clone();

    // GitHub sets the minimum polling interval; going faster earns a slow_down.
    let mut interval = Duration::from_secs(pending.interval.max(1));
    let deadline = Instant::now() + Duration::from_secs(pending.expires_in);

    let token = loop {
        tokio::time::sleep(interval).await;

        if Instant::now() >= deadline {
            return Err("The sign-in code expired before it was approved.".into());
        }

        let reply: TokenReply = http
            .post(ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id.as_str()),
                ("device_code", pending.device_code.as_str()),
                ("grant_type", GRANT_TYPE),
            ])
            .send()
            .await
            .map_err(|e| format!("Could not reach GitHub: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Unexpected response from GitHub: {e}"))?;

        if let Some(token) = reply.access_token {
            break token;
        }

        match reply.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => interval += Duration::from_secs(5),
            Some("expired_token") => {
                return Err("The sign-in code expired before it was approved.".into())
            }
            Some("access_denied") => return Err("Sign-in was cancelled on GitHub.".into()),
            Some(other) => return Err(format!("GitHub returned an error: {other}")),
            None => return Err("Unexpected response from GitHub.".into()),
        }
    };

    let user = fetch_user(&http, &token)
        .await?
        .ok_or("GitHub rejected the token it had just issued.")?;

    state.remember(token)?;

    Ok(user)
}

/// Who is signed in, if anyone. Lets the UI recover its state across a webview
/// reload, and is where a token restored from the keyring gets checked: if the
/// user revoked it since the last run, we drop it and report nobody home.
#[tauri::command]
pub async fn current_user(state: State<'_, Auth>) -> Result<Option<User>, String> {
    let Some(token) = state.token()? else {
        return Ok(None);
    };

    match fetch_user(&state.http(), &token).await? {
        Some(user) => Ok(Some(user)),
        None => {
            state.forget();
            Ok(None)
        }
    }
}

#[tauri::command]
pub fn sign_out(state: State<'_, Auth>) {
    state.forget();
}

/// `Ok(None)` means GitHub refused the token — revoked, expired, or for an app
/// the user has since removed. That is a fact about the session, not a failure,
/// so it is not an `Err`.
async fn fetch_user(http: &reqwest::Client, token: &str) -> Result<Option<User>, String> {
    let response = http
        .get(USER_URL)
        .header("Accept", "application/vnd.github+json")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(format!(
            "GitHub rejected the access token ({}).",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map(Some)
        .map_err(|e| format!("Unexpected response from GitHub: {e}"))
}
