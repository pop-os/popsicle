// SPDX-License-Identifier: MIT

use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    os::fd::FromRawFd,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use cosmic::{
    Apply, Element, Task,
    iced::{Length, Subscription},
    theme::spacing,
    widget,
};
use dbus::{
    arg::{OwnedFd, RefArg, Variant},
    blocking::{Connection, Proxy},
};
use dbus_udisks2::DiskDevice;
use futures::{
    executor,
    future::{AbortHandle, AbortRegistration, Abortable},
};
use popsicle::{Progress, Task as CopyTask};

use crate::{app::ActiveView, fl, views::devices::device_label};

/// Result of a flashing run, consumed by the summary view.
#[derive(Debug, Clone)]
pub struct FlashOutcome {
    /// Set when the run as a whole failed (image unreadable, no drive could be opened, ...).
    pub error: Option<String>,
    pub total: usize,
    pub failed: Vec<(Arc<DiskDevice>, String)>,
}

impl FlashOutcome {
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.failed.is_empty()
    }

    pub fn succeeded(&self) -> usize {
        if self.error.is_some() { 0 } else { self.total - self.failed.len() }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Finished { generation: u64, outcome: Result<FlashOutcome, String> },
}

/// State shared with the blocking worker. Plain atomics, no `atomic`/`bytemuck` crates needed.
struct Shared {
    progress: Vec<AtomicU64>,
    finished: Vec<AtomicBool>,
    errors: Mutex<Vec<Option<String>>>,
}

struct DeviceProgress {
    shared: Arc<Shared>,
    id: usize,
}

impl Progress for DeviceProgress {
    type Device = ();

    fn message(&mut self, _device: &(), kind: &str, message: &str) {
        // "E" = error. ("S"/"V" are seek/verify notices, only sent when verification is on.)
        if kind == "E" {
            self.shared.errors.lock().unwrap()[self.id] = Some(message.to_string());
        }
    }

    fn finish(&mut self) {
        self.shared.finished[self.id].store(true, Ordering::Relaxed);
    }

    fn set(&mut self, value: u64) {
        self.shared.progress[self.id].store(value, Ordering::Relaxed);
    }
}

#[derive(Default)]
struct DeviceState {
    fraction: f32,
    speed: Option<u64>,
    finished: bool,
    error: Option<String>,
    samples: VecDeque<(Instant, u64)>,
}

struct Run {
    devices: Vec<Arc<DiskDevice>>,
    image_size: u64,
    shared: Arc<Shared>,
    abort: AbortHandle,
    states: Vec<DeviceState>,
}

impl Run {
    fn sample(&mut self, now: Instant) {
        let errors = self.shared.errors.lock().unwrap().clone();

        for (id, state) in self.states.iter_mut().enumerate() {
            let bytes = self.shared.progress[id].load(Ordering::Relaxed);
            state.finished = self.shared.finished[id].load(Ordering::Relaxed);
            state.error = errors[id].clone();

            state.fraction = if state.finished && state.error.is_none() {
                1.0
            } else if self.image_size == 0 {
                0.0
            } else {
                (bytes as f64 / self.image_size as f64).min(1.0) as f32
            };

            // Rolling ~3 second window, like popsicle's GTK label.
            state.samples.push_back((now, bytes));
            while state.samples.len() > 2
                && now.duration_since(state.samples[0].0) > Duration::from_secs(3)
            {
                state.samples.pop_front();
            }

            state.speed = match (state.samples.front(), state.samples.back()) {
                (Some(&(t0, b0)), Some(&(t1, b1))) if t1 > t0 && !state.finished => {
                    let secs = t1.duration_since(t0).as_secs_f64();
                    Some((b1.saturating_sub(b0) as f64 / secs) as u64)
                }
                _ => None,
            };
        }
    }
}

#[derive(Default)]
pub struct FlashingView {
    run: Option<Run>,
    /// Bumped on every start so a late `Finished` from a cancelled run is ignored.
    generation: u64,
    outcome: Option<Result<FlashOutcome, String>>,
}

impl FlashingView {
    /// Begin flashing `image` to `devices`. Call when entering the view.
    pub fn start(
        &mut self,
        image: PathBuf,
        image_size: u64,
        devices: Vec<Arc<DiskDevice>>,
    ) -> Task<cosmic::Action<Message>> {
        self.cancel();
        self.generation += 1;
        self.outcome = None;

        let generation = self.generation;
        let count = devices.len();

        let shared = Arc::new(Shared {
            progress: (0..count).map(|_| AtomicU64::new(0)).collect(),
            finished: (0..count).map(|_| AtomicBool::new(false)).collect(),
            errors: Mutex::new(vec![None; count]),
        });

        let (abort, registration) = AbortHandle::new_pair();

        self.run = Some(Run {
            devices: devices.clone(),
            image_size,
            shared: shared.clone(),
            abort,
            states: (0..count).map(|_| DeviceState::default()).collect(),
        });

        cosmic::task::future(async move {
            let outcome =
                tokio::task::spawn_blocking(move || flash(&image, &devices, &shared, registration))
                    .await
                    .map_err(|error| format!("the flashing thread failed: {error}"))
                    .and_then(|result| result);

            Message::Finished { generation, outcome }
        })
        .map(cosmic::Action::App)
    }

    /// Stop an in-progress run. Safe to call when idle.
    pub fn cancel(&mut self) {
        if let Some(run) = self.run.take() {
            run.abort.abort();
        }
    }

