use digest::{Digest, Input};
use md5::Md5;
use sha3::Sha3_256;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub(crate) fn md5_hasher(path: &Path) -> io::Result<String> {
    let mut hasher = Md5::default();
    let mut buffer = [0; 16 * 1024];
    File::open(path).and_then(|mut file| {
        let mut read = file.read(&mut buffer)?;
        while read != 0 {
            hasher.process(&buffer[..read]);
            read = file.read(&mut buffer)?;
        }
        Ok(format!("{:x}", hasher.result()))
    })
}

pub(crate) fn sha256_hasher(path: &Path) -> io::Result<String> {
    let mut hasher = Sha3_256::default();
    let mut buffer = [0; 16 * 1024];
    File::open(path).and_then(|mut file| {
        let mut read = file.read(&mut buffer)?;
        while read != 0 {
            hasher.process(&buffer[..read]);
            read = file.read(&mut buffer)?;
        }
        Ok(format!("{:x}", hasher.result()))
    })
}
