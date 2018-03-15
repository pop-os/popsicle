use std::default::Default;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

fn read_file(path: &Path) -> String {
    File::open(path)
        .and_then(|mut file| {
            let mut string = String::new();
            file.read_to_string(&mut string)
                .map(|_| string.trim().to_owned())
        })
        .unwrap_or(String::default())
}

pub struct BlockDevice {
    path: PathBuf,
}

impl BlockDevice {
    pub fn new(path: &Path) -> Option<BlockDevice> {
        path.file_name().and_then(|file_name| {
            let path = PathBuf::from("/sys/class/block/").join(file_name);
            if path.exists() {
                Some(BlockDevice { path })
            } else {
                None
            }
        })
    }

    pub fn vendor(&self) -> String { read_file(&self.path.join("device/vendor")) }

    pub fn model(&self) -> String { read_file(&self.path.join("device/model")) }

    pub fn label(&self) -> String {
        let model = self.model();
        let vendor = self.vendor();
        if vendor.is_empty() {
            model.replace("_", " ")
        } else {
            [&vendor, " ", &model].concat().replace("_", " ")
        }
    }
}
