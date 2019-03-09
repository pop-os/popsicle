#[macro_use]
extern crate err_derive;
pub extern crate mnt;

use mnt::MountEntry;

use std::fs::{canonicalize, read_dir, File, OpenOptions};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

#[derive(Debug, Error)]
#[cfg_attr(rustfmt, rustfmt_skip)]
pub enum ImageError {
    #[error(display = "image could not be opened: {}", why)]
    Open { why: io::Error },
    #[error(display = "unable to get image metadata: {}", why)]
    Metadata { why: io::Error },
    #[error(display = "image was not a file")]
    NotAFile,
    #[error(display = "unable to read image: {}", why)]
    ReadError { why: io::Error },
    #[error(display = "reached EOF prematurely")]
    Eof,
}

#[derive(Debug, Error)]
#[cfg_attr(rustfmt, rustfmt_skip)]
pub enum DiskError {
    #[error(display = "unable to open directory at '{}': {}", dir, why)]
    Directory { dir: &'static str, why: io::Error },
    #[error(display = "writing to the device was killed")]
    Killed,
    #[error(display = "unable to read directory entry at '{:?}': invalid UTF-8", dir)]
    UTF8 { dir: PathBuf },
    #[error(display = "unable to find disk '{}': {}", disk, why)]
    NoDisk { disk: String, why: io::Error },
    #[error(display = "failed to unmount {:?}: exit status {}", path, status)]
    UnmountStatus { path: String, status: ExitStatus },
    #[error(display = "failed to unmount {:?}: {}", path, why)]
    UnmountCommand { path: String, why: io::Error },
    #[error(display = "error using disk '{}': {:?} already mounted at {:?}", arg, source, dest)]
    AlreadyMounted { arg: String, source: String, dest: PathBuf },
    #[error(display = "'{}' is not a block device", arg)]
    NotABlock { arg: String },
    #[error(display = "unable to get metadata of disk '{}': {}", arg, why)]
    Metadata { arg: String, why: io::Error },
    #[error(display = "unable to open disk '{}': {}", disk, why)]
    Open { disk: String, why: io::Error },
    #[error(display = "error writing disk '{}': {}", disk, why)]
    Write { disk: String, why: io::Error },
    #[error(display = "error writing disk '{}': reached EOF", disk)]
    WriteEOF { disk: String },
    #[error(display = "unable to flush disk '{}': {}", disk, why)]
    Flush { disk: String, why: io::Error },
    #[error(display = "error seeking disk '{}': seeked to {} instead of 0", disk, invalid)]
    SeekInvalid { disk: String, invalid: u64 },
    #[error(display = "error seeking disk '{}': {}", disk, why)]
    Seek { disk: String, why: io::Error },
    #[error(display = "error verifying disk '{}': {}", disk, why)]
    Verify { disk: String, why: io::Error },
    #[error(display = "error verifying disk '{}': reached EOF", disk)]
    VerifyEOF { disk: String },
    #[error(display = "error verifying disk '{}': mismatch at {}:{}", disk, x, y)]
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
    mounts: &[MountEntry],
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
                .spec
                .as_bytes()
                .starts_with(canonical_path.as_os_str().as_bytes())
            {
                if unmount {
                    eprintln!(
                        "unmounting '{}': {:?} is mounted at {:?}",
                        disk_arg, mount.spec, mount.file
                    );

                    Command::new("umount")
                        .arg(&mount.spec)
                        .status()
                        .map_err(|why| DiskError::UnmountCommand {
                            path: mount.spec.clone(),
                            why,
                        })
                        .and_then(|status| {
                            if !status.success() {
                                Err(DiskError::UnmountStatus {
                                    path: mount.spec.clone(),
                                    status,
                                })
                            } else {
                                Ok(())
                            }
                        })?;
                } else {
                    return Err(DiskError::AlreadyMounted {
                        arg:    disk_arg.clone(),
                        source: mount.spec.clone(),
                        dest:   mount.file.clone(),
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
