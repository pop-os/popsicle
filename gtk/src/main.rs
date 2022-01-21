#![allow(unknown_lints)]

#[macro_use]
extern crate cascade;

mod app;
mod flash;
mod gresource;
mod hash;
mod localize;
mod misc;

use crate::app::events::UiEvent;
use crate::app::state::State;
use crate::app::App;
use i18n_embed::DesktopLanguageRequester;
use std::env;
use std::path::PathBuf;

fn main() {
    let localizer = crate::localize::localizer();
    let requested_languages = DesktopLanguageRequester::requested_languages();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!("Error while loading languages for library_fluent {}", error);
    }

    gtk::init().unwrap();

    gresource::init().expect("failed to init popsicle gresource");

    glib::set_program_name("Popsicle".into());
    glib::set_application_name("Popsicle");

    let app = App::new(State::new());

    if let Some(iso_argument) = env::args().nth(1) {
        let path = PathBuf::from(iso_argument);
        if path.extension().map_or(false, |ext| {
            let lower_ext = ext.to_str().expect("Could not convert CStr to Str").to_lowercase();
            lower_ext == "iso" || lower_ext == "img"
        }) && path.exists()
        {
            let _ = app.state.ui_event_tx.send(UiEvent::SetImageLabel(path));
        }
    }

    app.connect_events().then_execute();
}
