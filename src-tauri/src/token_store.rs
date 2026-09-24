//! Where the GitHub access token lives between runs.
//!
//! The `keyring` crate picks the platform's own secret store at compile time —
//! Keychain Services on macOS, the Credential Manager on Windows, the Secret
//! Service (GNOME Keyring, KWallet) on Linux — so the token is protected by
//! the OS rather than by a file we wrote ourselves.
//!
//! Every operation here is best effort. A machine can legitimately have no
//! usable store (a headless Linux box with no Secret Service running, say), and
//! the app still works there — it just asks the user to sign in each time.

use keyring::{Entry, Error};

/// Together these name the credential the user will see in Keychain Access or
/// the Credential Manager, so they are part of the app's visible surface.
const SERVICE: &str = "jaguar";
const ACCOUNT: &str = "github-access-token";

fn entry() -> Option<Entry> {
    match Entry::new(SERVICE, ACCOUNT) {
        Ok(entry) => Some(entry),
        Err(e) => {
            eprintln!("keyring unavailable, the token will not persist: {e}");
            None
        }
    }
}

/// Remember the token, or carry on without persistence if the store refuses.
pub fn save(token: &str) {
    if let Some(entry) = entry() {
        if let Err(e) = entry.set_password(token) {
            eprintln!("could not save the token to the keyring: {e}");
        }
    }
}

/// The token from a previous run, if there is one we can read.
pub fn load() -> Option<String> {
    let entry = entry()?;
    match entry.get_password() {
        Ok(token) => Some(token),
        // Nobody has signed in on this machine yet; the common case, not a fault.
        Err(Error::NoEntry) => None,
        Err(e) => {
            eprintln!("could not read the token from the keyring: {e}");
            None
        }
    }
}

/// Forget the token. Missing is the desired end state, so it is not an error.
pub fn clear() {
    if let Some(entry) = entry() {
        match entry.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => {}
            Err(e) => eprintln!("could not delete the token from the keyring: {e}"),
        }
    }
}
