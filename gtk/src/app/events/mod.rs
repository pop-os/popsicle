use crate::flash::{FlashError, FlashRequest};
use crate::hash::hasher;

use crossbeam_channel::{Receiver, Sender};
use dbus_udisks2::{DiskDevice, Disks, UDisks2};
use md5::Md5;
use sha2::Sha256;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub enum UiEvent {
    SetImageLabel(PathBuf),
    RefreshDevices(Box<[Arc<DiskDevice>]>),
    SetHash(io::Result<String>),
    Flash(JoinHandle<anyhow::Result<(anyhow::Result<()>, Vec<Result<(), FlashError>>)>>),
    Reset,
}

pub enum BackgroundEvent {
    GenerateHash(PathBuf, &'static str),
    Flash(FlashRequest),
    RefreshDevices,
}

pub fn background_thread(events_tx: Sender<UiEvent>, events_rx: Receiver<BackgroundEvent>) {
    thread::spawn(move || {
        let mut hashed: HashMap<(PathBuf, &'static str), String> = HashMap::new();

        let mut device_paths = Vec::new();

        loop {
            match events_rx.recv() {
                Ok(BackgroundEvent::GenerateHash(path, kind)) => {
                    // Check if the cache already contains this hash, and return it.
                    if let Some(result) = hashed.get(&(path.clone(), kind)) {
                        let _ = events_tx.send(UiEvent::SetHash(Ok(result.clone())));
                        continue;
                    }

                    // Hash the file at the given path.
                    let result = match kind {
                        "MD5" => hasher::<Md5>(&path),
                        "SHA256" => hasher::<Sha256>(&path),
                        _ => Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "hash kind not supported",
                        )),
                    };

                    // If successful, cache the result.
                    if let Ok(ref result) = result {
                        hashed.insert((path.clone(), kind), result.clone());
                    }

                    // Send this result back to the main thread.
                    let _ = events_tx.send(UiEvent::SetHash(result));
                }
                Ok(BackgroundEvent::RefreshDevices) => {
                    // Fetch the current list of USB devices from popsicle.
                    match refresh_devices() {
                        Ok(devices) => {
                            let new_device_paths: Vec<_> =
                                devices.iter().map(|d| d.drive.path.clone()).collect();
                            if new_device_paths != device_paths {
                                device_paths = new_device_paths;
                                let _ = events_tx.send(UiEvent::RefreshDevices(devices));
                            }
                        }
                        Err(why) => eprintln!("failed to refresh devices: {}", why),
                    }
                }
                Ok(BackgroundEvent::Flash(request)) => {
                    let _ = events_tx.send(UiEvent::Flash(
                        thread::Builder::new()
                            .stack_size(10 * 1024 * 1024)
                            .spawn(|| request.write())
                            .unwrap(),
                    ));
                }
                Err(_) => break,
            }
        }
    });
}

fn refresh_devices() -> anyhow::Result<Box<[Arc<DiskDevice>]>> {
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
