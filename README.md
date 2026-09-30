# Jaguar

A Tauri desktop app in React, CSS and TypeScript.
- shadcn/ui

## Prerequisites
- gh cli

## Run
- install the [GitHub CLI](https://cli.github.com) and run `gh auth login`
- pnpm install
- pnpm tauri dev
- pnpm tauri build
- ./src-tauri/target/release/jaguar
- sudo apt install ./src-tauri/target/release/bundle/deb/Jaguar_0.1.0_amd64.deb

## Scripts
- node scripts/shadcn-diff.mjs
  - checks the difference of generation for components/ui between current/generated

## Details
- Windows, MacOS, Linux
- Authentication is the GitHub CLI's gh