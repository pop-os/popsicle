use flash::FlashRequest;

use super::{App, OpenDialog};
use app::{misc, AppWidgets};
use hash::HashState;
use std::io;
use std::fs::File;
use std::cell::{Cell, RefCell};
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::Instant;

use gtk;
use gtk::*;
use popsicle::mnt::MountEntry;
use popsicle::DiskError;

pub const FLASHING: usize = 1;
pub const KILL: usize = 2;
pub const CANCELLED: usize = 3;

/// Contains all of the state that needs to be shared across the program's lifetime.
pub struct State {
    /// Contains all of the progress bars in the flash view.
    pub bars: RefCell<Vec<(ProgressBar, Label)>>,
    /// Contains a list of devices detected, and their check buttons.
    pub devices: Mutex<Vec<(String, CheckButton)>>,
    /// Manages the state of hash requests
    pub(crate) hash: Arc<HashState>,
    /// Requests the background thread to generate a new hash.
    pub(crate) hash_request: Sender<(PathBuf, &'static str)>,
    /// Points to the location of the image to be flashed.
    pub image: RwLock<Option<(PathBuf, usize)>>,
    /// Stores the time when the flashing process began.
    pub start: RefCell<Instant>,
    /// Holds the task threads that write the image to each device.
    /// The handles may contain errors when joined, for printing on the summary page.
    pub task_handles: Mutex<Option<JoinHandle<io::Result<Vec<io::Result<()>>>>>>,
    /// Contains progress data regarding each active flash task -- namely the progress.
    pub tasks: Mutex<Option<FlashTask>>,
    /// Stores an integer which defines the currently-active view.
    pub view: Cell<u8>,
    /// Requests for a list of devices to be returned by an authenticated user (ie: root).
    pub devices_request: Sender<(Vec<String>, Vec<MountEntry>)>,
    /// The accompanying response that follows a device request.
    pub devices_response: Receiver<Result<Vec<(String, File)>, DiskError>>,
    /// Requests for a device to be flashed by an authenticated user (ie: root).
    pub flash_request: Sender<FlashRequest>,
    /// Contains the join handle to the thread where the task is being flashed.
    pub flash_response: Receiver<JoinHandle<io::Result<Vec<io::Result<()>>>>>,
    /// Signifies the status of flashing
    pub flash_state: Arc<AtomicUsize>,
}

impl State {
    /// Initailizes a new structure for managing the state of the application.
    pub(crate) fn new(
        hash: Arc<HashState>,
        hash_request: Sender<(PathBuf, &'static str)>,
        devices_request: Sender<(Vec<String>, Vec<MountEntry>)>,
        devices_response: Receiver<Result<Vec<(String, File)>, DiskError>>,
        flash_request: Sender<FlashRequest>,
        flash_response: Receiver<JoinHandle<io::Result<Vec<io::Result<()>>>>>,
    ) -> State {
        State {
            bars: RefCell::new(Vec::new()),
            devices: Mutex::new(Vec::new()),
            task_handles: Mutex::new(None),
            tasks: Mutex::new(None),
            view: Cell::new(0),
            start: RefCell::new(unsafe { mem::uninitialized() }),
            flash_state: Arc::new(AtomicUsize::new(0)),
            hash,
            hash_request,
            image: RwLock::new(None),
            devices_request,
            devices_response,
            flash_request,
            flash_response,
        }
    }

    pub fn reset(&self) {
        self.bars.borrow_mut().clear();
        self.devices.lock().unwrap().clear();
        *self.tasks.lock().unwrap() = None;
        self.flash_state.store(0, Ordering::SeqCst);
    }
}

pub struct FlashTask {
    pub progress: Arc<Vec<AtomicUsize>>,
    pub previous: Arc<Mutex<Vec<[usize; 7]>>>,
    pub finished: Arc<Vec<AtomicUsize>>,
}


pub struct Connected(App);

impl Connected {
    /// Display the window, and execute the gtk main event loop.
    pub fn then_execute(self) {
        self.0.widgets.window.show_all();
        gtk::main();
    }
}

pub trait Connect {
    /// Creates external state, and maps all of the UI functionality to the UI.
    fn connect_events(self) -> Connected;

