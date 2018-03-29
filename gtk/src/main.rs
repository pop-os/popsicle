extern crate digest;
extern crate gdk;
extern crate gtk;
extern crate md5;
extern crate pango;
extern crate popsicle;
extern crate pwd;
extern crate sha3;

mod app;
mod block;
mod hash;
mod image;

use app::{App, Connect};
use std::env;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::thread;

pub use block::BlockDevice;

fn main() {
    // If running in pkexec, restore home directory for open dialog
    if let Ok(pkexec_uid) = env::var("PKEXEC_UID") {
        if let Ok(uid) = pkexec_uid.parse::<u32>() {
            if let Some(passwd) = pwd::Passwd::from_uid(uid) {
                env::set_var("HOME", passwd.dir);
            }
        }
    }

    let (sender, receiver) = channel::<PathBuf>();
    let app = App::new(sender);

    {
        let buffer = app.state.buffer.clone();
        thread::spawn(move || image::event_loop(receiver, &buffer));

        let buffer = app.state.buffer.clone();
        let hash = app.state.hash.clone();
        thread::spawn(move || hash::event_loop(&buffer, &hash));
    }

    if let Some(iso_argument) = env::args().skip(1).next() {
        let _ = app.state.image_sender.send(PathBuf::from(iso_argument));
    }

    app.connect_events().then_execute()
}
