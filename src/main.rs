// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// The Puck document to open.
    document: Option<PathBuf>,
}

fn main() -> iced::Result {
    let args = Args::parse();

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    puck::i18n::init(&requested_languages);

    let document = args.document;
    iced::application(
        move || puck::app::AppModel::new(document.clone()),
        puck::app::AppModel::update,
        puck::app::AppModel::view,
    )
    .title(puck::app::AppModel::title)
    .executor::<iced::executor::Default>()
    .window(iced::window::Settings {
        min_size: Some(iced::Size::new(360.0, 180.0)),
        ..iced::window::Settings::default()
    })
    .run()
}
