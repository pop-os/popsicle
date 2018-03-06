use digest::{Digest, Input};
use gtk::{self, EntryExt};
use md5::Md5;
use sha3::Sha3_256;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

fn md5_hasher(data: &[u8]) -> String {
    let mut hasher = Md5::default();
    hasher.process(data);
    format!("{:x}", hasher.result())
}

fn sha256_hasher(data: &[u8]) -> String {
    let mut hasher = Sha3_256::default();
    hasher.process(data);
    format!("{:x}", hasher.result())
}

pub(crate) fn set(entry: &gtk::Entry, hash: &str, data: &[u8]) {
    let hash = match hash {
        "Type" => return,
        "SHA256" => sha256_hasher(data),
        "MD5" => md5_hasher(data),
        _ => unimplemented!(),
    };

    entry.get_buffer().set_text(&hash);
}
