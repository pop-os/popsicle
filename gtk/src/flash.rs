use atomic::Atomic;
use dbus::arg::{OwnedFd, RefArg, Variant};
use dbus::blocking::{Connection, Proxy};
use futures::executor;
use libc;
use popsicle::{Progress, Task};
use proc_mounts::MountList;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::fs::File;
use std::os::unix::io::FromRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type UDisksOptions = HashMap<&'static str, Variant<Box<dyn RefArg>>>;

#[derive(Clone, Copy, PartialEq)]
pub enum FlashStatus {
    Inactive,
    Active,
    Killing,
}

pub struct FlashRequest {
    source: Option<File>,
    destinations: Vec<PathBuf>,
    status: Arc<Atomic<FlashStatus>>,
    progress: Arc<Vec<Atomic<u64>>>,
    finished: Arc<Vec<Atomic<bool>>>,
}

pub struct FlashTask {
    pub progress: Arc<Vec<Atomic<u64>>>,
    pub previous: Arc<Mutex<Vec<[u64; 7]>>>,
    pub finished: Arc<Vec<Atomic<bool>>>,
}

struct FlashProgress<'a> {
    request: &'a FlashRequest,
    id: usize,
    errors: &'a [Cell<Result<(), FlashError>>],
}

#[derive(Clone, Debug)]
pub struct FlashError {
    kind: String, 
    message: String,
}

impl Display for FlashError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for FlashError {}

impl<'a> Progress for FlashProgress<'a> {
    type Device = ();

    fn message(&mut self, _device: &(), kind: &str, message: &str) {
        self.errors[self.id].set(Err(FlashError {
            kind: kind.to_string(),
            message: message.to_string(),
        }));
    }

    fn finish(&mut self) {
        self.request.finished[self.id].store(true, Ordering::SeqCst);
    }

    fn set(&mut self, value: u64) {
        self.request.progress[self.id].store(value, Ordering::SeqCst);
    }
}

impl FlashRequest {
    pub fn new(
        source: File,
        destinations: Vec<PathBuf>,
        status: Arc<Atomic<FlashStatus>>,
        progress: Arc<Vec<Atomic<u64>>>,
        finished: Arc<Vec<Atomic<bool>>>,
    ) -> FlashRequest {
        FlashRequest { source: Some(source), destinations, status, progress, finished }
    }

    pub fn write(mut self) -> anyhow::Result<(anyhow::Result<()>, Vec<Result<(), FlashError>>)> {
        self.status.store(FlashStatus::Active, Ordering::SeqCst);

        let source = self.source.take().unwrap();
        let res = self.write_inner(source);

        for atomic in self.finished.iter() {
            atomic.store(true, Ordering::SeqCst);
        }

        self.status.store(FlashStatus::Inactive, Ordering::SeqCst);

        res
    }

    fn write_inner<'a>(&'a self, source: File) -> anyhow::Result<(anyhow::Result<()>, Vec<Result<(), FlashError>>)> {
        // Unmount the devices beforehand.
        if let Ok(mounts) = MountList::new() {
            for file in &self.destinations {
                for mount in mounts.source_starts_with(file) {
                    let _ = udisks_unmount(&mount.source);
                }
            }
        }

        // Then open them for writing to.
        let mut files = Vec::new();
        for file in &self.destinations {
            let file = udisks_open(file)?;
            files.push(file);
        }

        let mut errors = vec![Ok(()); files.len()];
        let errors_cells = Cell::from_mut(&mut errors as &mut [_]).as_slice_of_cells();

        // How many bytes to write at a given time.
        let mut bucket = [0u8; 64 * 1024];

        let mut task = Task::new(source.into(), false);
        for (i, file) in files.into_iter().enumerate() {
            let progress = FlashProgress {request: &self, errors: errors_cells, id: i};
            task.subscribe(file.into(), (), progress);
        }

        let res = executor::block_on(task.process(&mut bucket));

        Ok((res, errors))
    }
}

fn udisks_unmount(block_device: &Path) -> anyhow::Result<()> {
    let connection = Connection::new_system()?;

    let mut dbus_path = "/org/freedesktop/UDisks2/block_devices/".to_string();
    dbus_path.push_str(str::from_utf8(block_device.strip_prefix("/dev")?.as_os_str().as_bytes())?);
    let dbus_path = ::dbus::strings::Path::new(dbus_path).map_err(anyhow::Error::msg)?;

    let proxy = Proxy::new(
        "org.freedesktop.UDisks2",
        dbus_path,
        Duration::new(25, 0),
        &connection,
    );

    let mut options = UDisksOptions::new();
    options.insert("force", Variant(Box::new(true)));
    let res: Result<(), _> = proxy.method_call(
        "org.freedesktop.UDisks2.Filesystem",
        "Unmount",
        (options,),
    );

    if let Err(err) = res {
        if err.name() != Some("org.freedesktop.UDisks2.Error.NotMounted") {
            return Err(anyhow::Error::new(err));
        }
    }

    Ok(())
}

fn udisks_open(block_device: &Path) -> anyhow::Result<File> {
    let connection = Connection::new_system()?;

    let mut dbus_path = "/org/freedesktop/UDisks2/block_devices/".to_string();
    dbus_path.push_str(str::from_utf8(block_device.strip_prefix("/dev")?.as_os_str().as_bytes())?);
    let dbus_path = ::dbus::strings::Path::new(dbus_path).map_err(anyhow::Error::msg)?;

    let proxy = Proxy::new(
        "org.freedesktop.UDisks2",
        &dbus_path,
        Duration::new(25, 0),
        &connection,
    );

    let mut options = UDisksOptions::new();
    options.insert("flags", Variant(Box::new(libc::O_SYNC)));
    let res: (OwnedFd,) = proxy.method_call(
        "org.freedesktop.UDisks2.Block",
        "OpenDevice",
        ("rw", options),
    )?;

    Ok(unsafe { File::from_raw_fd(res.0.into_fd()) })
}
