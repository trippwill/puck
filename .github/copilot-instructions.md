# Copilot instructions for Puck

API Compatibility is NOT a concern until there has been a release.

## Commands

- Build release: `just` or `just build-release`
- Build debug: `just build-debug`
- Run the app: `just run`
- Run the cli: `just run --bin puckctl`
- Format: `just fmt`
- Check formatting: `just fmt -- --check`
- Lint: `just check`
- Full tests: `just test`
- Single test: `just test -- core::note::tests::empty_body_should_return_error`
- Packaging/vendor flow: `just vendor`, then `just build-vendored`

Always use the `just fmt`, `just test`, and `just check` targets instead of invoking `cargo fmt`,
`cargo test`, or `cargo clippy` directly. The recipes pass extra Cargo arguments through `*args` and
define the required toolchain, profile, features, and platform flags.

## Architecture

- `src/main.rs` is the GUI entry point: it selects desktop languages, initializes `puck::i18n`, configures COSMIC window limits, then runs `puck::app::AppModel`.
- `src/app.rs` is still COSMIC template/demo code, not product logic. It demonstrates the app shell (`AppModel`, messages, nav, subscriptions, context drawer), but product behavior currently lives in `src/core/` and `src/bin/puckctl.rs`.
- `src/core/` contains owned domain values and query projections. The GUI is not data-bound: it owns
  materialized values, changes them through messages in `update`, and handles query and command
  results explicitly.
- The exact `Document` persistence API is intentionally undecided until it is implemented. Do not
  assume aggregates, modeled command payloads, raw ID/value payloads, repository traits, event
  sourcing, or caching without a concrete need.
- Domain IDs are small, opaque values used by GUI messages and `Document` lookups. Keep their storage
  representation private.
- There is no domain record schema or allowed-field set. A collection's available fields are derived
  from the field definitions used by its records. `SchemaVersion` refers only to the SQLite storage
  format.
- Collections, records, and field definitions have domain identity. Field values do not: a value is
  identified by its `(RecordId, FieldDefId)` relationship and is replaced in place.
- `src/bin/puckctl.rs` is a separate CLI binary using clap/tracing. It currently only parses `document new <file>` and logs the intended action.
- Localization is embedded from `i18n/` with `rust-embed` and `i18n-embed`; UI strings should use the exported `fl!` macro and entries in `i18n/en/puck.ftl`.
- `resources/` plus `justfile` handle Linux desktop/appstream/icon installation. Keep packaging app IDs and `AppModel::APP_ID` aligned when changing identity/config behavior.

## Conventions

- Use Rust 2024 and keep `Cargo.lock` authoritative; existing commands use `--locked`.
- Write type documentation in the style of `Document`: open with a short noun phrase that says
  what the type is (for example, `An open Puck document.`), then explain ownership, invariants, or
  construction only when useful.
- Do not begin doc comments with filler such as "Represents a...", "This struct...", or "A struct
  that...". Describe behavior and constraints rather than restating the declaration.
- Do not infer product requirements from the demo GUI pages, timer, or placeholder app ID.
- New COSMIC UI state should be explicit fields on `AppModel`, with user events added to `Message` and handled in `update`.
- Long-running UI work belongs in `subscription()` and emits `Message` values rather than spawning detached tasks from widgets.
- Core note invariants live in constructors/methods returning `Result<_, NoteError>`; keep validation there instead of duplicating it in callers.
- Config fields live in `src/config.rs` under `#[derive(CosmicConfigEntry)]`; bump the config `#[version = ...]` when changing persisted shape.
