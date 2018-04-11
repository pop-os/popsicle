use digest::{Digest, Input};
use image::{self, BufferingData};
use md5::Md5;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

pub(crate) const SLEEPING: usize = 0;
pub(crate) const PROCESSING: usize = 1;
pub(crate) const COMPLETED: usize = 2;
pub(crate) const INTERRUPT: usize = 3;

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
            state: SLEEPING.into(),
        }
    }

    /// If it's true, it's not sleeping.
    pub(crate) fn is_busy(&self) -> bool { self.state.load(Ordering::SeqCst) != SLEEPING }

    pub(crate) fn is_ready(&self) -> bool { self.state.load(Ordering::SeqCst) == COMPLETED }

    /// Requests for a new hash to be generated, if it hasn't been generated already.
    pub(crate) fn request(&self, requested: &'static str) {
        let mut data = self.data.lock().unwrap();
        data.requested = requested;
        self.state.store(INTERRUPT, Ordering::SeqCst);
    }

    /// Obtains the current hash stored within the state, if it exists.
    pub(crate) fn obtain(&self) -> String {
        let data = self.data.lock().unwrap();
        let output = match data.store.get(data.requested) {
            Some(hash) => hash.clone(),
            None => "failed".into(),
        };
        self.state.store(SLEEPING, Ordering::SeqCst);
        output
    }
}

pub(crate) struct HashData {
    requested: &'static str,
    store:     BTreeMap<&'static str, String>,
}

impl HashData {
    fn new() -> HashData {
        HashData {
            requested: "",
            store:     BTreeMap::new(),
        }
    }
}

pub(crate) fn event_loop(buffer: &BufferingData, hash: &HashState) {
    let mut last_image = PathBuf::new();

    loop {
        if hash.state.load(Ordering::SeqCst) == INTERRUPT {
            hash.state.store(PROCESSING, Ordering::SeqCst);
            let mut hash_data = hash.data.lock().unwrap();
            while buffer.state.load(Ordering::SeqCst) != image::COMPLETED {
                thread::sleep(Duration::from_millis(16));
            }

            let buffer_data = buffer.data.lock().unwrap();
            let current_image = &buffer_data.0;
            let image_data = &buffer_data.1;
            let requested = hash_data.requested;
            let same_image = &last_image != current_image.as_path();

            if !same_image || !hash_data.store.contains_key(requested) {
                if !same_image {
                    hash_data.store.clear();
                }

                hash_data.store.insert(
                    requested,
                    match requested {
                        "MD5" => md5_hasher(&image_data),
                        "SHA256" => sha256_hasher(&image_data),
                        _ => "Critical Error".into(),
                    },
                );
            }

            if !same_image {
                last_image = current_image.to_path_buf();
            }

            hash.state.store(COMPLETED, Ordering::SeqCst);
        }
        thread::sleep(Duration::from_millis(16));
    }
}

pub(crate) fn md5_hasher(data: &[u8]) -> String {
    let mut hasher = Md5::default();
    hasher.process(data);
    format!("{:x}", hasher.result())
}

pub(crate) fn sha256_hasher(data: &[u8]) -> String {
    let mut hasher = Sha256::default();
    hasher.process(data);
    format!("{:x}", hasher.result())
}
