// SPDX-License-Identifier: GPL-2.0-only

use std::path::PathBuf;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::app::Flags;

mod app;
mod hash;
mod i18n;
mod views;

fn main() -> cosmic::iced::Result {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    // Initialize tracing for logging and debugging.
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("popsicle_ui=info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default()
        .size_limits(cosmic::iced::Limits::NONE.min_width(470.0).min_height(310.0));

    let mut flags = Flags { iso_argument: None };

    // Checks if an ISO argument was provided.
    if let Some(path) = std::env::args_os().nth(1).map(PathBuf::from)
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("iso") || ext.eq_ignore_ascii_case("img"))
        && path.exists()
    {
        flags.iso_argument = Some(path);
    }

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, flags)
}
