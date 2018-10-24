//! CLI application for flashing multiple drives in parallel.

extern crate bus_writer;
extern crate clap;
extern crate libc;
extern crate pbr;
extern crate popsicle;

use bus_writer::*;
use clap::{App, Arg};
use pbr::{MultiBar, Units};
use std::fs::File;
use std::io::{self, Write};
use std::thread::{self, JoinHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::path::Path;
use std::process;

use popsicle::mnt;

fn popsicle() -> Result<(), String> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("IMAGE")
                .help("Input image file")
                .required(true),
        )
        .arg(
            Arg::with_name("DISKS")
                .help("Output disk devices")
                .multiple(true),
        )
        .arg(
            Arg::with_name("all")
                .help("Flash all detected USB drives")
                .short("a")
                .long("all"),
        )
        .arg(
            Arg::with_name("check")
                .help("Check written image matches read image")
                .short("c")
                .long("check"),
        )
        .arg(
            Arg::with_name("unmount")
                .help("Unmount mounted devices")
                .short("u")
                .long("unmount"),
        )
        .arg(
            Arg::with_name("yes")
                .help("Continue without confirmation")
                .short("y")
                .long("yes"),
        )
        .get_matches();

    let image_path = matches.value_of("IMAGE").expect("IMAGE not set");
    let mut image = File::open(image_path)
        .map_err(|why| format!("error with image at '{}': {}", image_path, why))?;
    let image_size = image.metadata()
        .map(|x| x.len())
        .map_err(|why| format!("image metadata error at '{}': {}", image_path, why))?;

    let mut disk_args = vec![];
    if matches.is_present("all") {
        if let Err(err) = popsicle::get_disk_args(&mut disk_args) {
            return Err(format!("error getting USB disks: {}", err));
        }
    } else if let Some(disks) = matches.values_of("DISKS") {
        for arg in disks {
            disk_args.push(arg.to_string());
        }
    }

    if disk_args.is_empty() {
        return Err("no disks specified".to_owned());
    }

    let mounts = match mnt::get_submounts(Path::new("/")) {
        Ok(mounts) => mounts,
        Err(err) => {
            return Err(format!("error reading mounts: {}", err));
        }
    };

    let disks = popsicle::disks_from_args(
        disk_args.into_iter(),
        &mounts,
        matches.is_present("unmount"),
    ).map_err(|why| format!("disk error: {}", why))?;

    if !matches.is_present("yes") {
        println!(
            "Are you sure you want to flash '{}' to the following drives?",
            image_path
        );

        for disk_tuple in &disks {
            println!("  - {}", disk_tuple.0);
        }

        print!("y/N: ");
        io::stdout().flush().unwrap();

        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm).unwrap();

        if confirm.trim() != "y" && confirm.trim() != "yes" {
            return Err("exiting without flashing".to_owned());
        }
    }

    let check = matches.is_present("check");

    println!();

    let mut mb = MultiBar::new();

    let mut paths = Vec::with_capacity(disks.len());
    let mut destinations = Vec::with_capacity(disks.len());
    let mut pbs = Vec::with_capacity(disks.len());
    let errored = Arc::new(
        (0..disks.len())
            .map(|_| AtomicBool::new(false))
            .collect::<Vec<_>>()
    );

    for (disk_path, disk) in disks {
        let mut pb = mb.create_bar(image_size);
        pb.set_units(Units::Bytes);
        pb.message(&format!("W {}: ", disk_path));
        pbs.push(pb);
        paths.push(disk_path);
        destinations.push(disk);
    }

    let pbs = Arc::new(Mutex::new(pbs));
    let errored_ = errored.clone();
    let handle: JoinHandle<io::Result<()>> = thread::Builder::new().stack_size(10 * 1024 * 1024).spawn(move || {
        let mut bucket = [0u8; 8 * 1024 * 1024];
        BusWriter::new(
            &mut image,
            &mut destinations,
            |event| {
                let pbs = &mut pbs.lock().unwrap();
                match event {
                    BusWriterMessage::Written { id, bytes_written } => {
                        pbs[id].set(bytes_written);
                    },
                    BusWriterMessage::Completed { id } => {
                        pbs[id].set(image_size);
                        if ! check {
                            pbs[id].finish();
                        }
                    }
                    BusWriterMessage::Errored { id, why } => {
                        let pb = &mut pbs[id];
                        pb.message(&format!("E {}: {}", paths[id], why));
                        pb.finish();
                        errored_[id].store(true, Ordering::SeqCst);
                    }
                }
            },
            // TODO Ctrl + C signal handling
            || false
        ).with_bucket(&mut bucket).write()?;

        let mut unerrored_ids = Vec::new();
        let mut unerrored_destinations = Vec::new();

        for (id, dest) in destinations.iter().enumerate()
            .filter(|&(id, _)| !errored_[id].load(Ordering::SeqCst))
        {
            unerrored_ids.push(id);
            unerrored_destinations.push(dest);
        }

        if check {
            for (pb, path) in pbs.lock().unwrap().iter_mut().zip(paths.iter()) {
                pb.set(0);
                pb.message(&format!("V {}: ", path));
            }

            BusVerifier::new(
                image,
                &mut unerrored_destinations,
                |event| {
                    let pbs = &mut pbs.lock().unwrap();
                    match event {
                        BusVerifierMessage::Read { id, bytes_read } => {
                            pbs[unerrored_ids[id]].set(bytes_read);
                        }
                        BusVerifierMessage::Invalid { id } => {
                            pbs[unerrored_ids[id]].finish();
                            errored_[unerrored_ids[id]].store(true, Ordering::SeqCst);
                        }
                        BusVerifierMessage::Valid { id } => {
                            pbs[unerrored_ids[id]].set(image_size);
                            pbs[unerrored_ids[id]].finish();
                        }
                        BusVerifierMessage::Errored { id, why } => {
                            let pb = &mut pbs[unerrored_ids[id]];
                            pb.message(&format!("E {}: {}", paths[unerrored_ids[id]], why));
                            pb.finish();
                            errored_[unerrored_ids[id]].store(true, Ordering::SeqCst);
                        }
                    }
                },
                || false
            ).with_bucket(&mut bucket).verify()
        } else {
            Ok(())
        }
    }).unwrap();

    mb.listen();

    handle.join()
        .unwrap()
        .map_err(|why| format!("bus failed: {}", why))
        .and_then(|_| {
            if errored.iter().any(|x| x.load(Ordering::SeqCst)) {
                Err("a device failed".to_owned())
            } else {
                Ok(())
            }
        })
}

fn main() {
    match popsicle() {
        Ok(()) => (),
        Err(err) => {
            eprintln!("popsicle: {}", err);
            process::exit(1);
        }
    }
}
