//! Credentials borrowed from the GitHub CLI.
//!
//! jaguar has no OAuth app of its own. It asks `gh` for the token `gh` already
//! holds, which means it inherits every organisation approval `gh` has been
//! granted and never needs to be approved as a third-party app itself. The
//! cost is a hard dependency: `gh` has to be installed and signed in.

use std::io::ErrorKind;
use std::process::Command;

/// `gh`, with the environment cleaned up.
fn gh() -> Command {
    let mut command = Command::new("gh");

    // A shell that exported GH_TOKEN or GITHUB_TOKEN passes it down to us, and
    // `gh` prefers an environment token over its own stored credentials — so a
    // stale value in the launching shell would silently override the real
    // login. Neither is any use to us, so both go.
    command.env_remove("GH_TOKEN").env_remove("GITHUB_TOKEN");

    // Spawning a console program from a windowed app flashes up a console
    // unless it is asked not to.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

/// The GitHub token `gh` is signed in with.
///
/// Every error here is something the user can act on, so they are written to
/// be shown as-is.
pub fn token() -> Result<String, String> {
    let output = gh().args(["auth", "token"]).output().map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            "The GitHub CLI is not installed, or is not on PATH. See https://cli.github.com."
                .to_string()
        } else {
            format!("Could not run the GitHub CLI: {e}")
        }
    })?;

    if !output.status.success() {
        // `gh` explains itself well on stderr — "You are not logged into any
        // GitHub hosts" and so on — so pass its own words along.
        let reason = String::from_utf8_lossy(&output.stderr);
        let reason = reason.trim();
        return Err(if reason.is_empty() {
            "The GitHub CLI has no token. Run: gh auth login".to_string()
        } else {
            format!("{reason}\n\nRun: gh auth login")
        });
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return Err("The GitHub CLI returned an empty token. Run: gh auth login".into());
    }

    Ok(token)
}
