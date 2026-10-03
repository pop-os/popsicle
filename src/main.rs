//! USB flasher with a COSMIC front end and a command line interface.

mod cli;
mod gui;
mod localize;

use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    localize::localize();

    // No argument, or a single image path, opens the front end; anything else is the CLI.
    let mut args = env::args().skip(1);
    let image = match (args.next(), args.next()) {
        (None, _) => None,
        (Some(arg), None) if !arg.starts_with('-') => Some(PathBuf::from(arg)),
        _ => return cli::run(),
    };

    if let Err(why) = gui::run(image) {
        eprintln!("popsicle: {why}");
        process::exit(1);
    }
}
