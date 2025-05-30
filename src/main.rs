// SPDX-License-Identifier: GPL-3

mod app;
mod apps;
mod i18n;

fn main() -> cosmic::iced::Result {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(300.0)
            .min_height(450.0),
    );

    cosmic::app::run::<app::AppModel>(settings, ())
}
