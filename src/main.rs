//! MUFF - Multiple USB File Flasher

extern crate clap;
extern crate libc;
extern crate pbr;

use clap::{App, Arg};
use pbr::{ProgressBar, MultiBar, Units};
use std::{cmp, process, thread};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::Arc;

mod mounts;

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
                .required(true)
                .multiple(true)
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
        Ok(metadata) => metadata.len(),
        Err(err) => {
            eprintln!("muff: error getting metadata of image '{}': {}", image_path, err);
            process::exit(1);
        }
    };

    let mut disk_paths = Vec::new();
    for disk_arg in matches.values_of("DISKS").expect("DISKS not set") {
        let canonical_path = match fs::canonicalize(disk_arg) {
            Ok(p) => p,
            Err(err) => {
                eprintln!("muff: error finding disk '{}': {}", disk_arg, err);
                process::exit(1);
            }
        };

        disk_paths.push(canonical_path);
    }

    let mut disks = Vec::new();
    for disk_path in disk_paths {
        let file_res = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open(&disk_path);

        let disk = match file_res {
            Ok(file) => file,
            Err(err) => {
                eprintln!("muff: error opening disk '{}': {}", disk_path.display(), err);
                process::exit(1);
            }
        };

        disks.push((format!("{}", disk_path.display()), disk));
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

    println!("");

    let mut mb = MultiBar::new();

    for (disk_path, mut disk) in disks {
        let mut pb = mb.create_bar(image_size);
        pb.message(&format!("Writing {}: ", disk_path));
        pb.set_units(Units::Bytes);

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

            pb.finish();
        });
    }

    mb.listen();
}
