use sysfs_class::{Block, SysClass};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct BlockDevice {
    block: Block,
    pub path: PathBuf,
    pub vendor: String,
    pub model: String,
    pub sectors: u64,
    pub sector_size: u64
}

impl BlockDevice {
    pub fn new_from<P: AsRef<Path>>(path: P) -> io::Result<BlockDevice> {
        let path = path.as_ref();

        let file_name = path.file_name().ok_or_else(|| io::Error::new(
            io::ErrorKind::InvalidInput,
            "BlockDevice::new_from path does not have a file name"
        ))?;

        let file_name = file_name.to_str().ok_or_else(|| io::Error::new(
            io::ErrorKind::InvalidData,
            "BlockDevice::new_from path is not UTF-8"
        ))?;

        let block = Block::new(file_name)?;

        let device = BlockDevice {
            path: path.to_path_buf(),
            vendor: block.device_vendor()?.trim().to_owned(),
            model: block.device_model()?.trim().to_owned(),
            sectors: block.size()?,
            sector_size: block.queue_hw_sector_size()?,
            block,
        };

        Ok(device)
    }

    pub fn recheck_size(&mut self) {
        if self.sector_size == 0 {
            if let Ok(size) = self.block.queue_hw_sector_size() {
                self.sector_size = size;
            }
        }

        if let Ok(size) = self.block.size() {
            self.sectors = size;
        }
    }

    pub fn size_in_bytes(&self) -> u64 {
        self.sectors * self.sector_size
    }

    pub fn label(&self) -> String {
        format!("{} {} ({})", self.vendor, self.model, self.path.display())
    }
}
