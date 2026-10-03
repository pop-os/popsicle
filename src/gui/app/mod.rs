pub mod events;
mod views;

use self::events::FlashResult;
use crate::fl;
use crate::gui::flash::{FlashError, FlashRequest, FlashStatus, FlashTask};
use crate::gui::misc;

use atomic::Atomic;
use cosmic::app::{Core, Task};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::{Subscription, event, time, window};
use cosmic::{ApplicationExt, Element};
use dbus_udisks2::DiskDevice;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveView {
    Images,
    Devices,
    Flashing,
    Summary,
    Error,
}

#[derive(Clone, Debug)]
pub enum Message {
    Back,
    Next,
    ChooseImage,
    ImageChosen(Option<PathBuf>),
    ImagesDropped(Vec<PathBuf>),
    FileTransfer(String),
    ImageInspected(PathBuf, Result<(u64, bool), String>),
    HashKind(usize),
    HashInput(String),
    CheckHash,
    HashResult(PathBuf, &'static str, Result<String, String>),
    RefreshDevices,
    DevicesRefreshed(Option<Box<[Arc<DiskDevice>]>>),
    ToggleDevice(usize, bool),
    SelectAll(bool),
    FlashTick,
}

/// The image selected for flashing.
#[derive(Default)]
pub struct Image {
    pub path: Option<PathBuf>,
    pub size: u64,
    pub warning: Option<String>,
}

/// State of the checksum widgets on the image view.
pub struct Hash {
    /// Dropdown entries: the localized "None" followed by [`events::HASH_KINDS`].
    pub kinds: Vec<String>,
    pub selected: usize,
    pub input: String,
    pub busy: bool,
    /// Whether the computed checksum matched the one entered by the user.
    pub matches: Option<bool>,
    cache: HashMap<(PathBuf, &'static str), String>,
}

impl Hash {
    fn kind(&self) -> Option<&'static str> {
        self.selected.checked_sub(1).and_then(|id| events::HASH_KINDS.get(id).copied())
    }

    /// Fills the entry with the checksum, or compares it against the entry when one was given.
    fn set_result(&mut self, hash: &str) {
        if self.input.is_empty() {
            hash.clone_into(&mut self.input);
        } else {
            self.matches = Some(self.input.trim().eq_ignore_ascii_case(hash));
        }
    }
}

/// The USB devices available for flashing, and which of them were selected.
#[derive(Default)]
pub struct Devices {
    pub list: Box<[Arc<DiskDevice>]>,
    pub selected: Vec<bool>,
    pub select_all: bool,
    refreshing: bool,
}

impl Devices {
    /// Replaces the device list when it changed, keeping the selection of devices still present.
    fn update(&mut self, devices: Box<[Arc<DiskDevice>]>) {
        let unchanged = devices.len() == self.list.len()
            && devices.iter().zip(&self.list).all(|(a, b)| a.drive.path == b.drive.path);
        if unchanged {
            return;
        }

        self.selected = devices
            .iter()
            .map(|device| {
                self.list
                    .iter()
                    .position(|d| d.drive.path == device.drive.path)
                    .is_some_and(|id| self.selected[id])
            })
            .collect();
        self.list = devices;
    }

    fn reset(&mut self) {
        self.select_all = false;
        self.selected.iter_mut().for_each(|s| *s = false);
    }

    pub fn any_selected(&self) -> bool {
        self.selected.iter().any(|s| *s)
    }
}

/// A flash job in progress.
pub struct Flash {
    handle: Option<JoinHandle<FlashResult>>,
    task: FlashTask,
    devices: Vec<Arc<DiskDevice>>,
    pub rows: Vec<FlashRow>,
}

/// Progress of one device being flashed.
pub struct FlashRow {
    pub label: String,
    pub fraction: f32,
    pub status: String,
}

#[derive(Default)]
pub struct Summary {
    pub topic: String,
    pub description: String,
    pub errors: Vec<(String, String)>,
}

pub struct App {
    core: Core,
    view: ActiveView,
    icons: views::Icons,
    image: Image,
    hash: Hash,
    devices: Devices,
    flash: Option<Flash>,
    flash_status: Arc<Atomic<FlashStatus>>,
    summary: Summary,
    error: String,
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = Option<PathBuf>;
    type Message = Message;

