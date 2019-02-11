use digest::Digest;
use hex_view::HexView;

use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;
use std::io::{self, Read};
use std::fs::File;
use std::path::Path;
use std::os::unix::ffi::OsStrExt;

fn hash_id(image: &Path, kind: &'static str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(image.as_os_str().as_bytes());
    hasher.write(kind.as_bytes());
    hasher.finish()
}

pub(crate) fn hasher<H: Digest>(image: &Path) -> io::Result<String> {
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
