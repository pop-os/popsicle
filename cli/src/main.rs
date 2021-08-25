//! CLI application for flashing multiple drives concurrently.

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate cascade;
#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate fomat_macros;

mod localize;

use anyhow::Context;
use async_std::{
    fs::OpenOptions,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use clap::{App, Arg, ArgMatches};
use futures::{
    channel::{mpsc, oneshot},
    executor, join,
    prelude::*,
};
use i18n_embed::DesktopLanguageRequester;
use pbr::{MultiBar, Pipe, ProgressBar, Units};
use popsicle::{mnt, Progress, Task};
use std::{
    io::{self, Write},
    process, thread,
};

fn main() {
    translate();
    better_panic::install();

    let arg_image = fl!("arg-image");
    let arg_disks = fl!("arg-disks");

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name(&arg_image).help(&fl!("arg-image-desc")).required(true))
        .arg(Arg::with_name(&arg_disks).help(&fl!("arg-disks-desc")).multiple(true))
        .arg(Arg::with_name("all").help(&fl!("arg-all-desc")).short("a").long("all"))
        .arg(Arg::with_name("check").help(&fl!("arg-check-desc")).short("c").long("check"))
        .arg(Arg::with_name("unmount").help(&fl!("arg-unmount-desc")).short("u").long("unmount"))
        .arg(Arg::with_name("yes").help(&fl!("arg-yes-desc")).short("y").long("yes"))
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
            epintln!("    " (fl!("error-caused-by")) ": " (source))
        }

        process::exit(1);
    }
}

async fn popsicle(
    rtx: oneshot::Sender<anyhow::Result<()>>,
    matches: ArgMatches<'_>,
) -> anyhow::Result<()> {
    let image_path =
        matches.value_of(&fl!("arg-image")).with_context(|| fl!("error-image-not-set"))?;

    let image = OpenOptions::new()
        .custom_flags(libc::O_SYNC)
        .read(true)
        .open(image_path)
        .await
        .with_context(|| fl!("error-image-open", image_path = image_path.clone()))?;

    let image_size = image
        .metadata()
        .await
        .map(|x| x.len())
        .with_context(|| fl!("error-image-metadata", image_path = image_path.clone()))?;

    let mut disk_args = Vec::new();
    if matches.is_present("all") {
        popsicle::usb_disk_devices(&mut disk_args)
            .await
            .with_context(|| fl!("error-disks-fetch"))?;
    } else if let Some(disks) = matches.values_of("DISKS") {
        disk_args.extend(disks.map(String::from).map(PathBuf::from).map(Box::from));
    }

    if disk_args.is_empty() {
        return Err(anyhow!(fl!("error-no-disks-specified")));
    }

    let mounts = mnt::get_submounts(Path::new("/")).with_context(|| fl!("error-reading-mounts"))?;

    let disks =
        popsicle::disks_from_args(disk_args.into_iter(), &mounts, matches.is_present("unmount"))
            .await
            .with_context(|| fl!("error-opening-disks"))?;

    let is_tty = atty::is(atty::Stream::Stdout);

    if is_tty && !matches.is_present("yes") {
        epint!(
            (fl!("question", image_path = image_path)) "\n"
            for (path, _) in &disks {
                " - " (path.display()) "\n"
            }
            (fl!("yn")) ": "
        );

        io::stdout().flush().unwrap();

        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm).unwrap();

        if confirm.trim() != "y" && confirm.trim() != "yes" {
            return Err(anyhow!(fl!("error-exiting")));
        }
    }

    let check = matches.is_present("check");

    // If this is a TTY, display a progress bar. If not, display machine-readable info.
    if is_tty {
        println!();

        let mb = MultiBar::new();
        let mut task = Task::new(image, check);

        for (disk_path, disk) in disks {
            let pb = InteractiveProgress::new(cascade! {
                mb.create_bar(image_size);
                ..set_units(Units::Bytes);
                ..message(&format!("W {}: ", disk_path.display()));
            });

            task.subscribe(disk, disk_path, pb);
        }

        thread::spawn(|| {
            executor::block_on(async move {
                let buf = &mut [0u8; 64 * 1024];
                let _ = rtx.send(task.process(buf).await);
            })
        });

        mb.listen();
    } else {
        let (etx, erx) = mpsc::unbounded();
        let mut paths = Vec::new();
        let mut task = Task::new(image, check);

        for (disk_path, disk) in disks {
            let pb = MachineProgress::new(paths.len(), etx.clone());
            paths.push(disk_path.clone());
            task.subscribe(disk, disk_path, pb);
        }

        drop(etx);

        let task = async move {
            let buf = &mut [0u8; 64 * 1024];
            let _ = rtx.send(task.process(buf).await);
        };

        join!(machine_output(erx, &paths, image_size), task);
    }

    Ok(())
}

/// An event for creating a machine-readable output
pub enum Event {
    Message(usize, Box<str>),
    Finished(usize),
    Set(usize, u64),
}

/// Tracks progress
#[derive(new)]
pub struct MachineProgress {
    id: usize,

    handle: mpsc::UnboundedSender<Event>,
}

impl Progress for MachineProgress {
    type Device = Box<Path>;

    fn message(&mut self, _path: &Box<Path>, kind: &str, message: &str) {
        let _ = self.handle.unbounded_send(Event::Message(
            self.id,
            if message.is_empty() { kind.into() } else { [kind, " ", message].concat().into() },
        ));
    }

    fn finish(&mut self) {
        let _ = self.handle.unbounded_send(Event::Finished(self.id));
    }

    fn set(&mut self, written: u64) {
        let _ = self.handle.unbounded_send(Event::Set(self.id, written));
    }
}

#[derive(new)]
pub struct InteractiveProgress {
    pipe: ProgressBar<Pipe>,
}

impl Progress for InteractiveProgress {
    type Device = Box<Path>;

    fn message(&mut self, path: &Box<Path>, kind: &str, message: &str) {
        self.pipe.message(&format!("{} {}: {}", kind, path.display(), message));
    }

    fn finish(&mut self) {
        self.pipe.finish();
    }

    fn set(&mut self, written: u64) {
        self.pipe.set(written);
    }
}

/// Writes a machine-friendly output, when this program is being piped into another.
async fn machine_output(
    mut rx: mpsc::UnboundedReceiver<Event>,
    paths: &[Box<Path>],
    image_size: u64,
) {
    let stdout = io::stdout();
    let stdout = &mut stdout.lock();

    let _ = wite!(
        stdout,
        "Size(" (image_size) ")\n"
        for path in paths {
            "Device(\"" (path.display()) "\")\n"
        }
    );

    while let Some(event) = rx.next().await {
        match event {
            Event::Message(id, message) => {
                let _ = witeln!(stdout, "Message(\"" (paths[id].display()) "\",\"" (message) "\")");
            }
            Event::Finished(id) => {
                let _ = witeln!(stdout, "Finished(\"" (paths[id].display()) "\")");
            }
            Event::Set(id, written) => {
                let _ = witeln!(stdout, "Set(\"" (paths[id].display()) "\"," (written) ")");
            }
        }
    }
}

fn translate() {
    let requested_languages = DesktopLanguageRequester::requested_languages();
    let localizer = crate::localize::localizer();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!("Error while loading languages for popsicle-cli {}", error);
    }
}
