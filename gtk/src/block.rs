use std::default::Default;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

const SLEEP_AFTER_FAIL: u64 = 500;
const ATTEMPTS: u64 = 5_000 / SLEEP_AFTER_FAIL;

fn read_file(path: &Path) -> String {
    let output = File::open(path)
        .and_then(|mut file| {
            let mut string = String::new();
            file.read_to_string(&mut string)
                .map(|_| string.trim().to_owned())
        })
        .unwrap_or_else(|_| String::default());
    output
}

pub struct BlockDevice {
    path: PathBuf,
    pub sectors: u64
}

impl BlockDevice {
    pub fn new(path: &Path) -> Option<BlockDevice> {
        path.file_name().and_then(|file_name| {
            let path = PathBuf::from("/sys/class/block/").join(file_name);
            if path.exists() {
                let sectors = Self::sectors(&path);
                Some(BlockDevice { path, sectors })
            } else {
                None
            }
        })
    }

    fn sectors(path: &Path) -> u64 {
        let get_sectors = || read_file(&path.join("size")).parse::<u64>().unwrap_or(0);
        let (mut result, mut attempts) = (get_sectors(), 0);

        while result == 0 {
            result = get_sectors();
            sleep(Duration::from_millis(SLEEP_AFTER_FAIL));
            if attempts == ATTEMPTS {
                break
            }
            attempts += 1;
        }

        result
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
