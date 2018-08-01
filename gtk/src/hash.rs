use digest::Digest;
use hex_view::HexView;
use md5::Md5;
use sha2::Sha256;
use std::hash::Hasher;
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::io::{self, Read};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use std::os::unix::ffi::OsStrExt;

pub(crate) const SLEEPING: usize = 0;
pub(crate) const PROCESSING: usize = 1;

pub(crate) struct HashState {
    // K = Variant, V = Hashed Value
    pub(crate) data: Mutex<HashData>,
    // Defines if a hash is being generated.
    pub(crate) state: AtomicUsize,
}

impl HashState {
    pub(crate) fn new() -> HashState {
        HashState {
            data:  Mutex::new(HashData::new()),
            state: AtomicUsize::new(SLEEPING)
        }
    }

    /// If it's true, it's not sleeping.
    fn is_busy(&self) -> bool { self.state.load(Ordering::SeqCst) != SLEEPING }

    /// Attempt to receive the hash for the given path.
    pub(crate) fn try_obtain(&self, requested: &Path, hash: &'static str) -> Option<String> {
        let data = self.data.lock().unwrap();
        if self.is_busy() {
            None
        } else {
            let value: String = match data.store.get(&hash_id(requested, hash)) {
                Some(ref hash) => hash.as_ref()
                    .map(|x| x.clone())
                    .unwrap_or_else(|why| format!("ERROR: {}", why)),
                None => return None,
            };

            Some(value)
        }
    }
}

pub(crate) struct HashData {
    store: BTreeMap<u64, io::Result<String>>,
}

impl HashData {
    fn new() -> HashData {
        HashData {
            store: BTreeMap::new(),
        }
    }
}

pub(crate) fn event_loop(
    images: &Receiver<(PathBuf, &'static str)>,
    hash: &HashState
) {
    let mut set = false;
    loop {
        while let Ok((image, type_of)) = images.try_recv() {
            eprintln!("received image to flash");
            set = true;
            let identifying_hash = hash_id(&image, type_of);

            hash.state.store(PROCESSING, Ordering::SeqCst);
            let mut hash_data = hash.data.lock().unwrap();
            if hash_data.store.get(&identifying_hash).map_or(true, |e| !e.is_ok()) {
                eprintln!("storing hash");
                hash_data.store.insert(
                    identifying_hash,
                    match type_of {
                        "MD5" => hasher::<Md5>(&image),
                        "SHA256" => hasher::<Sha256>(&image),
                        _ => Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("{} not supported", type_of)
                        )),
                    },
                );
                eprintln!("stored new image checksum");
            }
        }

        if set {
            hash.state.store(SLEEPING, Ordering::SeqCst);
            set = false;
        }

        thread::sleep(Duration::from_millis(16));
    }
}

fn hash_id(image: &Path, kind: &'static str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(image.as_os_str().as_bytes());
    hasher.write(kind.as_bytes());
    hasher.finish()
}

pub(crate) fn hasher<H: Digest>(image: &Path) -> io::Result<String> {
    eprintln!("hashing image");
    File::open(image).and_then(move |mut file| {
        let mut buffer = [0u8; 8 * 1024];
        let mut hasher = H::new();

        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 { break }
            hasher.input(&buffer[..read]);
        }

        Ok(format!("{:x}", HexView::from(hasher.result().as_slice())))
    })
}
