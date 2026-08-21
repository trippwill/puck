# Puck

Dynamically structured notes

## Installation

Project commands are defined as [mise tasks][mise-tasks].

- `mise run` builds the application with the default `build-release` task
- `mise run run` builds and runs the application
- `mise run install` installs the project into the system
- `mise run vendor` creates a vendored tarball
- `mise run build-vendored` compiles with vendored dependencies from that tarball
- `mise run check` checks source attribution, dependencies, tasks, and Rust lints
- `mise run check-json` emits Cargo JSON output for editors and other tools
- `mise run pre-commit` runs the fast checks developers should run before committing
- `mise run ci` runs the complete check and test gate

## Command-line usage

`puckctl` takes the document path before its command. Commands that create objects print IDs that
can be passed to later commands:

```sh
mise run ctl -- notes.puck new
note_id=$(mise run ctl -- notes.puck note add "Remember this")
mise run ctl -- notes.puck note read "$note_id"

collection_id=$(mise run ctl -- notes.puck collection add Tasks)
record_id=$(mise run ctl -- notes.puck record add "$collection_id")
title_id=$(mise run ctl -- notes.puck field-def add text Title)
mise run ctl -- notes.puck field set "$record_id" "$title_id" "Ship Puck"
mise run ctl -- notes.puck field read "$record_id" "$title_id"
```

## Translators

[Fluent][fluent] is used for localization of the software. Fluent's translation files are found in the [i18n directory](./i18n). New translations may copy the [English (en) localization](./i18n/en) of the project, rename `en` to the desired [ISO 639-1 language code][iso-codes], and then translations can be provided for each [message identifier][fluent-guide]. If no translation is necessary, the message may be omitted.

## Packaging

If packaging for a Linux distribution, vendor dependencies locally with the `vendor` task, and build with the vendored sources using the `build-vendored` task. Set `DESTDIR` and `PREFIX` to change installation paths.

```sh
mise run vendor
mise run build-vendored
DESTDIR=debian/puck PREFIX=/usr mise run install
```

It is recommended to build a source tarball with the vendored dependencies, which can typically be done by running `mise run vendor` on the host system before it enters the build environment.

## Developers

Developers should install [rustup][rustup] and [mise][mise], then run `mise install` to
install the pinned auxiliary tools. Rustup uses [`rust-toolchain.toml`](./rust-toolchain.toml)
to install the project's Rust toolchain, including rust-analyzer. To improve compilation
times, configure [sccache][sccache] for use with Rust. Release tasks disable LTO so the
mise-managed [mold][mold] linker can improve link times on Linux.

[fluent]: https://projectfluent.org/
[fluent-guide]: https://projectfluent.org/fluent/guide/hello.html
[iso-codes]: https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes
[mise]: https://mise.jdx.dev/
[mise-tasks]: https://mise.jdx.dev/tasks/
[rustup]: https://rustup.rs/
[rust-analyzer]: https://rust-analyzer.github.io/
[mold]: https://github.com/rui314/mold
[sccache]: https://github.com/mozilla/sccache
