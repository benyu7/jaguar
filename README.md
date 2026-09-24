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
- The GitHub access token is kept in the OS credential store (Credential Manager
  on Windows, Keychain on macOS, Secret Service on Linux) under `jaguar` /
  `github-access-token`, so the sign-in survives a restart. "Sign out" deletes
  it. If no credential store is available the app still runs — it just asks you
  to sign in each time.