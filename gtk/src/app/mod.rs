mod content;
mod dialogs;
mod header;
mod misc;
mod state;

use self::content::{Content, DeviceList};
pub use self::dialogs::OpenDialog;
use self::header::Header;
pub use self::misc::*;
pub use self::state::{Connect, FlashTask, State, FLASHING, KILL, CANCELLED};

// TODO: Use AtomicU64 / Bool when https://github.com/rust-lang/rust/issues/32976 is stable.

use block::BlockDevice;
use flash::FlashRequest;
use gtk;
use gtk::*;
use hash::HashState;
use popsicle::mnt::{self, MountEntry};
use popsicle::{self, DiskError};
use std::path::{Path, PathBuf};
use std::process;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use crossbeam_channel::{Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use std::fs::File;
use std::thread::JoinHandle;

const CSS: &str = include_str!("ui.css");

pub struct App {
    pub widgets: Rc<AppWidgets>,
    pub state:   Arc<State>,
}

impl App {
    pub(crate) fn new(
        hash: Arc<HashState>,
        hash_request: Sender<(PathBuf, &'static str)>,
        devices_request: Sender<(Vec<String>, Vec<MountEntry>)>,
        devices_response: Receiver<Result<Vec<(String, File)>, DiskError>>,
        flash_request: Sender<FlashRequest>,
        flash_response: Receiver<JoinHandle<io::Result<Vec<io::Result<()>>>>>,
    ) -> App {
        // Initialize GTK before proceeding.
        if gtk::init().is_err() {
            eprintln!("failed to initialize GTK Application");
            process::exit(1);
        }


        // Create a the headerbar and it's associated content.
        let header = Header::new();
        // Create the content container and all of it's widgets.
        let content = Content::new();

        // Create a new top level window.
        let window = cascade! {
            Window::new(WindowType::Toplevel);
            // Set the headerbar as the title bar widget.
            ..set_titlebar(&header.container);
            // Set the title of the window.
            ..set_title("Popsicle");
            // Set the window manager class.
            ..set_wmclass("popsicle", "Popsicle");
            // The default size of the window to create.
            ..set_default_size(500, 250);
            // Add the content to the window.
            ..add(&content.container);
        };

        // Add a custom CSS style
        let screen = window.get_screen().unwrap();
        let style = CssProvider::new();
        let _ = CssProviderExt::load_from_data(&style, CSS.as_bytes());
        StyleContext::add_provider_for_screen(&screen, &style, STYLE_PROVIDER_PRIORITY_USER);

        // The icon the app will display.
        Window::set_default_icon_name("iconname");

        // Programs what to do when the exit button is used.
        window.connect_delete_event(move |_, _| {
            main_quit();
            Inhibit(false)
        });

        // Return the application structure.
        App {
            widgets: Rc::new(AppWidgets { window, header, content }),
            state: Arc::new(State::new(hash, hash_request, devices_request, devices_response, flash_request, flash_response)),
        }
    }
}

pub struct AppWidgets {
    pub window:  Window,
    pub header:  Header,
    pub content: Content,
}

impl AppWidgets {
    pub fn set_image(&self, state: &State, image: &Path) {
        let next = self.header.next.clone();
        let image_label = self.content.image_view.image_path.clone();
        let hash_button = self.content.image_view.hash.clone();

        // TODO: Write an error message on failure.
        if let Ok(file) = File::open(image) {
            if let Ok(size) = file.metadata().map(|m| m.len() as usize) {
                image_label.set_text(&image.file_name()
                    .expect("file chooser can't select directories")
                    .to_string_lossy());
                *state.image.write() = Some(image.to_path_buf());
                state.image_length.set(size);
                next.set_sensitive(true);
                hash_button.set_sensitive(true);
            }
        }
    }

    pub fn switch_to_main(&self, state: &State) {
        // If tasks are running, signify that tasks should be considered as completed.
        if FLASHING == state.flash_state.load(Ordering::SeqCst) {
            state.flash_state.store(KILL, Ordering::SeqCst);
        }

        self.content.devices_view.list.select_all.set_active(false);

        cascade! {
            &self.content.container;
            ..set_transition_type(StackTransitionType::SlideRight);
            ..set_visible_child_name("image");
        };

        cascade! {
            &self.header.back;
            ..set_visible(true);
            ..set_label("Cancel");
            ..get_style_context().map(|c| {
                c.remove_class("back-button");
                c.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
            });
        };

        cascade! {
            &self.header.next;
            ..set_visible(true);
            ..set_label("Next");
            ..set_sensitive(true);
            ..get_style_context().map(|c| {
                c.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
                c.add_class(&gtk::STYLE_CLASS_SUGGESTED_ACTION);
            });
        };

        state.view.set(0)
    }

    pub fn switch_to_device_selection(&self, state: Arc<State>) {
        eprintln!("switching to device selection");
        let stack = &self.content.container;
        let back = &self.header.back;
        let next = &self.header.next;
        let list = &self.content.devices_view.list;

        back.set_label("Back");
        back.get_style_context().map(|c| {
            c.add_class("back-button");
        });
        next.set_label("Flash");
        next.get_style_context().map(|c| {
            c.remove_class(&gtk::STYLE_CLASS_SUGGESTED_ACTION);
            c.add_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
        });
        stack.set_visible_child_name("devices");

        let image_sectors = (state.image_length.get() / 512 + 1) as u64;

        let mut devices = vec![];
        if let Err(why) = popsicle::get_disk_args(&mut devices) {
            eprintln!("popsicle: unable to get devices: {}", why);
        }

        if let Err(why) = list.refresh(state.clone(), devices, image_sectors) {
            self.set_error(&state, &why);
        }
    }

    pub fn watch_device_selection(widgets: Rc<AppWidgets>, state: Arc<State>) {
        eprintln!("watching devices");
        gtk::timeout_add(16, move || {
            if state.refreshing_devices.get() {
                return gtk::Continue(true);
            }

            let list = &widgets.content.devices_view.list;
            let next = &widgets.header.next;

            let image_length = state.image_length.get();

            if state.view.get() != 1 {
                eprintln!("stopping device watching");
                return gtk::Continue(false);
            }

            let mut disable_select_all = false;
            let mut error = None;
            let mut devs = None;

            if let Some(ref mut device_list) = state.devices.try_lock() {
                match DeviceList::requires_refresh(&device_list) {
                    Some(devices) => devs = Some(devices),
                    None => {
                        next.set_sensitive(device_list.iter().any(|x| x.1.get_active()));
                    }
                }
            }

            if let Some(devices) = devs {
                let image_sectors = (image_length / 512 + 1) as u64;
                if let Err(why) = list.refresh(state.clone(), devices, image_sectors) {
                    error = Some(why);
                }
                disable_select_all = true;
                next.set_sensitive(false);
            }

            if let Some(why) = error {
                widgets.set_error(&state, &why);
            }

            if disable_select_all {
                list.select_all.set_active(false);
            }

            gtk::Continue(true)
        });
    }

    pub fn switch_to_device_flashing(&self, state: &State) {
        let back = &self.header.back;
        let next = &self.header.next;
        let stack = &self.content.container;
        let summary_grid = &self.content.flash_view.progress_list;
        let task_handles = &state.task_handles;
        let bars = &state.bars;
        let start = &state.start;
        let tasks = &state.tasks;

        macro_rules! try_or_error {
            ($action:expr, $msg:expr) => {{
                match $action {
                    Ok(value) => value,
                    Err(why) => {
                        self.set_error(state, &format!("{}: {}", $msg, why));
                        return;
                    }
                }
            }}
        }

        // Wait for the flash state to be 0 before proceeding.
        while state.flash_state.load(Ordering::SeqCst) != 0 {
            thread::sleep(Duration::from_millis(16));
        }

        {
            let path: PathBuf = state.image.read().as_ref().unwrap().clone();

            let image = try_or_error!(
                 File::open(path),
                 "unable to open source for reading"
            );

            let device_list = state.devices.lock();

            let devs = device_list
                .iter()
                .filter(|x| x.1.get_active())
                .map(|x| x.0.clone())
                .collect::<Vec<_>>();

            let mounts = try_or_error!(
                mnt::get_submounts(Path::new("/")),
                "unable to obtain mount points"
            );

            try_or_error!(
                state.devices_request.send((devs, mounts.clone())),
                "unable to send device request"
            );

            let disks_result = try_or_error!(
                state.devices_response.recv(),
                "unable to get device request response"
            );

            let disks = try_or_error!(
                disks_result,
                "unable to get devices"
            );

            back.get_style_context().map(|c| {
                c.remove_class("back-button");
                c.add_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
            });

            back.set_label("Cancel");
            back.set_visible(true);
            next.set_visible(false);
            stack.set_visible_child_name("flash");

            // Clear the progress bar summaries.
            let mut bars = bars.borrow_mut();
            bars.clear();
            summary_grid.get_children().iter().for_each(|c| c.destroy());

            *start.borrow_mut() = Instant::now();
            let mut tasks = tasks.lock();
            let mut task_handles = task_handles.lock();

            state.flash_state.store(FLASHING, Ordering::SeqCst);

            let mut destinations = Vec::new();

            for (id, (disk_path, mut disk)) in disks.into_iter().enumerate() {
                let id = id as i32;
                let pbar = ProgressBar::new();
                pbar.set_hexpand(true);

                let label = {
                    let disk_path = try_or_error!(
                        Path::new(&disk_path).canonicalize(),
                        format!("unable to get canonical path of {}", disk_path)
                    );
                    if let Some(block) = BlockDevice::new(&disk_path) {
                        gtk::Label::new(
                            [&block.label(), " (", &disk_path.to_string_lossy(), ")"]
                                .concat()
                                .as_str(),
                        )
                    } else {
                        gtk::Label::new(disk_path.to_string_lossy().as_ref())
                    }
                };

                label.set_justify(gtk::Justification::Right);
                label.get_style_context().map(|c| c.add_class("bold"));
                let bar_label = gtk::Label::new("");
                bar_label.set_halign(gtk::Align::Center);

                let bar_container = cascade! {
                    gtk::Box::new(Orientation::Vertical, 0);
                    ..pack_start(&pbar, false, false, 0);
                    ..pack_start(&bar_label, false, false, 0);
                };

                summary_grid.attach(&label, 0, id, 1, 1);
                summary_grid.attach(&bar_container, 1, id, 1, 1);
                bars.push((pbar, bar_label));

                destinations.push(disk);
            }

            let ndestinations = destinations.len();
            let progress = Arc::new((0..ndestinations).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>());
            let finished = Arc::new((0..ndestinations).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>());

            // Spawn a thread that will update the progress value over time.
            //
            // The value will be stored within an intermediary atomic integer,
            // because it is unsafe to send GTK widgets across threads.
            *task_handles = {
                let _ = state.flash_request.send(FlashRequest::new(
                    image,
                    destinations,
                    state.flash_state.clone(),
                    progress.clone(),
                    finished.clone()
                ));

                Some(state.flash_response.recv().expect("expected join handle to be returned"))
            };

            *tasks = Some(FlashTask {
                previous: Arc::new(Mutex::new(vec![[0; 7]; ndestinations])),
                progress,
                finished,
            });
        }

        summary_grid.show_all();
    }

    pub fn switch_to_summary(&self, state: &State, ntasks: usize) -> Result<(), ()> {
        let stack = &self.content.container;
        let back = &self.header.back;
        let next = &self.header.next;
        let description = &self.content.summary_view.view.description.clone();
        let list = &self.content.summary_view.list.clone();
        let devices = &state.devices;
        let task_handles = &state.task_handles;

        back.set_label("Flash Again");
        back.get_style_context()
            .map(|c| c.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION));

        next.set_label("Done");
        next.set_visible(true);
        next.get_style_context()
            .map(|c| c.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION));

        stack.set_visible_child_name("summary");

        macro_rules! try_or_error {
            ($action:expr, $msg:expr) => {{
                match $action {
                    Ok(value) => value,
                    Err(why) => {
                        self.set_error(state, &format!("{}: {:?}", $msg, why));
                        return Err(());
                    }
                }
            }}
        }

        {
            let mut errored: Vec<(String, io::Error)> = Vec::new();

            let mut handle = task_handles.lock();

            let results = try_or_error!(
                handle.take().unwrap().join(),
                "failed to join flashing thread"
            );

            let device_results = try_or_error!(
                results,
                "main flashing process failed"
            );

            let mut devices = devices.lock();

            for ((device, _), result) in devices.drain(..).zip(device_results.into_iter()) {
                if let Err(why) = result {
                    errored.push((device.clone(), why));
                }
            }

            if errored.is_empty() {
                description.set_text(&format!("{} devices successfully flashed", ntasks));
                list.set_visible(false);
            } else {
                description.set_text(&format!(
                    "{} of {} devices successfully flashed",
                    ntasks - errored.len(),
                    ntasks
                ));
                list.set_visible(true);
                for (device, why) in errored {
                    let device = Label::new(device.as_str());
                    let why = Label::new(format!("{}", why).as_str());
                    let container = cascade! {
                        Box::new(Orientation::Horizontal, 0);
                        ..pack_start(&device, false, false, 0);
                        ..pack_start(&why, true, true, 0);
                    };
                    list.insert(&container, -1);
                }
            }
        }

        state.reset();

        Ok(())
    }

    pub fn set_error(&self, state: &State, msg: &str) {
        let stack = &self.content.container;
        let back = &self.header.back;
        let error = &self.content.error_view.view.description;

        back.set_visible(false);
        cascade! {
            &self.header.next;
            ..set_visible(true);
            ..set_label("Close");
            ..get_style_context().map(|c| {
                c.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
                c.remove_class(&gtk::STYLE_CLASS_SUGGESTED_ACTION);
            });
        };
        error.set_text(&msg);
        stack.set_visible_child_name("error");
        state.view.set(2);
        state.reset();
    }
}
