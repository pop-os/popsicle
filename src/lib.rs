extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate libc;

mod mount;

pub use self::mount::Mount;

use std::cmp;
use std::ffi::OsString;
use std::fs::{canonicalize, read_dir, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const BUFFER_SIZE: usize = 4 * 1024 * 1024;

#[derive(Debug, Fail)]
#[cfg_attr(rustfmt, rustfmt_skip)]
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
    path: PathBuf,
    file: File,
    size: u64,
}

impl Image {
    /// Opens the file and obtains the size from the metadata, then returns an
    /// `Image` structure that contains the opened file and its file size.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Image, ImageError> {
        let path = path.as_ref();
        File::open(path)
            .map_err(|why| ImageError::Open { why })
            .and_then(|file| {
                file.metadata()
                    .map_err(|why| ImageError::Metadata { why })
                    .and_then(|metadata| {
                        if metadata.file_type().is_file() {
                            Ok(Image {
                                path: path.to_path_buf(),
                                file,
                                size: metadata.len(),
                            })
                        } else {
                            Err(ImageError::NotAFile)
                        }
                    })
            })
    }

    pub fn get_path(&self) -> &Path { &self.path }

    /// Returns the size of the file, in bytes.
    pub fn get_size(&self) -> u64 { self.size }

    /// Reads the image into a vector, and reports progress to a callback.
    pub fn read<P: FnMut(u64)>(
        &mut self,
        data: &mut Vec<u8>,
        mut progress_callback: P,
    ) -> Result<(), ImageError> {
        if data.capacity() < self.size as usize {
            let capacity = self.size as usize - data.capacity();
            data.reserve_exact(capacity);
            data.append(&mut vec![0; capacity])
        } else if data.capacity() > self.size as usize {
            data.truncate(self.size as usize);
            data.shrink_to_fit();
        }

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

        Ok(())
    }
}