    /// Programs the button for selecting an image.
    fn connect_image_chooser(&self);

    /// Sets the image via a drag and drop.
    fn connect_image_drag_and_drop(&self);

    /// Programs the combo box which generates the hash sum for initial image selection view.
    fn connect_hash_generator(&self);

    /// Programs the back button, whose behavior changes based on the currently active view.
    fn connect_back_button(&self);

    /// Programs the next button, whose behavior changes based on the currently active view.
    fn connect_next_button(&self);

    /// Programs the action that will be performed when the check all button is clicked.
    fn connect_check_all(&self);

    /// Adds a function for GTK to execute when the application is idle, to monitor and
    /// update the progress bars for devices that are being flashed, and to generate
    /// the summary view after all devices have been flashed.
    fn watch_flashing_devices(&self);
}

impl Connect for App {
    fn connect_events(self) -> Connected {
        self.connect_image_chooser();
        self.connect_image_drag_and_drop();
        self.connect_hash_generator();
        self.connect_back_button();
        self.connect_next_button();
        self.connect_check_all();
        self.watch_flashing_devices();

        Connected(self)
    }

    fn connect_image_chooser(&self) {
        let state = self.state.clone();
        let widgets = self.widgets.clone();
        self.widgets.content.image_view.chooser.connect_clicked(move |_| {
            if let Some(path) = OpenDialog::new(None).run() {
                widgets.set_image(&state, &path);
            }
        });
    }

    fn connect_image_drag_and_drop(&self) {
        let state = self.state.clone();
        let widgets = self.widgets.clone();
        let image_view = widgets.content.image_view.view.container.clone();

        misc::drag_and_drop(&image_view, move |data| {
            if let Some(uri) = data.get_text() {
                if uri.starts_with("file://") {
                    let path = Path::new(&uri[7..uri.len() - 1]);
                    if path.extension().map_or(false, |ext| ext == "iso" || ext == "img") && path.exists() {
                        widgets.set_image(&state, path);
                    }
                }
            }
        });
    }

    fn connect_hash_generator(&self) {
        let state = self.state.clone();
        let hash_label = self.widgets.content.image_view.hash_label.clone();
        let chooser = self.widgets.content.image_view.chooser_container.clone();

        self.widgets.content
            .image_view
            .hash
            .connect_changed(move |hash_kind| {
                if let Some((ref path, _)) = *state.image.read().unwrap() {
                    let hash_kind = match hash_kind.get_active() {
                        1 => Some("SHA256"),
                        2 => Some("MD5"),
                        _ => None,
                    };

                    if let Some(hash_kind) = hash_kind {
                        let hash = state.hash.clone();

                        let _ = state.hash_request.send((path.clone(), hash_kind));

                        let hash_label = hash_label.clone();
                        let path = path.clone();
                        let chooser = chooser.clone();
                        gtk::timeout_add(16, move || {
                            match hash.try_obtain(&path, hash_kind) {
                                Some(hash) => {
                                    hash_label.set_text(&hash);
                                    chooser.set_visible_child_name("chooser");
                                    Continue(false)
                                }
                                None => {
                                    chooser.set_visible_child_name("checksum");
                                    Continue(true)
                                }
                            }
                        });
                    }
                }
            });
    }

    fn connect_back_button(&self) {
        let widgets = self.widgets.clone();
        let state = self.state.clone();
        self.widgets.header.back.connect_clicked(move |_| {
            match state.view.get() {
                0 => {
                    gtk::main_quit();
                    return;
                },
                _ => widgets.switch_to_main(&state),
            }
        });
    }

