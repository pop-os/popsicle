extern crate gdk;
extern crate gtk;
extern crate pango;

mod ui;

use ui::App;

fn main() { App::new().connect_events().then_execute() }
