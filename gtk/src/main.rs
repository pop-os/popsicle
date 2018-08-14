#![allow(unknown_lints)]
#![allow(option_map_unit_fn)]

extern crate bus_writer;
#[macro_use]
extern crate cascade;
extern crate digest;
extern crate gdk;
extern crate gtk;
extern crate hex_view;
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

use app::{App, Connect};
use hash::HashState;
use std::env;
use std::path::PathBuf;
use std::thread;
use std::io;
use std::time::Duration;
use std::fs::File;
use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
use std::sync::Arc;
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
        flash_response_sender,
    );

    // If running in pkexec or sudo, restore home directory for open dialog,
    // and then downgrade permissions back to a regular user.
    if let Ok(pkexec_uid) = env::var("PKEXEC_UID").or_else(|_| env::var("SUDO_UID")) {
        if let Ok(uid) = pkexec_uid.parse::<u32>() {
            if let Some(passwd) = pwd::Passwd::from_uid(uid) {
                env::set_var("HOME", passwd.dir);
                downgrade_permissions(passwd.uid, passwd.gid);
            }
        }
    }

    let (hash_tx, hash_rx) = channel::<(PathBuf, &'static str)>();
    let hash_state = Arc::new(HashState::new());

    {
        let hash_state = hash_state.clone();
        thread::spawn(move || hash::event_loop(&hash_rx, &hash_state));
    }

    let app = App::new(hash_state, hash_tx, devices_request, devices_response, flash_request, flash_response);

    if let Some(iso_argument) = env::args().nth(1) {
        let path = PathBuf::from(iso_argument);
        // TODO: Write an error message on failure.
        if let Ok(file) = File::open(&path) {
            if let Ok(size) = file.metadata().map(|m| m.len() as usize) {
                *app.state.image.write().unwrap() = Some((path, size));
            }
        }
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
    flash_response: Sender<JoinHandle<io::Result<Vec<io::Result<()>>>>>,
) {
    thread::spawn(move || {
        loop {
            let mut disconnected = 0;

            match devices_request.try_recv() {
                Ok((devs, mounts)) => {
                    let resp = popsicle::disks_from_args(devs.into_iter(), &mounts, true);
                    let _ = devices_response.send(resp);
                }
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => disconnected += 1,
            }

            match flash_request.try_recv() {
                Ok(flash_request) => {
                    let _ = flash_response.send(
                        thread::Builder::new()
                            .stack_size(10 * 1024 * 1024)
                            .spawn(move || flash_request.write())
                            .unwrap()
                    );
                }
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => disconnected += 1,
            }

            if disconnected == 2 {
                break
            }

            thread::sleep(Duration::from_millis(1));
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
