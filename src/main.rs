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

fn muff() -> Result<(), String> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
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
            return Err(format!(
                "error opening image '{}': {}", image_path, err
            ));
        }
    };

    let image_size = match image.metadata() {
        Ok(metadata) => {
            if ! metadata.file_type().is_file() {
                return Err(format!(
                    "error using image '{}': not a file", image_path
                ));
            }
            metadata.len()
        }
        Err(err) => {
            return Err(format!(
                "error getting metadata of image '{}': {}", image_path, err
            ));
        }
    };

    let mut disk_args = vec![];
    if matches.is_present("all") {
        let disk_dir = "/dev/disk/by-id/";
        let readdir = match fs::read_dir(disk_dir) {
            Ok(readdir) => readdir,
            Err(err) => {
                return Err(format!(
                    "error opening directory '{}': {}", disk_dir, err
                ));
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
                                    return Err(format!(
                                        "error reading directory entry '{}': invalid UTF-8", path.display()
                                    ));
                                }
                            }
                        }
                    }
                },
                Err(err) => {
                    return Err(format!(
                        "error reading directory '{}': {}", disk_dir, err
                    ));
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
        return Err(format!(
            "no disks specified"
        ));
    }

    let mounts = match Mount::all() {
        Ok(mounts) => mounts,
        Err(err) => {
            return Err(format!(
                "error reading mounts: {}", err
            ));
        }
    };

    let mut disks = Vec::new();
    for disk_arg in disk_args {
        let canonical_path = match fs::canonicalize(&disk_arg) {
            Ok(p) => p,
            Err(err) => {
                return Err(format!(
                    "error finding disk '{}': {}", disk_arg, err
                ));
            }
        };

        for mount in mounts.iter() {
            if mount.source.as_bytes().starts_with(canonical_path.as_os_str().as_bytes()) {
                if matches.is_present("unmount") {
                    println!(
                        "unmounting '{}': {:?} is mounted at {:?}",
                        disk_arg, mount.source, mount.dest
                    );

                    match Command::new("umount").arg(&mount.source).status() {
                        Ok(status) => {
                            if ! status.success() {
                                return Err(format!(
                                    "failed to unmount {:?}: exit status {}", mount.source, status
                                ));
                            }
                        },
                        Err(err) => {
                            return Err(format!(
                                "failed to unmount {:?}: {}", mount.source, err
                            ));
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
                if ! metadata.file_type().is_block_device() {
                    return Err(format!(
                        "error using disk '{}': not a block device", disk_arg
                    ));
                }
            }
            Err(err) => {
                return Err(format!(
                    "error getting metadata of disk '{}': {}", disk_arg, err
                ));
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
                return Err(format!(
                    "error opening disk '{}': {}", disk_arg, err
                ));
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
                    return Err(format!(
                        "error reading image '{}': {}", image_path, err
                    ));
                }
            };
            if count == 0 {
                return Err(format!(
                    "error reading image '{}': reached EOF", image_path
                ));
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
            return Err(format!(
                "exiting without flashing"
            ));
        }
    }

    let check = matches.is_present("check");

    println!("");

    let mut mb = MultiBar::new();

    let mut threads = Vec::new();
    for (disk_path, mut disk) in disks {
        let mut pb = mb.create_bar(image_size);
        pb.message(&format!("W {}: ", disk_path));
        pb.set_units(Units::Bytes);
        pb.set(0);

        let image_data = image_data.clone();
        threads.push(thread::spawn(move || -> Result<(), String> {
            let mut total = 0;
            while total < image_data.len() {
                let end = cmp::min(image_size as usize, total + 4 * 1024 * 1024);
                let count = match disk.write(&image_data[total..end]) {
                    Ok(count) => count,
                    Err(err) => {
                        pb.message(&format!("! {}: ", disk_path));
                        pb.finish();

                        return Err(format!(
                            "error writing disk '{}': {}", disk_path, err
                        ));
                    }
                };
                if count == 0 {
                    pb.message(&format!("! {}: ", disk_path));
                    pb.finish();

                    return Err(format!(
                        "error writing disk '{}': reached EOF", disk_path
                    ));
                }
                total += count;
                pb.set(total as u64);
            }

            if let Err(err) = disk.flush() {
                pb.message(&format!("! {}: ", disk_path));
                pb.finish();

                return Err(format!(
                    "error flushing disk '{}': {}", disk_path, err
                ));
            }

            if check {
                match disk.seek(SeekFrom::Start(0)) {
                    Ok(0) => (),
                    Ok(invalid) => {
                        pb.message(&format!("! {}: ", disk_path));
                        pb.finish();

                        return Err(format!(
                            "error seeking disk '{}': seeked to {} instead of 0", disk_path, invalid
                        ));
                    },
                    Err(err) => {
                        pb.message(&format!("! {}: ", disk_path));
                        pb.finish();

                        return Err(format!(
                            "error seeking disk '{}': {}", disk_path, err
                        ));
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
                            pb.message(&format!("! {}: ", disk_path));
                            pb.finish();

                            return Err(format!(
                                "error verifying disk '{}': {}", disk_path, err
                            ));
                        }
                    };
                    if count == 0 {
                        pb.message(&format!("! {}: ", disk_path));
                        pb.finish();

                        return Err(format!(
                            "error verifying disk '{}': reached EOF", disk_path
                        ));
                    }

                    if buf[.. count] != image_data[total..total + count] {
                        pb.message(&format!("! {}: ", disk_path));
                        pb.finish();

                        return Err(format!(
                            "error verifying disk '{}': mismatch at {}:{}", disk_path, total, total + count
                        ));
                    }

                    total += count;
                    pb.set(total as u64);
                }
            }

            pb.finish();

            Ok(())
        }));
    }

    mb.listen();

    for thread in threads {
        thread.join().unwrap()?;
    }

    Ok(())
}

fn main() {
    match muff() {
        Ok(()) => (),
        Err(err) => {
            writeln!(io::stderr(), "muff: {}", err).unwrap();
            process::exit(1);
        }
    }
}
