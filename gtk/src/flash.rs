use bus_writer::{BusWriter, BusWriterMessage};
use std::io;
use std::fs::File;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use app::{KILL, CANCELLED};

pub struct FlashRequest {
    source:       File,
    destinations: Vec<File>,
    status:       Arc<AtomicUsize>,
    progress:     Arc<Vec<AtomicUsize>>,
    finished:     Arc<Vec<AtomicUsize>>,
}

impl FlashRequest {
    pub fn new(
        source: File,
        destinations: Vec<File>,
        status: Arc<AtomicUsize>,
        progress: Arc<Vec<AtomicUsize>>,
        finished: Arc<Vec<AtomicUsize>>,
    ) -> FlashRequest {
        FlashRequest {
            source,
            destinations,
            status,
            progress,
            finished,
        }
    }

    pub fn write(self) -> io::Result<Vec<io::Result<()>>> {
        let status = self.status;
        let progress = self.progress;
        let finished = self.finished;
        let mut destinations = self.destinations;
        let mut source = self.source;

        let mut errors = (0..destinations.len())
            .map(|_| Ok(()))
            .collect::<Vec<_>>();

        let mut bucket = vec![0u8; 16 * 1024 * 1024];

        let result = BusWriter::new(
            &mut source,
            &mut destinations,
            |event| match event {
                BusWriterMessage::Written { id, bytes_written } => {
                    progress[id].store(bytes_written as usize, Ordering::SeqCst);
                }
                BusWriterMessage::Completed { id } => {
                    finished[id].store(1, Ordering::SeqCst);
                }
                BusWriterMessage::Errored { id, why } => {
                    errors[id] = Err(why);
                }
            },
            // Write will exit early when this is true
            || KILL == status.load(Ordering::SeqCst),
        ).with_bucket(&mut bucket).write().map(|_| errors);

        if status.load(Ordering::SeqCst) == KILL {
            status.store(CANCELLED, Ordering::SeqCst);
        }

        result
    }
}
