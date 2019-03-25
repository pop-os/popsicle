use atomic::Atomic;
use bus_writer::{BusWriter, BusWriterMessage};
use libc;
use proc_mounts::MountList;
use std::fs::{self, File};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use sys_mount::{unmount, UnmountFlags};

#[derive(Clone, Copy, PartialEq)]
pub enum FlashStatus {
    Inactive,
    Active,
    Killing,
}

pub struct FlashRequest {
    source: File,
    destinations: Vec<PathBuf>,
    status: Arc<Atomic<FlashStatus>>,
    progress: Arc<Vec<Atomic<u64>>>,
    finished: Arc<Vec<Atomic<bool>>>,
}

pub struct FlashTask {
    pub progress: Arc<Vec<Atomic<u64>>>,
    pub previous: Arc<Mutex<Vec<[u64; 7]>>>,
    pub finished: Arc<Vec<Atomic<bool>>>,
}

impl FlashRequest {
    pub fn new(
        source: File,
        destinations: Vec<PathBuf>,
        status: Arc<Atomic<FlashStatus>>,
        progress: Arc<Vec<Atomic<u64>>>,
        finished: Arc<Vec<Atomic<bool>>>,
    ) -> FlashRequest {
        FlashRequest { source, destinations, status, progress, finished }
    }

    pub fn write(self) -> io::Result<Vec<io::Result<()>>> {
        let status = &self.status;
        let progress = &self.progress;
        let finished = &self.finished;
        let destinations = self.destinations;
        let mut source = self.source;

        status.store(FlashStatus::Active, Ordering::SeqCst);

        // Unmount the devices beforehand.
        if let Ok(mounts) = MountList::new() {
            for file in &destinations {
                for mount in mounts.source_starts_with(file) {
                    let _ = unmount(&mount.dest, UnmountFlags::DETACH);
                }
            }
        }

        // Then open them for writing to.
        let mut files = Vec::new();
        for file in &destinations {
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_SYNC)
                .open(file)?;
            files.push(file);
        }

        let mut errors = (0..files.len()).map(|_| Ok(())).collect::<Vec<_>>();

        // How many bytes to write at a given time.
        let mut bucket = [0u8; 8 * 1024 * 1024];

        let result = BusWriter::new(
            &mut source,
            &mut files,
            |event| match event {
                BusWriterMessage::Written { id, bytes_written } => {
                    progress[id].store(bytes_written, Ordering::SeqCst);
                }
                BusWriterMessage::Completed { id } => {
                    finished[id].store(true, Ordering::SeqCst);
                }
                BusWriterMessage::Errored { id, why } => {
                    errors[id] = Err(why);
                }
            },
            // Write will exit early when this is true
            || FlashStatus::Killing == status.load(Ordering::SeqCst),
        )
        .with_bucket(&mut bucket[..])
        .write()
        .map(|_| errors);

        for atomic in finished.iter() {
            atomic.store(true, Ordering::SeqCst);
        }

        status.store(FlashStatus::Inactive, Ordering::SeqCst);

        result
    }
}