    const APP_ID: &'static str = "com.system76.Popsicle";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, image: Option<PathBuf>) -> (Self, Task<Message>) {
        let mut kinds = vec![fl!("none")];
        kinds.extend(events::HASH_KINDS.iter().map(|kind| (*kind).to_owned()));

        let mut app = App {
            core,
            view: ActiveView::Images,
            icons: views::Icons::load(),
            image: Image::default(),
            hash: Hash {
                kinds,
                selected: 0,
                input: String::new(),
                busy: false,
                matches: None,
                cache: HashMap::new(),
            },
            devices: Devices::default(),
            flash: None,
            flash_status: Arc::new(Atomic::new(FlashStatus::Inactive)),
            summary: Summary::default(),
            error: String::new(),
        };

        app.set_header_title(fl!("app-title"));

        let mut tasks = Vec::new();
        if let Some(id) = app.core.main_window_id() {
            tasks.push(app.set_window_title(fl!("app-name"), id));
        }
        if let Some(path) = image {
            tasks.push(select_image(path));
        }

        (app, Task::batch(tasks))
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        vec![views::back_button(self.view)]
    }

    fn header_end(&self) -> Vec<Element<'_, Message>> {
        views::next_button(self).into_iter().collect()
    }

    fn view(&self) -> Element<'_, Message> {
        match self.view {
            ActiveView::Images => views::images(self),
            ActiveView::Devices => views::devices(self),
            ActiveView::Flashing => views::flashing(self),
            ActiveView::Summary => views::summary(self),
            ActiveView::Error => views::error(self),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let files = event::listen_with(|event, _status, _id| match event {
            event::Event::Window(window::Event::FileDropped(paths)) => {
                Some(Message::ImagesDropped(paths))
            }
            _ => None,
        });

        let polling = match self.view {
            ActiveView::Devices => {
                time::every(Duration::from_secs(3)).map(|_| Message::RefreshDevices)
            }
            ActiveView::Flashing => {
                time::every(Duration::from_millis(500)).map(|_| Message::FlashTick)
            }
            _ => Subscription::none(),
        };

        Subscription::batch([files, polling])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Back => {
                if self.view == ActiveView::Images {
                    return quit();
                }

                self.reset();
                self.view = ActiveView::Images;
            }
            Message::Next => match self.view {
                ActiveView::Images => {
                    self.view = ActiveView::Devices;
                    return self.refresh_devices();
                }
                ActiveView::Devices => self.start_flash(),
                _ => return quit(),
            },
            Message::ChooseImage => return choose_image(),
            Message::ImageChosen(Some(path)) => return select_image(path),
            Message::ImageChosen(None) => (),
            Message::ImagesDropped(paths) => {
                if let Some(path) = paths.into_iter().find(|path| misc::is_image(path)) {
                    return select_image(path);
                }
            }
            Message::FileTransfer(key) => {
                return cosmic::command::file_transfer_receive(key).map(|result| {
                    let paths = result
                        .map_err(|why| eprintln!("failed to receive dropped files: {why}"))
                        .unwrap_or_default();
                    cosmic::Action::App(Message::ImagesDropped(
                        paths.into_iter().map(PathBuf::from).collect(),
                    ))
                });
            }
            // Images chosen or dropped while another view is active are ignored.
            Message::ImageInspected(_, _) if self.view != ActiveView::Images => (),
            Message::ImageInspected(path, Ok((size, windows))) => {
                let warning = windows.then(|| fl!("win-isos-not-supported"));
                self.image = Image { path: Some(path), size, warning };
                self.hash.matches = None;
                return self.check_hash();
            }
            Message::ImageInspected(path, Err(why)) => {
                eprintln!("failed to open {}: {}", path.display(), why);
            }
            Message::HashKind(kind) => {
                if !self.hash.busy {
                    self.hash.selected = kind;
                    self.hash.matches = None;
                }
            }
            Message::HashInput(input) => {
                self.hash.input = input;
                self.hash.matches = None;
            }
            Message::CheckHash => return self.check_hash(),
            Message::HashResult(path, kind, result) => {
                self.hash.busy = false;
                let hash = match result {
                    Ok(hash) => {
                        self.hash.cache.insert((path, kind), hash.clone());
                        hash
                    }
                    Err(why) => fl!("error", why = why),
                };
                self.hash.set_result(&hash);
            }
            Message::RefreshDevices => return self.refresh_devices(),
            Message::DevicesRefreshed(devices) => {
                self.devices.refreshing = false;
                if let Some(devices) = devices {
                    self.devices.update(devices);
                }
            }
            Message::ToggleDevice(id, checked) => {
                if let Some(selected) = self.devices.selected.get_mut(id) {
                    *selected = checked;
                }
                if !checked {
                    self.devices.select_all = false;
                }
            }
            Message::SelectAll(checked) => {
                self.devices.select_all = checked;
                let size = self.image.size;
                for (device, selected) in self.devices.list.iter().zip(&mut self.devices.selected) {
                    *selected = checked && device.parent.size >= size;
                }
            }
            Message::FlashTick => self.poll_flash(),
        }

        Task::none()
    }
}