#[derive(Debug, Fail)]
#[cfg_attr(rustfmt, rustfmt_skip)]
pub enum DiskError {
    #[fail(display = "unable to open directory at '{}': {}", dir, why)]
    Directory { dir: &'static str, why: io::Error },
    #[fail(display = "unable to read directory entry at '{:?}': invalid UTF-8", dir)]
    UTF8 { dir: PathBuf },
    #[fail(display = "unable to find disk '{}': {}", disk, why)]
    NoDisk { disk: String, why: io::Error },
    #[fail(display = "failed to unmount {:?}: exit status {}", path, status)]
    UnmountStatus { path: OsString, status: ExitStatus },
    #[fail(display = "failed to unmount {:?}: {}", path, why)]
    UnmountCommand { path: OsString, why: io::Error },
    #[fail(display = "error using disk '{}': {:?} already mounted at {:?}", arg, source, dest)]
    AlreadyMounted { arg: String, source: OsString, dest: OsString },
    #[fail(display = "'{}' is not a block device", arg)]
    NotABlock { arg: String },
    #[fail(display = "unable to get metadata of disk '{}': {}", arg, why)]
    Metadata { arg: String, why: io::Error },
    #[fail(display = "unable to open disk '{}': {}", disk, why)]
    Open { disk: String, why: io::Error },
    #[fail(display = "error writing disk '{}': {}", disk, why)]
    Write { disk: String, why: io::Error },
    #[fail(display = "error writing disk '{}': reached EOF", disk)]
    WriteEOF { disk: String },
    #[fail(display = "unable to flush disk '{}': {}", disk, why)]
    Flush { disk: String, why: io::Error },
    #[fail(display = "error seeking disk '{}': seeked to {} instead of 0", disk, invalid)]
    SeekInvalid { disk: String, invalid: u64 },
    #[fail(display = "error seeking disk '{}': {}", disk, why)]
    Seek { disk: String, why: io::Error },
    #[fail(display = "error verifying disk '{}': {}", disk, why)]
    Verify { disk: String, why: io::Error },
    #[fail(display = "error verifying disk '{}': reached EOF", disk)]
    VerifyEOF { disk: String },
    #[fail(display = "error verifying disk '{}': mismatch at {}:{}", disk, x, y)]
    VerifyMismatch { disk: String, x: usize, y: usize },
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

pub fn disks_from_args<D: Iterator<Item = String>>(
    disk_args: D,
    mounts: &[Mount],
    unmount: bool,
) -> Result<Vec<(String, File)>, DiskError> {
    let mut disks = Vec::new();

    for disk_arg in disk_args {
        let canonical_path = canonicalize(&disk_arg).map_err(|why| DiskError::NoDisk {
            disk: disk_arg.clone(),
            why,
        })?;

        for mount in mounts.iter() {
            if mount
                .source
                .as_bytes()
                .starts_with(canonical_path.as_os_str().as_bytes())
            {
                if unmount {
                    eprintln!(
                        "unmounting '{}': {:?} is mounted at {:?}",
                        disk_arg, mount.source, mount.dest
                    );

                    Command::new("umount")
                        .arg(&mount.source)
                        .status()
                        .map_err(|why| DiskError::UnmountCommand {
                            path: mount.source.clone(),
                            why,
                        })
                        .and_then(|status| {
                            if !status.success() {
                                Err(DiskError::UnmountStatus {
                                    path: mount.source.clone(),
                                    status,
                                })
                            } else {
                                Ok(())
                            }
                        })?;
                } else {
                    return Err(DiskError::AlreadyMounted {
                        arg:    disk_arg.clone(),
                        source: mount.source.clone(),
                        dest:   mount.dest.clone(),
                    });
                }
            }
        }

        let metadata = canonical_path
            .metadata()
            .map_err(|why| DiskError::Metadata {
                arg: disk_arg.clone(),
                why,
            })?;

        if !metadata.file_type().is_block_device() {
            return Err(DiskError::NotABlock {
                arg: disk_arg.clone(),
            });
        }

        let disk = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open(&canonical_path)
            .map_err(|why| DiskError::Open {
                disk: disk_arg.clone(),
                why,
            })?;

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
) -> Result<(), DiskError>
where
    M: FnMut(&str),
    F: Fn(),
    S: FnMut(u64),
{
    let mut total = 0;
    while total < image_data.len() {
        let end = cmp::min(image_size as usize, total + BUFFER_SIZE);
        let count = disk.write(&image_data[total..end]).map_err(|why| {
            message(&format!("! {}: ", disk_path));
            finish();
            DiskError::Write {
                disk: disk_path.clone(),
                why,
            }
        })?;

        if count == 0 {
            message(&format!("! {}: ", disk_path));
            finish();

            return Err(DiskError::WriteEOF {
                disk: disk_path.clone(),
            });
        }
        total += count;
        set(total as u64);
    }

    disk.flush().map_err(|why| {
        message(&format!("! {}: ", disk_path));
        finish();

        DiskError::Flush {
            disk: disk_path.clone(),
            why,
        }
    })?;

    if check {
        match disk.seek(SeekFrom::Start(0)) {
            Ok(0) => (),
            Ok(invalid) => {
                message(&format!("! {}: ", disk_path));
                finish();

                return Err(DiskError::SeekInvalid {
                    disk: disk_path.clone(),
                    invalid,
                });
            }
            Err(why) => {
                message(&format!("! {}: ", disk_path));
                finish();

                return Err(DiskError::Seek {
                    disk: disk_path.clone(),
                    why,
                });
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
                Err(why) => {
                    message(&format!("! {}: ", disk_path));
                    finish();

                    return Err(DiskError::Verify {
                        disk: disk_path.clone(),
                        why,
                    });
                }
            };

            if count == 0 {
                message(&format!("! {}: ", disk_path));
                finish();

                return Err(DiskError::VerifyEOF {
                    disk: disk_path.clone(),
                });
            }

            if buf[..count] != image_data[total..total + count] {
                message(&format!("! {}: ", disk_path));
                finish();

                return Err(DiskError::VerifyMismatch {
                    disk: disk_path.clone(),
                    x:    total,
                    y:    total + count,
                });
            }

            total += count;
            set(total as u64);
        }
    }

    finish();

    Ok(())
}
