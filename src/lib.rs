#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate thiserror;

pub extern crate mnt;

pub mod codec;

mod task;

pub use self::task::{Progress, Task};

use anyhow::Context;
use as_result::MapResult;
use async_std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use futures::{executor, prelude::*};
use mnt::MountEntry;
use std::{
    io,
    os::unix::{ffi::OsStrExt, fs::FileTypeExt},
    process::Command,
};
use usb_disk_probe::stream::UsbDiskProbe;

#[derive(Debug, Error)]
#[rustfmt::skip]
pub enum ImageError {
    #[error("image could not be opened: {}", why)]
    Open { why: io::Error },
    #[error("unable to get image metadata: {}", why)]
    Metadata { why: io::Error },
    #[error("image was not a file")]
    NotAFile,
    #[error("unable to read image: {}", why)]
    ReadError { why: io::Error },
    #[error("reached EOF prematurely")]
    Eof,
}

#[derive(Debug, Error)]
#[rustfmt::skip]
pub enum DiskError {
    #[error("failed to fetch devices from USB device stream: {}", _0)]
    DeviceStream(anyhow::Error),
    #[error("unable to open directory at '{}': {}", dir, why)]
    Directory { dir: &'static str, why: io::Error },
    #[error("writing to the device was killed")]
    Killed,
    #[error("unable to read directory entry at '{}': invalid UTF-8", dir.display())]
    UTF8 { dir: Box<Path> },
    #[error("unable to find disk '{}': {}", disk.display(), why)]
    NoDisk { disk: Box<Path>, why: io::Error },
    #[error("failed to unmount {}: {}", path.display(), why)]
    UnmountCommand { path: Box<Path>, why: io::Error },
    #[error("error using disk '{}': {} already mounted at {}", arg.display(), source_.display(), dest.display())]
    AlreadyMounted { arg: Box<Path>, source_: Box<Path>, dest: Box<Path> },
    #[error("'{}' is not a block device", arg.display())]
    NotABlock { arg: Box<Path> },
    #[error("unable to get metadata of disk '{}': {}", arg.display(), why)]
    Metadata { arg: Box<Path>, why: io::Error },
    #[error("unable to open disk '{}': {}", disk.display(), why)]
    Open { disk: Box<Path>, why: io::Error },
    #[error("error writing disk '{}': {}", disk.display(), why)]
    Write { disk: Box<Path>, why: io::Error },
    #[error("error writing disk '{}': reached EOF", disk.display())]
    WriteEOF { disk: Box<Path> },
    #[error("unable to flush disk '{}': {}", disk.display(), why)]
    Flush { disk: Box<Path>, why: io::Error },
    #[error("error seeking disk '{}': seeked to {} instead of 0", disk.display(), invalid)]
    SeekInvalid { disk: Box<Path>, invalid: u64 },
    #[error("error seeking disk '{}': {}", disk.display(), why)]
    Seek { disk: Box<Path>, why: io::Error },
    #[error("error verifying disk '{}': {}", disk.display(), why)]
    Verify { disk: Box<Path>, why: io::Error },
    #[error("error verifying disk '{}': reached EOF", disk.display())]
    VerifyEOF { disk: Box<Path> },
    #[error("error verifying disk '{}': mismatch at {}:{}", disk.display(), x, y)]
    VerifyMismatch { disk: Box<Path>, x: usize, y: usize },
}

pub async fn usb_disk_devices(disks: &mut Vec<Box<Path>>) -> anyhow::Result<()> {
    let mut stream = UsbDiskProbe::new().await.context("failed to create USB disk probe")?;

    while let Some(device_result) = stream.next().await {
        match device_result {
            Ok(disk) => disks.push(PathBuf::from(&*disk).into_boxed_path()),
            Err(why) => {
                eprintln!("failed to reach device path: {}", why);
            }
        }
    }

    Ok(())
}

/// Stores all discovered USB disk paths into the supplied `disks` vector.
pub fn get_disk_args(disks: &mut Vec<Box<Path>>) -> Result<(), DiskError> {
    executor::block_on(
        async move { usb_disk_devices(disks).await.map_err(DiskError::DeviceStream) },
    )
}

pub async fn disks_from_args<D: Iterator<Item = Box<Path>>>(
    disk_args: D,
    mounts: &[MountEntry],
    unmount: bool,
) -> Result<Vec<(Box<Path>, File)>, DiskError> {
    let mut disks = Vec::new();

    for disk_arg in disk_args {
        let canonical_path = fs::canonicalize(&disk_arg)
            .await
            .map_err(|why| DiskError::NoDisk { disk: disk_arg.clone(), why })?;

        for mount in mounts {
            if mount.spec.as_bytes().starts_with(canonical_path.as_os_str().as_bytes()) {
                if unmount {
                    eprintln!(
                        "unmounting '{}': {:?} is mounted at {:?}",
                        disk_arg.display(),
                        mount.spec,
                        mount.file
                    );

                    Command::new("umount").arg(&mount.spec).status().map_result().map_err(
                        |why| DiskError::UnmountCommand {
                            path: PathBuf::from(mount.spec.clone()).into_boxed_path(),
                            why,
                        },
                    )?;
                } else {
                    return Err(DiskError::AlreadyMounted {
                        arg: disk_arg.clone(),
                        source_: PathBuf::from(mount.spec.clone()).into_boxed_path(),
                        dest: PathBuf::from(mount.file.clone()).into_boxed_path(),
                    });
                }
            }
        }

        let metadata = canonical_path
            .metadata()
            .await
            .map_err(|why| DiskError::Metadata { arg: disk_arg.clone(), why })?;

        if !metadata.file_type().is_block_device() {
            return Err(DiskError::NotABlock { arg: disk_arg.clone() });
        }

        let disk = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open(&canonical_path)
            .await
            .map_err(|why| DiskError::Open { disk: disk_arg.clone(), why })?;

        disks.push((canonical_path.into_boxed_path(), disk));
    }

    Ok(disks)
}
