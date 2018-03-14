extern crate digest;
extern crate gdk;
extern crate gtk;
extern crate md5;
extern crate muff;
extern crate pango;
extern crate sha3;

mod image;
mod ui;

use std::env;
use ui::{App, Connect};

fn main() {
    let app = App::new();

    if let Some(iso_argument) = env::args().skip(1).next() {
        let label = &app.content.image_view.image_path;
        let next = &app.header.next;
        image::load_image(&iso_argument, &app.state, label, next);
    }

    app.connect_events().then_execute()
}
