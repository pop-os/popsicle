#![allow(unknown_lints)]

extern crate atomic;
extern crate bus_writer;
#[macro_use]
extern crate cascade;
extern crate crossbeam_channel;
extern crate digest;
extern crate gdk;
extern crate glib;
extern crate gtk;
extern crate humansize;
extern crate hex_view;
extern crate libc;
extern crate md5;
extern crate pango;
extern crate popsicle;
extern crate parking_lot;
extern crate proc_mounts;
extern crate pwd;
extern crate sha2;
extern crate sysfs_class;
extern crate sys_mount;

mod app;
mod block;
mod flash;
mod hash;

use app::App;
use app::events::UiEvent;
use app::state::State;
use std::env;
use std::path::PathBuf;

fn main() {
    glib::set_program_name("Popsicle".into());
    glib::set_application_name("Popsicle");

    let app = App::new(State::new());

    if let Some(iso_argument) = env::args().nth(1) {
        let path = PathBuf::from(iso_argument);
        if path.extension().map_or(false, |ext| ext == "iso" || ext == "img") && path.exists() {
            let _ = app.state.ui_event_tx.send(UiEvent::SetImageLabel(path));
        }
    }

    app.connect_events().then_execute();
}