impl App {
    pub fn view_kind(&self) -> ActiveView {
        self.view
    }

    pub fn icons(&self) -> &views::Icons {
        &self.icons
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    pub fn devices(&self) -> &Devices {
        &self.devices
    }

    pub fn flash_rows(&self) -> &[FlashRow] {
        self.flash.as_ref().map_or(&[], |flash| flash.rows.as_slice())
    }

    pub fn summary(&self) -> &Summary {
        &self.summary
    }

    pub fn error(&self) -> &str {
        &self.error
    }

    /// Abandons any flash in progress and clears the device selection.
    fn reset(&mut self) {
        if self.flash_status.load(Ordering::SeqCst) == FlashStatus::Active {
            self.flash_status.store(FlashStatus::Killing, Ordering::SeqCst);
        }

        self.flash = None;
        self.devices.reset();
    }

    /// Switches to the error view with a description of what failed.
    fn fail(&mut self, context: &str, why: impl Display) {
        self.error = format!("{context}: {why}");
        self.view = ActiveView::Error;
    }

    /// Computes the selected checksum of the image, reusing a cached result if there is one.
    fn check_hash(&mut self) -> Task<Message> {
        let (Some(path), Some(kind)) = (self.image.path.clone(), self.hash.kind()) else {
            return Task::none();
        };

        if let Some(hash) = self.hash.cache.get(&(path.clone(), kind)) {
            let hash = hash.clone();
            self.hash.set_result(&hash);
            return Task::none();
        }

        self.hash.busy = true;
        cosmic::task::future(async move {
            let hashed = path.clone();
            let result = events::blocking(move || events::hash(&hashed, kind)).await;
            Message::HashResult(path, kind, result.map_err(|why| why.to_string()))
        })
    }

    fn refresh_devices(&mut self) -> Task<Message> {
        if self.devices.refreshing {
            return Task::none();
        }

        self.devices.refreshing = true;
        cosmic::task::future(async {
            let result = events::blocking(events::refresh_devices).await;
            Message::DevicesRefreshed(
                result.map_err(|why| eprintln!("failed to refresh devices: {why}")).ok(),
            )
        })
    }

    /// Opens the image and starts writing it to the selected devices.
    fn start_flash(&mut self) {
        let path = self.image.path.clone().unwrap_or_default();
        let image = match File::open(&path) {
            Ok(image) => image,
            Err(why) => return self.fail(&fl!("iso-open-failed"), why),
        };

        let devices: Vec<Arc<DiskDevice>> = self
            .devices
            .list
            .iter()
            .zip(&self.devices.selected)
            .filter(|(_, selected)| **selected)
            .map(|(device, _)| device.clone())
            .collect();

        let ndestinations = devices.len();
        let progress = Arc::new((0..ndestinations).map(|_| Atomic::new(0u64)).collect::<Vec<_>>());
        let finished = Arc::new((0..ndestinations).map(|_| Atomic::new(false)).collect::<Vec<_>>());

        let request = FlashRequest::new(
            image,
            devices.clone(),
            self.flash_status.clone(),
            progress.clone(),
            finished.clone(),
        );

        let handle = match events::spawn_flash(request) {
            Ok(handle) => handle,
            Err(why) => return self.fail("Failed to spawn flash thread", why),
        };

        let rows = devices
            .iter()
            .map(|device| FlashRow {
                label: misc::device_label(device),
                fraction: 0.0,
                status: String::new(),
            })
            .collect();

        self.flash = Some(Flash {
            handle: Some(handle),
            task: FlashTask {
                previous: Arc::new(Mutex::new(vec![[0; 7]; ndestinations])),
                progress,
                finished,
            },
            devices,
            rows,
        });
        self.view = ActiveView::Flashing;
    }

    /// Updates the progress of every device, then summarizes once all of them are done.
    #[allow(clippy::cast_precision_loss)]
    fn poll_flash(&mut self) {
        let Some(flash) = self.flash.as_mut() else { return };

        let length = self.image.size;
        let mut all_tasks_finished = true;
        let mut previous = flash.task.previous.lock().expect("mutex lock");

        for (id, row) in flash.rows.iter_mut().enumerate() {
            let raw_value = flash.task.progress[id].load(Ordering::SeqCst);

            if flash.task.finished[id].load(Ordering::SeqCst) {
                row.fraction = 1.0;
                row.status = fl!("task-finished");
                continue;
            }

            all_tasks_finished = false;
            row.fraction = if length == 0 { 0.0 } else { raw_value as f32 / length as f32 };

            // Average the bytes written over the last six polls (three seconds).
            let prev_values = &mut previous[id];
            prev_values.copy_within(2..7, 1);
            prev_values[6] = raw_value.saturating_sub(prev_values[0]);
            prev_values[0] = raw_value;

            let sum: u64 = prev_values.iter().skip(1).sum();
            row.status = format!("{}/s", bytesize::to_string(sum / 3, true));
        }

        drop(previous);

        if all_tasks_finished {
            self.finish_flash();
        }
    }

    /// Collects the results of the finished flash and shows the summary view.
    fn finish_flash(&mut self) {
        let Some(mut flash) = self.flash.take() else { return };

        let Some(handle) = flash.handle.take() else {
            return self.fail("Taking flash handles failed", fl!("no-value-found"));
        };

        let result = match handle.join() {
            Ok(result) => result,
            Err(why) => return self.fail("Failed to join flash thread", format!("{why:?}")),
        };

        let (result, results) = match result {
            Ok(result) => result,
            Err(why) => return self.fail("Errored starting flashing process", why),
        };

        let ntasks = flash.devices.len();
        let errors: Vec<(String, FlashError)> = flash
            .devices
            .iter()
            .zip(results)
            .filter_map(|(device, result)| {
                result.err().map(|why| (misc::device_label(device), why))
            })
            .collect();

        self.summary = if result.is_ok() && errors.is_empty() {
            Summary {
                topic: fl!("flashing-completed"),
                description: fl!("successful-flash", total = ntasks),
                errors: Vec::new(),
            }
        } else {
            let number = ntasks - errors.len();
            let mut description = fl!("partial-flash", number = number, total = ntasks);

            if let Err(why) = result {
                description = format!("{description}: {why}");
            }

            Summary {
                topic: fl!("flashing-completed-with-errors"),
                description,
                errors: errors.into_iter().map(|(device, why)| (device, why.to_string())).collect(),
            }
        };

        self.view = ActiveView::Summary;
    }
}

fn quit() -> Task<Message> {
    cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Close))
}

/// Inspects the image on a background thread before showing it as selected.
fn select_image(path: PathBuf) -> Task<Message> {
    cosmic::task::future(async move {
        let inspect = path.clone();
        let result = events::blocking(move || events::inspect_image(&inspect)).await;
        Message::ImageInspected(path, result.map_err(|why| why.to_string()))
    })
}

/// Asks the desktop portal for an image to flash.
fn choose_image() -> Task<Message> {
    cosmic::task::future(async {
        let dialog = file_chooser::open::Dialog::new()
            .title(fl!("open"))
            .accept_label(fl!("open"))
            .filter(FileFilter::new("ISO / IMG").glob("*.[Ii][Ss][Oo]").glob("*.[Ii][Mm][Gg]"));

        match dialog.open_file().await {
            Ok(response) => Message::ImageChosen(response.url().to_file_path().ok()),
            Err(file_chooser::Error::Cancelled) => Message::ImageChosen(None),
            Err(why) => {
                eprintln!("failed to open file chooser: {why}");
                Message::ImageChosen(None)
            }
        }
    })
}
