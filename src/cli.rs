//! Command line interface for flashing multiple drives concurrently.

use crate::fl;
use anyhow::{Context, anyhow};
use async_std::{
    fs::OpenOptions,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use derive_new::new;
use fomat_macros::{epint, epintln, wite, witeln};
use futures::{
    channel::{mpsc, oneshot},
    executor, join,
    prelude::*,
};
use pbr::{MultiBar, Pipe, ProgressBar, Units};
use popsicle::{Progress, Task, mnt};
use std::{
    env,
    io::{self, IsTerminal, Write},
    process, thread,
};

/// Parsed command line.
struct Args {
    image: Option<String>,
    disks: Vec<String>,
    all: bool,
    check: bool,
    unmount: bool,
    yes: bool,
}

enum Parsed {
    Run(Args),
    Help,
    Version,
}

impl Args {
    /// Parses the arguments by hand: four flags, an image, and any number of disks.
    fn parse(args: impl Iterator<Item = String>) -> Result<Parsed, String> {
        let mut parsed = Args {
            image: None,
            disks: Vec::new(),
            all: false,
            check: false,
            unmount: false,
            yes: false,
        };
        let mut options_done = false;

        for arg in args {
            if options_done || arg == "-" || !arg.starts_with('-') {
                match parsed.image {
                    None => parsed.image = Some(arg),
                    Some(_) => parsed.disks.push(arg),
                }
                continue;
            }

            match arg.as_str() {
                "--" => options_done = true,
                "--all" => parsed.all = true,
                "--check" => parsed.check = true,
                "--unmount" => parsed.unmount = true,
                "--yes" => parsed.yes = true,
                "--help" => return Ok(Parsed::Help),
                "--version" => return Ok(Parsed::Version),
                long if long.starts_with("--") => return Err(unexpected(long)),
                // Short flags may be combined, as in `-ay`.
                short => {
                    for flag in short.chars().skip(1) {
                        match flag {
                            'a' => parsed.all = true,
                            'c' => parsed.check = true,
                            'u' => parsed.unmount = true,
                            'y' => parsed.yes = true,
                            'h' => return Ok(Parsed::Help),
                            'V' => return Ok(Parsed::Version),
                            other => return Err(unexpected(&format!("-{other}"))),
                        }
                    }
                }
            }
        }

        Ok(Parsed::Run(parsed))
    }
}

fn unexpected(arg: &str) -> String {
    fl!("error-unexpected-argument", arg = arg)
}

fn usage() -> String {
    format!(
        "{}: {} [{}] <{}> [{}]...",
        fl!("help-usage"),
        env!("CARGO_PKG_NAME"),
        fl!("help-options").to_uppercase(),
        fl!("arg-image"),
        fl!("arg-disks")
    )
}

/// The `--help` text, which `help2man` also turns into the manual page.
fn help() -> String {
    let image = format!("<{}>", fl!("arg-image"));
    let disks = format!("[{}]...", fl!("arg-disks"));
    let width = image.len().max(disks.len());

    format!(
        "{about}\n{front_end}\n\n{usage}\n\n{arguments}:\n  {image:width$}  {image_desc}\n  {disks:width$}  {disks_desc}\n\n{options}:\n  -a, --all      {all}\n  -c, --check    {check}\n  -u, --unmount  {unmount}\n  -y, --yes      {yes}\n  -h, --help     {help}\n  -V, --version  {version}\n",
        about = env!("CARGO_PKG_DESCRIPTION"),
        front_end = fl!("help-front-end"),
        usage = usage(),
        arguments = fl!("help-arguments"),
        image_desc = fl!("arg-image-desc"),
        disks_desc = fl!("arg-disks-desc"),
        options = fl!("help-options"),
        all = fl!("arg-all-desc"),
        check = fl!("arg-check-desc"),
        unmount = fl!("arg-unmount-desc"),
        yes = fl!("arg-yes-desc"),
        help = fl!("help-print-help"),
        version = fl!("help-print-version"),
    )
}

pub fn run() {
    better_panic::install();

    let args = match Args::parse(env::args().skip(1)) {
        Ok(Parsed::Run(args)) => args,
        Ok(Parsed::Help) => {
            print!("{}", help());
            return;
        }
        Ok(Parsed::Version) => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            return;
        }
        Err(why) => {
            eprintln!("popsicle: {why}\n\n{}", usage());
            process::exit(2);
        }
    };

    let (rtx, rrx) = oneshot::channel::<anyhow::Result<()>>();

    let result = executor::block_on(async move {
        match popsicle(rtx, args).await {
            Err(why) => Err(why),
            _ => match rrx.await {
                Ok(Err(why)) => Err(why),
                _ => Ok(()),
            },
        }
    });

    if let Err(why) = result {
        eprintln!("popsicle: {why}");
        for source in why.chain().skip(1) {
            epintln!("    " (fl!("error-caused-by")) ": " (source))
        }

        process::exit(1);
    }
}

async fn popsicle(rtx: oneshot::Sender<anyhow::Result<()>>, args: Args) -> anyhow::Result<()> {
    let image_path = args.image.with_context(|| fl!("error-image-not-set"))?;

    let image = OpenOptions::new()
        .custom_flags(libc::O_SYNC)
        .read(true)
        .open(&image_path)
        .await
        .with_context(|| fl!("error-image-open", image_path = image_path.clone()))?;

    let image_size = image
        .metadata()
        .await
        .map(|x| x.len())
        .with_context(|| fl!("error-image-metadata", image_path = image_path.clone()))?;

    let mut disk_args = Vec::new();
    if args.all {
        popsicle::usb_disk_devices(&mut disk_args)
            .await
            .with_context(|| fl!("error-disks-fetch"))?;
    } else {
        disk_args.extend(args.disks.iter().map(PathBuf::from).map(Box::from));
    }

    if disk_args.is_empty() {
        return Err(anyhow!(fl!("error-no-disks-specified")));
    }

    let mounts = mnt::get_submounts(Path::new("/")).with_context(|| fl!("error-reading-mounts"))?;

    let disks = popsicle::disks_from_args(disk_args.into_iter(), &mounts, args.unmount)
        .await
        .with_context(|| fl!("error-opening-disks"))?;

    let is_tty = io::stdout().is_terminal();

    if is_tty && !args.yes {
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

        if confirm.trim() != fl!("y") && confirm.trim() != "yes" {
            return Err(anyhow!(fl!("error-exiting")));
        }
    }

    let check = args.check;

    // If this is a TTY, display a progress bar. If not, display machine-readable info.
    if is_tty {
        println!();

        let mb = MultiBar::new();
        let mut task = Task::new(image, check);

        for (disk_path, disk) in disks {
            let mut bar = mb.create_bar(image_size);
            bar.set_units(Units::Bytes);
            bar.message(&format!("W {}: ", disk_path.display()));
            let pb = InteractiveProgress::new(bar);

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
