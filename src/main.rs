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

fn main() -> cosmic::iced::Result {
    let args = Args::parse();

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    puck::i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(360.0)
            .min_height(180.0),
    );

    cosmic::app::run::<puck::app::AppModel>(settings, args.document)
}
