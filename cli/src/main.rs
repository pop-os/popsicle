//! CLI application for flashing multiple drives concurrently.

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate cascade;
#[macro_use]
extern crate fomat_macros;

use anyhow::Context;
use async_std::{
    fs::File,
    path::{Path, PathBuf},
};
use clap::{App, Arg, ArgMatches};
use futures::{channel::oneshot, executor};
use pbr::{MultiBar, Pipe, ProgressBar, Units};
use popsicle::{mnt, Progress, Task};
use std::{
    io::{self, Write},
    process, thread,
};

fn main() {
    better_panic::install();

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("IMAGE").help("Input image file").required(true))
        .arg(Arg::with_name("DISKS").help("Output disk devices").multiple(true))
        .arg(Arg::with_name("all").help("Flash all detected USB drives").short("a").long("all"))
        .arg(
            Arg::with_name("check")
                .help("Check written image matches read image")
                .short("c")
                .long("check"),
        )
        .arg(Arg::with_name("unmount").help("Unmount mounted devices").short("u").long("unmount"))
        .arg(Arg::with_name("yes").help("Continue without confirmation").short("y").long("yes"))
        .get_matches();

    let (rtx, rrx) = oneshot::channel::<anyhow::Result<()>>();

    let result = executor::block_on(async move {
        match popsicle(rtx, matches).await {
            Err(why) => Err(why),
            _ => match rrx.await {
                Ok(Err(why)) => Err(why),
                _ => Ok(()),
            },
        }
    });

    if let Err(why) = result {
        eprintln!("popsicle: {}", why);
        for source in why.chain().skip(1) {
            eprintln!("    caused by: {}", source)
        }

        process::exit(1);
    }
}

async fn popsicle(
    rtx: oneshot::Sender<anyhow::Result<()>>,
    matches: ArgMatches<'_>,
) -> anyhow::Result<()> {
    let image_path = matches.value_of("IMAGE").expect("IMAGE not set");

    let image = File::open(image_path)
        .await
        .with_context(|| format!("error with image at '{}'", image_path))?;

    let image_size = image
        .metadata()
        .await
        .map(|x| x.len())
        .with_context(|| format!("image metadata error at '{}'", image_path))?;

    let mut disk_args = Vec::new();
    if matches.is_present("all") {
        popsicle::usb_disk_devices(&mut disk_args).await.context("error getting USB disks")?;
    } else if let Some(disks) = matches.values_of("DISKS") {
        disk_args.extend(disks.map(String::from).map(PathBuf::from).map(Box::from));
    }

    if disk_args.is_empty() {
        return Err(anyhow!("no disks specified"));
    }

    let mounts = mnt::get_submounts(Path::new("/")).context("error reading mounts")?;

    let disks =
        popsicle::disks_from_args(disk_args.into_iter(), &mounts, matches.is_present("unmount"))
            .await
            .context("failed to open disks")?;

    if !matches.is_present("yes") {
        epint!(
            "Are you sure you want to flash '" (image_path) "' to the following drives?\n"
            for (path, _) in &disks {
                " - " (path.display()) "\n"
            }
            "y/N: "
        );

        io::stdout().flush().unwrap();

        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm).unwrap();

        if confirm.trim() != "y" && confirm.trim() != "yes" {
            return Err(anyhow!("exiting without flashing"));
        }
    }

    let check = matches.is_present("check");

    println!();

    let mut mb = MultiBar::new();
    let mut task = Task::new(image, check);

    for (disk_path, disk) in disks {
        let pb = CliProgress(cascade! {
            mb.create_bar(image_size);
            ..set_units(Units::Bytes);
            ..message(&format!("W {}: ", disk_path.display()));
        });

        task.subscribe(disk, disk_path, pb);
    }

    thread::spawn(move || {
        executor::block_on(async move {
            let buf = &mut [0u8; 64 * 1024];
            let _ = rtx.send(task.process(buf).await);
        })
    });

    mb.listen();

    Ok(())
}

pub struct CliProgress(ProgressBar<Pipe>);

impl Progress for CliProgress {
    fn message(&mut self, message: &str) {
        self.0.message(message);
    }

    fn finish(&mut self) {
        self.0.finish();
    }

    fn set(&mut self, written: u64) {
        self.0.set(written);
    }
}
