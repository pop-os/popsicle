extern crate digest;
extern crate gdk;
extern crate gtk;
extern crate libc;
extern crate md5;
extern crate pango;
extern crate popsicle;
extern crate pwd;
extern crate sha2;

mod app;
mod block;
mod flash;
mod hash;
mod image;

use app::{App, Connect};
use std::env;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::thread;
use std::fs::File;
use std::sync::mpsc::{Sender, Receiver};
use std::thread::JoinHandle;

pub use block::BlockDevice;
pub use flash::FlashRequest;

use popsicle::mnt::MountEntry;
use popsicle::DiskError;

fn main() {
    let (devices_request, devices_request_receiver) =
        channel::<(Vec<String>, Vec<MountEntry>)>();
    let (devices_response_sender, devices_response) =
        channel::<Result<Vec<(String, File)>, DiskError>>();
    let (flash_request, flash_request_receiver) = channel::<FlashRequest>();
    let (flash_response_sender, flash_response) = channel();

    authenticated_threads(
        devices_request_receiver,
        devices_response_sender,
        flash_request_receiver,
        flash_response_sender
    );

    // If running in pkexec or sudo, restore home directory for open dialog,
    // and then downgrade permissions back to a regular user.
    if let Ok(pkexec_uid) = env::var("PKEXEC_UID").or(env::var("SUDO_UID")) {
        if let Ok(uid) = pkexec_uid.parse::<u32>() {
            if let Some(passwd) = pwd::Passwd::from_uid(uid) {
                env::set_var("HOME", passwd.dir);
                downgrade_permissions(passwd.uid, passwd.gid);
            }
        }
    }

    let (sender, receiver) = channel::<PathBuf>();
    let app = App::new(sender, devices_request, devices_response, flash_request, flash_response);

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

/// Actions which require root authentication will be spawned as background threads from here.
///
/// This function should be called before `downgrade_permissions()`.
fn authenticated_threads(
    devices_request: Receiver<(Vec<String>, Vec<MountEntry>)>,
    devices_response: Sender<Result<Vec<(String, File)>, DiskError>>,
    flash_request: Receiver<FlashRequest>,
    flash_response: Sender<JoinHandle<Result<(), DiskError>>>
) {
    thread::spawn(move || {
        while let Ok((devs, mounts)) = devices_request.recv() {
            let resp = popsicle::disks_from_args(devs.into_iter(), &mounts, true);
            let _ = devices_response.send(resp);
        }
    });

    thread::spawn(move || {
        while let Ok(flash_request) = flash_request.recv() {
            let handle = thread::spawn(move || flash_request.write());
            let _ = flash_response.send(handle);
        }
    });
}

/// Downgrades the permissions of the current thread to the specified user and group ID.
fn downgrade_permissions(uid: u32, gid: u32) {
    unsafe {
        // By using system calls directly, we apply this on a per-thread basis.
        // The setresuid() and setresguid() functions apply to all threads.
        libc::syscall(libc::SYS_setresgid, gid, gid, gid);
        libc::syscall(libc::SYS_setresuid, uid, uid, uid);
    }
}
