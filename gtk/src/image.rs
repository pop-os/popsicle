use super::ui::BufferingData;
use popsicle::Image;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;

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
