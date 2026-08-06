// SPDX-License-Identifier: MPL-2.0

fn main() -> cosmic::iced::Result {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    puck::i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default()
        .size_limits(cosmic::iced::Limits::NONE.min_width(360.0).min_height(180.0));

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<puck::app::AppModel>(settings, ())
}
