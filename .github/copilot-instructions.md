# Copilot instructions for Puck

API Compatibility is NOT a concern until there has been a release.

## Commands

- Build release: `just` or `just build-release`
- Build debug: `just build-debug`
- Run the app: `just run`
- Run the cli: `just run --bin puckctl`
- Lint: `just check` (`cargo clippy --all-features --locked -- -W clippy::pedantic`)
- Full tests: `cargo test --locked`
- Single test: `cargo test --locked --lib core::note::pile_note::tests::empty_body_should_return_error`
- Packaging/vendor flow: `just vendor`, then `just build-vendored`

`just` recipes pass extra Cargo args through `*args`, so prefer them over duplicating build flags. The Linux release/run recipes intentionally disable release LTO and use mold.

## Architecture

- `src/main.rs` is the GUI entry point: it selects desktop languages, initializes `puck::i18n`, configures COSMIC window limits, then runs `puck::app::AppModel`.
- `src/app.rs` is still COSMIC template/demo code, not product logic. It demonstrates the app shell (`AppModel`, messages, nav, subscriptions, context drawer), but product behavior currently lives in `src/core/` and `src/bin/puckctl.rs`.
- `src/core/note/` is the domain layer. `PileNote` validates non-empty bodies, owns revision/timestamps, and creates summaries; `NoteId` wraps UUID v7 IDs.
- `src/bin/puckctl.rs` is a separate CLI binary using clap/tracing. It currently only parses `document new <file>` and logs the intended action.
- Localization is embedded from `i18n/` with `rust-embed` and `i18n-embed`; UI strings should use the exported `fl!` macro and entries in `i18n/en/puck.ftl`.
- `resources/` plus `justfile` handle Linux desktop/appstream/icon installation. Keep packaging app IDs and `AppModel::APP_ID` aligned when changing identity/config behavior.

## Conventions

- Use Rust 2024 and keep `Cargo.lock` authoritative; existing commands use `--locked`.
- Do not infer product requirements from the demo GUI pages, timer, or placeholder app ID.
- New COSMIC UI state should be explicit fields on `AppModel`, with user events added to `Message` and handled in `update`.
- Long-running UI work belongs in `subscription()` and emits `Message` values rather than spawning detached tasks from widgets.
- Core note invariants live in constructors/methods returning `Result<_, NoteError>`; keep validation there instead of duplicating it in callers.
- Config fields live in `src/config.rs` under `#[derive(CosmicConfigEntry)]`; bump the config `#[version = ...]` when changing persisted shape.
