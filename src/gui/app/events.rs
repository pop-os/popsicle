//! Blocking work that runs on background threads.

use crate::gui::flash::{FlashError, FlashRequest};
use crate::gui::hash::hasher;

use blake2::Blake2b512;
use dbus_udisks2::{DiskDevice, Disks, UDisks2};
use futures::channel::oneshot;
use iso9660::ISO9660;
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub type FlashResult = anyhow::Result<(anyhow::Result<()>, Vec<Result<(), FlashError>>)>;

/// Hash algorithms offered by the image view, in dropdown order after `None`.
pub const HASH_KINDS: [&str; 5] = ["SHA512", "SHA256", "SHA1", "MD5", "BLAKE2b"];

/// Runs a blocking function on its own thread and awaits its result.
pub async fn blocking<T: Send + 'static>(func: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        let _ = tx.send(func());
    });
    rx.await.expect("background thread panicked")
}

/// Hashes the image at the given path with the named algorithm.
pub fn hash(path: &Path, kind: &str) -> io::Result<String> {
    match kind {
        "MD5" => hasher::<Md5>(path),
        "SHA256" => hasher::<Sha256>(path),
        "SHA1" => hasher::<Sha1>(path),
        "SHA512" => hasher::<Sha512>(path),
        "BLAKE2b" => hasher::<Blake2b512>(path),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "hash kind not supported")),
    }
}

/// Reads the size of an image, and whether it is a Windows ISO.
pub fn inspect_image(path: &Path) -> io::Result<(u64, bool)> {
    let file = File::open(path)?;
    let size = file.metadata().map_or(0, |m| m.len());
    Ok((size, is_windows_iso(&file)))
}

fn is_windows_iso(file: &File) -> bool {
    if let Ok(fs) = ISO9660::new(file) {
        return fs.publisher_identifier() == "MICROSOFT CORPORATION";
    }
    false
}

/// Fetches the current list of USB devices from `UDisks2`.
pub fn refresh_devices() -> anyhow::Result<Box<[Arc<DiskDevice>]>> {
    let udisks = UDisks2::new()?;
    let devices = Disks::new(&udisks).devices;
    let mut devices = devices
        .into_iter()
        .filter(|d| d.drive.connection_bus == "usb" || d.drive.connection_bus == "sdio")
        .filter(|d| d.parent.size != 0)
        .map(Arc::new)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    devices.sort_by_key(|d| d.drive.id.clone());
    Ok(devices)
}

/// Writes the image on a dedicated thread with a stack large enough for the copy task.
pub fn spawn_flash(request: FlashRequest) -> io::Result<JoinHandle<FlashResult>> {
    thread::Builder::new().stack_size(10 * 1024 * 1024).spawn(|| request.write())
}
