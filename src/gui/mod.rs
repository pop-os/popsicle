//! The COSMIC front end.

mod app;
mod flash;
mod hash;
mod misc;

use cosmic::iced::Size;
use cosmic::iced::advanced::layout::Limits;
use std::path::PathBuf;

/// Runs the front end, with an image preselected when one was given on the command line.
pub fn run(image: Option<PathBuf>) -> cosmic::iced::Result {
    misc::downgrade_from_pkexec();

    let image = image.filter(|path| misc::is_image(path));

    let settings = cosmic::app::Settings::default()
        .size(Size::new(640.0, 440.0))
        .size_limits(Limits::NONE.min_width(480.0).min_height(320.0));

    cosmic::app::run::<app::App>(settings, image)
}
