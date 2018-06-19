use std::fs::File;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use popsicle::{self, DiskError};

pub struct FlashRequest {
    disk: File,
    disk_path: String,
    image_len: u64,
    image_data: Arc<Vec<u8>>,
    progress: Arc<AtomicUsize>,
    finished: Arc<AtomicUsize>,
}

impl FlashRequest {
    pub fn new(
        disk: File,
        disk_path: String,
        image_len: u64,
        image_data: Arc<Vec<u8>>,
        progress: Arc<AtomicUsize>,
        finished: Arc<AtomicUsize>,
    ) -> FlashRequest {
        FlashRequest {
            disk,
            disk_path,
            image_len,
            image_data,
            progress,
            finished,
        }
    }

    pub fn write(self) -> Result<(), DiskError> {
        let disk = self.disk;
        let disk_path = self.disk_path;
        let progress = self.progress;
        let image_len = self.image_len;
        let image_data = self.image_data;

        let result = popsicle::write_to_disk(
            |_msg| (),
            || (),
            |value| progress.store(value as usize, Ordering::SeqCst),
            disk,
            disk_path,
            image_len,
            &image_data,
            false,
        );

        self.finished.store(1, Ordering::SeqCst);

        result
    }
}
