# Jaguar

A Tauri desktop app in vanilla HTML, CSS and TypeScript.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Run
- install the [GitHub CLI](https://cli.github.com) and run `gh auth login`
- pnpm install
- pnpm tauri dev

## Details
- Only Windows, MacOS, Linux
- Authentication is the GitHub CLI's. jaguar shells out to `gh auth token` and
  uses that credential, so it has no OAuth app of its own, stores no token, and
  has no sign-in screen — `gh auth login` and `gh auth logout` are the sign-in
  and sign-out. `gh` keeps its own credential in the OS keyring.
- This is deliberate: an OAuth app of jaguar's own would need separate approval
  from every organisation that restricts third-party apps, whereas `gh` has
  generally been approved already.
- The token needs the `repo` scope for private repositories to appear in the
  pull request list. If it is missing, the app says so and gives you the fix
  (`gh auth refresh -s repo`).