    /// Set once a run completes; the app moves to the summary view when this is `Some`.
    pub fn take_outcome(&mut self) -> Option<Result<FlashOutcome, String>> {
        self.outcome.take()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.run.is_some() {
            cosmic::iced::time::every(Duration::from_millis(500)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tick => {
                if let Some(run) = &mut self.run {
                    run.sample(Instant::now());
                }
            }

            Message::Finished { generation, outcome } => {
                if generation == self.generation && self.run.take().is_some() {
                    self.outcome = Some(outcome);
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let space = spacing();

        let header = widget::column([])
            .push(widget::text::title2(fl!("flash-view-title")))
            .push(widget::text::body(fl!("flash-view-description")))
            .width(Length::Fill)
            .spacing(space.space_xxs);

        let mut list = widget::column([]).spacing(space.space_xs);

        if let Some(run) = &self.run {
            for (device, state) in run.devices.iter().zip(&run.states) {
                list = list.push(device_row(device, state));
            }
        }

        widget::column([])
            .push(header)
            .push(list)
            .spacing(space.space_s)
            .apply(widget::scrollable)
            .into()
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, crate::app::Message>> {
        if *view != ActiveView::Flashing {
            return None;
        }

        let cancel =
            widget::button::destructive(fl!("cancel")).on_press(crate::app::Message::Restart);

        Some(
            widget::row([])
                .push(widget::space::horizontal())
                .push(cancel)
                .spacing(spacing().space_xs)
                .padding(spacing().space_xs)
                .into(),
        )
    }
}

fn device_row<'a>(device: &DiskDevice, state: &DeviceState) -> Element<'a, Message> {
    let status = if let Some(error) = &state.error {
        error.clone()
    } else if state.finished {
        fl!("task-finished")
    } else if let Some(speed) = state.speed {
        format!("{}/s", bytesize::to_string(speed, true))
    } else {
        String::new()
    };

    let content = widget::column([])
        .push(widget::text::heading(device_label(device)))
        .push(widget::determinate_linear(state.fraction).width(Length::Fill).girth(8.0))
        .push(widget::text::caption(status))
        .spacing(spacing().space_xxs);

    widget::container(content)
        .width(Length::Fill)
        .padding(spacing().space_s)
        .class(cosmic::theme::Container::Card)
        .into()
}

fn flash(
    image: &Path,
    devices: &[Arc<DiskDevice>],
    shared: &Arc<Shared>,
    registration: AbortRegistration,
) -> Result<FlashOutcome, String> {
    let copy = write_all(image, devices, shared, registration).map_err(|e| format!("{e:#}"));

    let error = copy.err().map(|e| format!("{e:#}"));

    let errors = shared.errors.lock().unwrap();
    let failed = devices
        .iter()
        .zip(errors.iter())
        .filter_map(|(device, error)| error.clone().map(|error| (device.clone(), error)))
        .collect();

    Ok(FlashOutcome { error, total: devices.len(), failed })
}

fn write_all(
    image: &Path,
    devices: &[Arc<DiskDevice>],
    shared: &Arc<Shared>,
    registration: AbortRegistration,
) -> anyhow::Result<()> {
    let source = File::open(image).context("failed to open the image")?;

    // Unmount everything on the target drives first.
    for device in devices {
        let _ = udisks_unmount(&device.parent.path);
        for partition in &device.partitions {
            let _ = udisks_unmount(&partition.path);
        }
    }

    // Open each drive; a drive that fails to open is reported but doesn't sink the others.
    let mut task = CopyTask::new(source.into(), false);
    let mut opened = 0;

    for (id, device) in devices.iter().enumerate() {
        match udisks_open(&device.parent.path) {
            Ok(file) => {
                task.subscribe(file.into(), (), DeviceProgress { shared: shared.clone(), id });
                opened += 1;
            }
            Err(why) => {
                shared.errors.lock().unwrap()[id] = Some(format!("{why:#}"));
                shared.finished[id].store(true, Ordering::Relaxed);
            }
        }
    }

    anyhow::ensure!(opened > 0, "none of the selected drives could be opened");

    // Heap-allocated: no need for popsicle's 10 MiB thread stack.
    let mut bucket = vec![0u8; 64 * 1024];

    match executor::block_on(Abortable::new(task.process(&mut bucket), registration)) {
        Ok(result) => result,
        Err(_aborted) => anyhow::bail!("cancelled"),
    }
}

type UDisksOptions = HashMap<&'static str, Variant<Box<dyn RefArg>>>;

fn udisks_unmount(dbus_path: &str) -> anyhow::Result<()> {
    let connection = Connection::new_system()?;
    let dbus_path = ::dbus::strings::Path::new(dbus_path).map_err(anyhow::Error::msg)?;
    let proxy = Proxy::new("org.freedesktop.UDisks2", dbus_path, Duration::new(25, 0), &connection);

    let mut options = UDisksOptions::new();
    options.insert("force", Variant(Box::new(true)));

    let res: Result<(), _> =
        proxy.method_call("org.freedesktop.UDisks2.Filesystem", "Unmount", (options,));

    if let Err(err) = res {
        if err.name() != Some("org.freedesktop.UDisks2.Error.NotMounted") {
            return Err(anyhow::Error::new(err));
        }
    }

    Ok(())
}

fn udisks_open(dbus_path: &str) -> anyhow::Result<File> {
    let connection = Connection::new_system()?;
    let dbus_path = ::dbus::strings::Path::new(dbus_path).map_err(anyhow::Error::msg)?;
    let proxy =
        Proxy::new("org.freedesktop.UDisks2", &dbus_path, Duration::new(25, 0), &connection);

    let mut options = UDisksOptions::new();
    options.insert("flags", Variant(Box::new(libc::O_SYNC)));

    let res: (OwnedFd,) =
        proxy.method_call("org.freedesktop.UDisks2.Block", "OpenDevice", ("rw", options))?;

    Ok(unsafe { File::from_raw_fd(res.0.into_fd()) })
}
