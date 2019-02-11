use block::BlockDevice;
use crossbeam_channel::{Sender, Receiver};
use flash::FlashRequest;
use hash::hasher;
use md5::Md5;
use popsicle;
use sha2::Sha256;
use std::collections::HashMap;
use std::io;
use std::mem;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};
use std::time::Instant;

pub enum UiEvent {
    SetImageLabel(PathBuf),
    RefreshDevices(Box<[BlockDevice]>),
    SetHash(io::Result<String>),
    Flash(JoinHandle<io::Result<Vec<io::Result<()>>>>),
    Reset
}

pub enum BackgroundEvent {
    GenerateHash(PathBuf, &'static str),
    RefreshDevices
}

pub enum PrivilegedEvent {
    Flash(FlashRequest),
}

/// Actions which require root authentication will be spawned as background threads from here.
///
/// This function should be called before `downgrade_permissions()`.
pub fn privileged(
    events_tx: Sender<UiEvent>,
    events_rx: Receiver<PrivilegedEvent>
) {
    thread::spawn(move || {
        while let Ok(PrivilegedEvent::Flash(request)) = events_rx.recv() {
            let _ = events_tx.send(UiEvent::Flash(
                thread::Builder::new()
                    .stack_size(10 * 1024 * 1024)
                    .spawn(move || request.write())
                    .unwrap()
            ));
        }
    });
}

pub fn unprivileged(
    events_tx: Sender<UiEvent>,
    events_rx: Receiver<BackgroundEvent>
) {
    thread::spawn(move || {
        let mut hashed: HashMap<(PathBuf, &'static str), String> = HashMap::new();

        let mut devices = Vec::new();
        let mut devices_cmp = Vec::new();

        loop {
            match events_rx.recv() {
                Ok(BackgroundEvent::GenerateHash(path, kind)) => {
                    // Check if the cache already contains this hash, and return it.
                    if let Some(result) = hashed.get(&(path.clone(), kind)) {
                        let _ = events_tx.send(UiEvent::SetHash(Ok(result.clone())));
                        continue
                    }

                    // Hash the file at the given path.
                    let result = match kind {
                        "MD5" => hasher::<Md5>(&path),
                        "SHA256" => hasher::<Sha256>(&path),
                        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "hash kind not supported"))
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
                    match popsicle::get_disk_args(&mut devices_cmp) {
                        Ok(()) => {
                            if devices_cmp != devices {
                                // If they differed, the new device vec is the current device vec.
                                mem::swap(&mut devices_cmp, &mut devices);

                                // Attempt to fetch more detailed information on the block devices.
                                match fetch_block_devices(&devices) {
                                    Ok(devices) => {
                                        // Signal to the UI to refresh the list.
                                        let _ = events_tx.send(UiEvent::RefreshDevices(devices));
                                    }
                                    Err(why) => eprintln!("failed to fetch block info: {}", why)
                                }
                            }

                            devices_cmp.clear();
                        }
                        Err(why) => eprintln!("failed to refresh devices: {}", why)
                    }
                }
                Err(_) => break
            }
        }
    });
}

fn fetch_block_devices(devices: &[String]) -> io::Result<Box<[BlockDevice]>> {
    let mut output = Vec::new();
    let start = Instant::now();

    for device in devices {
        if let Ok(ref device) = Path::new(&device).canonicalize() {
            let mut block = BlockDevice::new_from(device)?;

            // In the case of a device with a sector count of 0, it will be polled again.
            // The polling will stop if there is no change after 5 seconds since this function
            // began execution.
            while block.sectors == 0 && Instant::now().duration_since(start).as_secs() < 5 {
                block.recheck_size();
            }

            output.push(block);
        }

    }

    Ok(output.into_boxed_slice())
}
