//! MUFF - Multiple USB File Flasher

extern crate clap;
extern crate libc;
extern crate pbr;

use clap::{App, Arg};
use pbr::{ProgressBar, MultiBar, Units};
use std::{cmp, process, thread};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::process::Command;
use std::sync::Arc;

use mount::Mount;

mod mount;

fn main() {
    let matches = App::new("Multiple USB File Flasher")
        .arg(
            Arg::with_name("IMAGE")
                .help("Input image file")
                .required(true)
        )
        .arg(
            Arg::with_name("DISKS")
                .help("Output disk devices")
                .multiple(true)
        )
        .arg(
            Arg::with_name("all")
                .help("Flash all detected USB drives")
                .short("a")
                .long("all")
        )
        .arg(
            Arg::with_name("check")
                .help("Check written image matches read image")
                .short("c")
                .long("check")
        )
        .arg(
            Arg::with_name("unmount")
                .help("Unmount mounted devices")
                .short("u")
                .long("unmount")
        )
        .arg(
            Arg::with_name("yes")
                .help("Continue without confirmation")
                .short("y")
                .long("yes")
        )
        .get_matches();

    let image_path = matches.value_of("IMAGE").expect("IMAGE not set");
    let mut image = match File::open(&image_path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("muff: error opening image '{}': {}", image_path, err);
            process::exit(1);
        }
    };

    let image_size = match image.metadata() {
        Ok(metadata) => {
            if ! metadata.file_type().is_file() {
                eprintln!("muff: error using image '{}': not a file", image_path);
                process::exit(1);
            }
            metadata.len()
        }
        Err(err) => {
            eprintln!("muff: error getting metadata of image '{}': {}", image_path, err);
            process::exit(1);
        }
    };

    let mut disk_args = vec![];
    if matches.is_present("all") {
        let disk_dir = "/dev/disk/by-id/";
        let readdir = match fs::read_dir(disk_dir) {
            Ok(readdir) => readdir,
            Err(err) => {
                eprintln!("muff: error opening directory '{}': {}", disk_dir, err);
                process::exit(1);
            }
        };
        for entry_res in readdir {
            match entry_res {
                Ok(entry) => {
                    let path = entry.path();
                    if let Some(filename) = path.file_name() {
                        if filename.as_bytes().starts_with(b"usb-") && filename.as_bytes().ends_with(b"-0:0") {
                            match path.to_str() {
                                Some(arg) => {
                                    disk_args.push(arg.to_string());
                                },
                                None => {
                                    eprintln!("muff: error reading directory entry '{}': invalid UTF-8", path.display());
                                    process::exit(1);
                                }
                            }
                        }
                    }
                },
                Err(err) => {
                    eprintln!("muff: error reading directory '{}': {}", disk_dir, err);
                    process::exit(1);
                }
            }
        }
    } else {
        if let Some(disks) = matches.values_of("DISKS") {
            for arg in disks {
                disk_args.push(arg.to_string());
            }
        }
    }

    if disk_args.is_empty() {
        eprintln!("muff: no disks specified");
        process::exit(1);
    }

    let mounts = match Mount::all() {
        Ok(mounts) => mounts,
        Err(err) => {
            eprintln!("muff: error reading mounts: {}", err);
            process::exit(1);
        }
    };

    let mut disks = Vec::new();
    for disk_arg in disk_args {
        let canonical_path = match fs::canonicalize(&disk_arg) {
            Ok(p) => p,
            Err(err) => {
                eprintln!("muff: error finding disk '{}': {}", disk_arg, err);
                process::exit(1);
            }
        };

        for mount in mounts.iter() {
            if mount.source.as_bytes().starts_with(canonical_path.as_os_str().as_bytes()) {
                if matches.is_present("unmount") {
                    println!(
                        "muff: unmounting '{}': {:?} is mounted at {:?}",
                        disk_arg, mount.source, mount.dest
                    );

                    match Command::new("umount").arg(&mount.source).status() {
                        Ok(status) => {
                            if ! status.success() {
                                eprintln!("muff: failed to unmount {:?}: exit status {}", mount.source, status);
                                process::exit(1);
                            }
                        },
                        Err(err) => {
                            eprintln!("muff: failed to unmount {:?}: {}", mount.source, err);
                            process::exit(1);
                        }
                    }
                } else {
                    eprintln!(
                        "muff: error using disk '{}': {:?} already mounted at {:?}",
                        disk_arg, mount.source, mount.dest
                    );
                    process::exit(1);
                }
            }
        }

        match canonical_path.metadata() {
            Ok(metadata) => {
                if ! metadata.file_type().is_block_device() {
                    eprintln!("muff: error using disk '{}': not a block device", disk_arg);
                    process::exit(1);
                }
            }
            Err(err) => {
                eprintln!("muff: error getting metadata of disk '{}': {}", disk_arg, err);
                process::exit(1);
            }
        }

        let disk_res = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_DIRECT | libc::O_SYNC)
            .open(&canonical_path);

        let disk = match disk_res {
            Ok(disk) => disk,
            Err(err) => {
                eprintln!("muff: error opening disk '{}': {}", disk_arg, err);
                process::exit(1);
            }
        };

        disks.push((disk_arg, disk));
    }

    let image_data = {
        let mut pb = ProgressBar::new(image_size);
        pb.message("Reading image: ");
        pb.set_units(Units::Bytes);

        let mut data = vec![0; image_size as usize];

        let mut total = 0;
        while total < data.len() {
            let end = cmp::min(data.len(), total + 4 * 1024 * 1024);
            let count = match image.read(&mut data[total..end]) {
                Ok(count) => count,
                Err(err) => {
                    eprintln!("muff: error reading image '{}': {}", image_path, err);
                    process::exit(1);
                }
            };
            if count == 0 {
                eprintln!("muff: error reading image '{}': reached EOF", image_path);
                process::exit(1);
            }
            total += count;
            pb.set(total as u64);
        }

        pb.finish();

        Arc::new(data)
    };

    if ! matches.is_present("yes") {
        println!("Are you sure you want to flash '{}' to the following drives?", image_path);
        for ref disk_tuple in disks.iter() {
            println!("  - {}", disk_tuple.0);
        }

        print!("y/N: ");
        io::stdout().flush().unwrap();

        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm).unwrap();

        if confirm.trim() != "y" && confirm.trim() != "yes" {
            println!("Exiting without flashing");
            process::exit(1);
        }
    }

    let check = matches.is_present("check");

    println!("");

    let mut mb = MultiBar::new();

    for (disk_path, mut disk) in disks {
        let mut pb = mb.create_bar(image_size);
        pb.message(&format!("W {}: ", disk_path));
        pb.set_units(Units::Bytes);
        pb.set(0);

        let image_data = image_data.clone();
        let _ = thread::spawn(move || {
            let mut total = 0;
            while total < image_data.len() {
                let end = cmp::min(image_size as usize, total + 4 * 1024 * 1024);
                let count = match disk.write(&image_data[total..end]) {
                    Ok(count) => count,
                    Err(err) => {
                        eprintln!("muff: error writing disk '{}': {}", disk_path, err);
                        process::exit(1);
                    }
                };
                if count == 0 {
                    eprintln!("muff: error writing disk '{}': reached EOF", disk_path);
                    process::exit(1);
                }
                total += count;
                pb.set(total as u64);
            }

            if let Err(err) = disk.flush() {
                eprintln!("muff: error flushing disk '{}': {}", disk_path, err);
                process::exit(1);
            }

            if check {
                match disk.seek(SeekFrom::Start(0)) {
                    Ok(0) => (),
                    Ok(invalid) => {
                        eprintln!("muff: error seeking disk '{}': seeked to {} instead of 0", disk_path, invalid);
                        process::exit(1);
                    },
                    Err(err) => {
                        eprintln!("muff: error seeking disk '{}': {}", disk_path, err);
                        process::exit(1);
                    }
                }

                pb.message(&format!("V {}: ", disk_path));
                pb.set(0);
                total = 0;

                let mut buf = vec![0; 4 * 1024 * 1024];
                while total < image_data.len() {
                    let end = cmp::min(image_size as usize, total + 4 * 1024 * 1024);
                    let count = match disk.read(&mut buf[..end - total]) {
                        Ok(count) => count,
                        Err(err) => {
                            eprintln!("muff: error verifying disk '{}': {}", disk_path, err);
                            process::exit(1);
                        }
                    };
                    if count == 0 {
                        eprintln!("muff: error verifying disk '{}': reached EOF", disk_path);
                        process::exit(1);
                    }

                    if buf[.. count] != image_data[total..total + count] {
                        eprintln!("muff: error verifying disk '{}': mismatch at {}:{}", disk_path, total, total + count);
                        process::exit(1);
                    }

                    total += count;
                    pb.set(total as u64);
                }
            }

            pb.finish();
        });
    }

    mb.listen();
}
