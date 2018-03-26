use popsicle::Image;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;

/// A structure for buffering disk images in the background.
pub struct BufferingData {
    /// Stores the path of the image, and the image contents stored in memory.
    pub data: Mutex<(PathBuf, Vec<u8>)>,
    /// This field will determine if the `data` field is ready to be used.
    pub state: AtomicUsize,
}

impl BufferingData {
    pub fn new() -> BufferingData {
        BufferingData {
            data:  Mutex::new((PathBuf::new(), Vec::new())),
            state: 0.into(),
        }
    }
}

/// An event loop that is meant to be run in a background thread, receiving image paths
/// to load, and buffering those images into the application's shared `BufferingData`
/// field.
pub fn image_load_event_loop(path_receiver: Receiver<PathBuf>, buffer: &BufferingData) {
    while let Ok(path) = path_receiver.recv() {
        buffer.state.store(0b1, Ordering::SeqCst);
        let (ref mut name, ref mut data) = *buffer
            .data
            .lock()
            .expect("failed to unlock image buffer mutex");
        match load_image(&path, data) {
            Ok(_) => {
                *name = path;
                buffer.state.store(0b10, Ordering::SeqCst);
            }
            Err(why) => {
                eprintln!("popsicle-gtk: image loading error: {}", why);
                buffer.state.store(0b100, Ordering::SeqCst);
            }
        }
    }
}

pub fn load_image<P: AsRef<Path>>(path: P, data: &mut Vec<u8>) -> io::Result<()> {
    let path = path.as_ref();
    let mut new_image = match Image::new(path) {
        Ok(image) => image,
        Err(why) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("unable to open image: {}", why),
            ));
        }
    };

    if let Err(why) = new_image.read(data, |_| ()) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("unable to open image: {}", why),
        ));
    }

    Ok(())
}