    fn connect_next_button(&self) {
        #[allow(unused_variables)]
        let widgets = self.widgets.clone();
        let next = widgets.header.next.clone();
        let stack = widgets.content.container.clone();
        let state = self.state.clone();

        next.connect_clicked(move |_| {
            let view = &state.view;
            let view_value = view.get();
            stack.set_transition_type(StackTransitionType::SlideLeft);

            match view_value {
                0 => widgets.switch_to_device_selection(&state),
                1 => widgets.switch_to_device_flashing(&state),
                2 => gtk::main_quit(),
                _ => unreachable!(),
            }

            view.set(view_value + 1);

            if view.get() == 1 {
                AppWidgets::watch_device_selection(widgets.clone(), state.clone());
            }
        });
    }

    fn connect_check_all(&self) {
        let state = self.state.clone();
        let widgets = self.widgets.clone();
        widgets.clone().content.devices_view.list.connect_select_all(
            state.clone(),
            move |result| if let Err(why) = result {
                widgets.set_error(&state, &format!("select all failed: {}", why));
            }
        )
    }

    fn watch_flashing_devices(&self) {
        let state = self.state.clone();
        let widgets = self.widgets.clone();

        gtk::timeout_add(500, move || {
            let tasks = &state.tasks;
            let bars = &state.bars;

            macro_rules! try_or_error {
                ($action:expr, $msg:expr, $val:expr) => {{
                    match $action {
                        Ok(value) => value,
                        Err(why) => {
                            widgets.set_error(&state, &format!("{}: {:?}", $msg, why));
                            return $val;
                        }
                    }
                }}
            }

            let mut all_tasks_finished = true;
            let ntasks;

            {
                // Ensure that an image has been selected before continuing
                let length = match *state.image.read().unwrap() {
                    Some(ref image) => image.1,
                    None => {
                        return Continue(true);
                    }
                };

                let tasks = try_or_error!(
                    tasks.lock(),
                    "tasks mutex lock failure",
                    Continue(false)
                );

                let tasks = match tasks.as_ref() {
                    Some(tasks) => tasks,
                    None => return Continue(true),
                };

                ntasks = tasks.progress.len();

                let mut previous = try_or_error!(
                    tasks.previous.lock(),
                    "tasks.previous mutex lock failure",
                    Continue(false)
                );

                for (id, &(ref pbar, ref label)) in bars.borrow().iter().enumerate() {
                    let prev_values = &mut previous[id];
                    let progress = &tasks.progress[id];
                    let finished = &tasks.finished[id];

                    let raw_value = progress.load(Ordering::SeqCst);
                    let task_is_finished = finished.load(Ordering::SeqCst) == 1;
                    let value = if task_is_finished {
                        1.0f64
                    } else {
                        all_tasks_finished = false;
                        raw_value as f64 / length as f64
                    };

                    pbar.set_fraction(value);

                    if task_is_finished {
                        label.set_label("Complete");
                    } else {
                        prev_values[1] = prev_values[2];
                        prev_values[2] = prev_values[3];
                        prev_values[3] = prev_values[4];
                        prev_values[4] = prev_values[5];
                        prev_values[5] = prev_values[6];
                        prev_values[6] = raw_value - prev_values[0];
                        prev_values[0] = raw_value;

                        let sum: usize = prev_values.iter().skip(1).sum();
                        let per_second = sum / 3;
                        label.set_label(&if per_second > (1024 * 1024) {
                            format!("{} MiB/s", per_second / (1024 * 1024))
                        } else {
                            format!("{} KiB/s", per_second / 1024)
                        });
                    }
                }
            }

            match state.flash_state.load(Ordering::SeqCst) {
                stat if stat == CANCELLED => state.reset(),
                stat if stat == FLASHING => if all_tasks_finished && widgets.switch_to_summary(&state, ntasks).is_err() {
                    return Continue(false);
                },
                _ => ()
            }

            Continue(true)
        });
    }
}
