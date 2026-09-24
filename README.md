# Jaguar

A Tauri desktop app in vanilla HTML, CSS and TypeScript.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Run
- populate .env at ./src-tauri/.env
    - CLIENT_ID: the client id of the github oauth application
- pnpm install
- pnpm tauri dev

## Details
- Only Windows, MacOS, Linux
- Sign-in asks for `read:user repo`. The `repo` scope is what lets the pull
  request list see private repositories; GitHub OAuth apps offer no read-only
  version of it, so this grants full repository access. A token stored before
  the app asked for `repo` is discarded on the next launch and you are asked to
  sign in again.
- Private repositories owned by an organisation that restricts third-party
  OAuth apps stay invisible until an org owner approves this app, even with the
  `repo` scope granted.
- The GitHub access token is kept in the OS credential store (Credential Manager
  on Windows, Keychain on macOS, Secret Service on Linux) under `jaguar` /
  `github-access-token`, so the sign-in survives a restart. "Sign out" deletes
  it. If no credential store is available the app still runs — it just asks you
  to sign in each time.