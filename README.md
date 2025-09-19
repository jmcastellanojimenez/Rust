# Rust Learning Workspace

A learning-oriented multi-application Rust workspace. Each app lives in its own folder (one Cargo crate per directory) with its own README, while the workspace lets you build, test, and run everything together or individually.

## Why a Cargo workspace?

- Single `target/` directory and single `Cargo.lock` for faster builds and less duplication
- Build/test all apps at once or run a specific app easily
- Scales as you add more examples
- Optionally share code via workspace libraries

## Repository structure

- Workspace root:
  - `Cargo.toml` with `[workspace]` listing all app members
  - `README.md` (this file)
  - Optional: `.gitignore`, `rust-toolchain.toml`, CI config
- One subdirectory per app, for example:
  - `01-guessing-game/` — introductory console app
  - `02-web-server/` — minimal REST API example

Each app contains:
- `Cargo.toml`
- `src/`
- `README.md` explaining how to run/test and the concepts it demonstrates

## Getting started

- Build all apps:
  ```
  cargo build --workspace
  ```
- Test all apps:
  ```
  cargo test --workspace
  ```
- Run a specific app (example):
  ```
  cargo run -p web-server-01
  ```
- Add a dependency to a specific app:
  ```
  cargo add <crate> -p <app-name>
  ```

## Workspace configuration

At the repository root, `Cargo.toml` should include something like:
toml [workspace] members = resolver = "2"

Tips:
- If you add a new app, create its folder and add it to `members`.
- If an app isn’t ready yet, remove it from `members` temporarily to avoid build errors.

## Lockfile and target directory

- Workspaces use a single `Cargo.lock` at the repository root.
  - Remove any `Cargo.lock` inside app folders—Cargo manages the root lockfile.
- Keep build artifacts out of Git with a root `.gitignore`:
  ```
  /target
  ```
- If cloud sync causes churn with `target`, move the build directory:
  - Environment variable:
    ```
    CARGO_TARGET_DIR=/path/outside/sync
    ```
  - Or a root `.cargo/config.toml`:
    ```toml
    [build]
    target-dir = "/path/outside/sync"
    ```

## Adding a new app

1. Create a new directory (e.g., `02-new-app/`) with:
   ```
   cargo new 02-new-app
   ```
2. Add it to the root workspace `members` in `Cargo.toml`.
3. Create a `README.md` in the new app describing:
   - Learning goals (e.g., ownership, async, web, error handling)
   - How to run and test
   - Key concepts and code pointers

## Conventions

- Name apps with numeric prefixes to reflect progression (e.g., `01-…`, `02-…`).
- Each app should include:
  - A minimal, runnable example
  - Brief notes on the Rust concepts demonstrated
  - Simple tests or a script showing how to exercise the code

## Optional but recommended

- Pin the toolchain at the root with `rust-toolchain.toml` for consistency across machines.
- Add CI to run:
  ```
  cargo fmt --check
  cargo clippy --workspace -- -D warnings
  cargo test --workspace
  ```
- If multiple apps share code, create a workspace library crate (e.g., `libs/common/`) and add it as a member.

## Troubleshooting

- “Package not found” when running `cargo run -p <name>`:
  - Ensure the app directory exists and `<name>` matches the crate’s `package.name` in its `Cargo.toml`.
  - Verify the app is listed in the root workspace `members`.
- Build errors after adding a new app:
  - Temporarily remove the app from `members` until it compiles.
  - Run `cargo clean` if you suspect stale artifacts.
