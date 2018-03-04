extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate libc;

mod mount;

pub use self::mount::Mount;

use std::cmp;
use std::fs::{canonicalize, read_dir, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

const BUFFER_SIZE: usize = 4 * 1024 * 1024;

#[derive(Debug, Fail)]
pub enum ImageError {
    #[fail(display = "image could not be opened: {}", why)]
    Open { why: io::Error },
    #[fail(display = "unable to get image metadata: {}", why)]
    Metadata { why: io::Error },
    #[fail(display = "image was not a file")]
    NotAFile,
    #[fail(display = "unable to read image: {}", why)]
    ReadError { why: io::Error },
    #[fail(display = "reached EOF prematurely")]
    Eof,
}

/// A simple wrapper around a `File` that ensures that the file is a file, and
/// obtains the file's size ahead of time.
pub struct Image {
    file: File,
    size: u64,
}

impl Image {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Image, ImageError> {
        File::open(path.as_ref())
            .map_err(|why| ImageError::Open { why })
            .and_then(|file| {
                file.metadata()
                    .map_err(|why| ImageError::Metadata { why })
                    .and_then(|metadata| {
                        if metadata.file_type().is_file() {
                            Err(ImageError::NotAFile)
                        } else {
                            Ok(Image {
                                file,
                                size: metadata.len(),
                            })
                        }
                    })
            })
    }

    pub fn get_size(&self) -> u64 { self.size }

    pub fn read_image<P: FnMut(u64)>(
        &mut self,
        mut progress_callback: P,
    ) -> Result<Vec<u8>, ImageError> {
        let mut data = vec![0; self.size as usize];

        let mut total = 0;
        while total < data.len() {
            let end = cmp::min(data.len(), total + BUFFER_SIZE);
            let count = self.file
                .read(&mut data[total..end])
                .map_err(|why| ImageError::ReadError { why })?;

            if count == 0 {
                return Err(ImageError::Eof);
            }
            total += count;
            progress_callback(total as u64);
        }

        Ok(data)
    }
}

#[derive(Debug, Fail)]
pub enum DiskError {
    #[fail(display = "unable to open directory at '{}': {}", dir, why)]
    Directory { dir: &'static str, why: io::Error },
    #[fail(display = "unable to read directory entry at '{:?}': invalid UTF-8", dir)]
    UTF8 { dir: PathBuf },
}

fn is_usb(filename: &str) -> bool {
    filename.starts_with("pci-") && filename.contains("-usb-") && filename.ends_with("-0:0:0:0")
}

const DISK_DIR: &str = "/dev/disk/by-path/";

/// Stores all discovered USB disk paths into the supplied `disks` vector.
pub fn get_disk_args(disks: &mut Vec<String>) -> Result<(), DiskError> {
    let readdir = read_dir(DISK_DIR).map_err(|why| DiskError::Directory { dir: DISK_DIR, why })?;

    for entry_res in readdir {
        let entry = entry_res.map_err(|why| DiskError::Directory { dir: DISK_DIR, why })?;
        let path = entry.path();
        if let Some(filename_os) = path.file_name() {
            if is_usb(filename_os.to_str().unwrap()) {
                disks.push(
                    path.to_str()
                        .ok_or_else(|| DiskError::UTF8 { dir: path.clone() })?
                        .into(),
                );
            }
        }
    }

    Ok(())
}

pub fn disks_from_args(
    disk_args: Vec<String>,
    mounts: &[Mount],
    unmount: bool,
) -> Result<Vec<(String, File)>, String> {
    let mut disks = Vec::new();

    for disk_arg in disk_args {
        let canonical_path = match canonicalize(&disk_arg) {
            Ok(p) => p,
            Err(err) => {
                return Err(format!("error finding disk '{}': {}", disk_arg, err));
            }
        };

        for mount in mounts.iter() {
            if mount
                .source
                .as_bytes()
                .starts_with(canonical_path.as_os_str().as_bytes())
            {
                if unmount {
                    println!(
                        "unmounting '{}': {:?} is mounted at {:?}",
                        disk_arg, mount.source, mount.dest
                    );

                    match Command::new("umount").arg(&mount.source).status() {
                        Ok(status) => {
                            if !status.success() {
                                return Err(format!(
                                    "failed to unmount {:?}: exit status {}",
                                    mount.source, status
                                ));
                            }
                        }
                        Err(err) => {
                            return Err(format!("failed to unmount {:?}: {}", mount.source, err));
                        }
                    }
                } else {
                    return Err(format!(
                        "error using disk '{}': {:?} already mounted at {:?}",
                        disk_arg, mount.source, mount.dest
                    ));
                }
            }
        }

        match canonical_path.metadata() {
            Ok(metadata) => {
                if !metadata.file_type().is_block_device() {
                    return Err(format!(
                        "error using disk '{}': not a block device",
                        disk_arg
                    ));
                }
            }
            Err(err) => {
                return Err(format!(
                    "error getting metadata of disk '{}': {}",
                    disk_arg, err
                ));
            }
        }

        let disk_res = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open(&canonical_path);

        let disk = match disk_res {
            Ok(disk) => disk,
            Err(err) => {
                return Err(format!("error opening disk '{}': {}", disk_arg, err));
            }
        };

        disks.push((disk_arg, disk));
    }

    Ok(disks)
}

/// Writes an image to the specified disk.
pub fn write_to_disk<M, F, S>(
    mut message: M,
    finish: F,
    mut set: S,
    mut disk: File,
    disk_path: String,
    image_size: u64,
    image_data: &[u8],
    check: bool,
) -> Result<(), String>
where
    M: FnMut(&str),
    F: Fn(),
    S: FnMut(u64),
{
    let mut total = 0;
    while total < image_data.len() {
        let end = cmp::min(image_size as usize, total + BUFFER_SIZE);
        let count = match disk.write(&image_data[total..end]) {
            Ok(count) => count,
            Err(err) => {
                message(&format!("! {}: ", disk_path));
                finish();

                return Err(format!("error writing disk '{}': {}", disk_path, err));
            }
        };
        if count == 0 {
            message(&format!("! {}: ", disk_path));
            finish();

            return Err(format!("error writing disk '{}': reached EOF", disk_path));
        }
        total += count;
        set(total as u64);
    }

    if let Err(err) = disk.flush() {
        message(&format!("! {}: ", disk_path));
        finish();

        return Err(format!("error flushing disk '{}': {}", disk_path, err));
    }

    if check {
        match disk.seek(SeekFrom::Start(0)) {
            Ok(0) => (),
            Ok(invalid) => {
                message(&format!("! {}: ", disk_path));
                finish();

                return Err(format!(
                    "error seeking disk '{}': seeked to {} instead of 0",
                    disk_path, invalid
                ));
            }
            Err(err) => {
                message(&format!("! {}: ", disk_path));
                finish();

                return Err(format!("error seeking disk '{}': {}", disk_path, err));
            }
        }

        message(&format!("V {}: ", disk_path));
        set(0);
        total = 0;

        let mut buf = vec![0; BUFFER_SIZE];
        while total < image_data.len() {
            let end = cmp::min(image_size as usize, total + BUFFER_SIZE);
            let count = match disk.read(&mut buf[..end - total]) {
                Ok(count) => count,
                Err(err) => {
                    message(&format!("! {}: ", disk_path));
                    finish();

                    return Err(format!("error verifying disk '{}': {}", disk_path, err));
                }
            };

            if count == 0 {
                message(&format!("! {}: ", disk_path));
                finish();

                return Err(format!("error verifying disk '{}': reached EOF", disk_path));
            }

            if buf[..count] != image_data[total..total + count] {
                message(&format!("! {}: ", disk_path));
                finish();

                return Err(format!(
                    "error verifying disk '{}': mismatch at {}:{}",
                    disk_path,
                    total,
                    total + count
                ));
            }

            total += count;
            set(total as u64);
        }
    }

    finish();

    Ok(())
}
