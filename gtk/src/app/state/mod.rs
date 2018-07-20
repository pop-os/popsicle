#[macro_use]
mod try;

use flash::FlashRequest;

use super::{App, OpenDialog};
use app::AppWidgets;

use hash::HashState;
use image::{self, BufferingData};
use std::fs::File;
use std::cell::{Cell, RefCell};
use std::mem;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use gtk;
use gtk::*;
use popsicle::mnt::MountEntry;
use popsicle::DiskError;

/// Contains all of the state that needs to be shared across the program's lifetime.
pub struct State {
    /// Contains all of the progress bars in the flash view.
    pub bars: RefCell<Vec<(ProgressBar, Label)>>,
    /// Contains the disk image that is loaded into memory and shared across threads.
    pub buffer: Arc<BufferingData>,
    /// Contains a list of devices detected, and their check buttons.
    pub devices: Mutex<Vec<(String, CheckButton)>>,
    /// Manages the state of hash requests
    pub(crate) hash: Arc<HashState>,
    /// Useful for storing the size of the image that was loaded.
    pub image_length: Cell<usize>,
    /// Signals to load a new disk image into the `buffer` field.
    pub image_sender: Sender<PathBuf>,
    /// Stores the time when the flashing process began.
    pub start: RefCell<Instant>,
    /// Holds the task threads that write the image to each device.
    /// The handles may contain errors when joined, for printing on the summary page.
    pub task_handles: Mutex<Vec<JoinHandle<Result<(), DiskError>>>>,
    /// Contains progress data regarding each active flash task -- namely the progress.
    pub tasks: Mutex<Vec<FlashTask>>,
    /// Stores an integer which defines the currently-active view.
    pub view: Cell<u8>,
    /// Requests for a list of devices to be returned by an authenticated user (ie: root).
    pub devices_request: Sender<(Vec<String>, Vec<MountEntry>)>,
    /// The accompanying response that follows a device request.
    pub devices_response: Receiver<Result<Vec<(String, File)>, DiskError>>,
    /// Requests for a device to be flashed by an authenticated user (ie: root).
    pub flash_request: Sender<FlashRequest>,
    /// Contains the join handle to the thread where the task is being flashed.
    pub flash_response: Receiver<JoinHandle<Result<(), DiskError>>>,
    /// Signifies the status of flashing
    pub flash_state: Arc<AtomicUsize>,
}

impl State {
    /// Initailizes a new structure for managing the state of the application.
    pub fn new(
        image_sender: Sender<PathBuf>,
        devices_request: Sender<(Vec<String>, Vec<MountEntry>)>,
        devices_response: Receiver<Result<Vec<(String, File)>, DiskError>>,
        flash_request: Sender<FlashRequest>,
        flash_response: Receiver<JoinHandle<Result<(), DiskError>>>,
    ) -> State {
        State {
            bars: RefCell::new(Vec::new()),
            devices: Mutex::new(Vec::new()),
            task_handles: Mutex::new(Vec::new()),
            tasks: Mutex::new(Vec::new()),
            view: Cell::new(0),
            start: RefCell::new(unsafe { mem::uninitialized() }),
            flash_state: Arc::new(AtomicUsize::new(0)),
            hash: Arc::new(HashState::new()),
            buffer: Arc::new(BufferingData::new()),
            image_sender,
            image_length: Cell::new(0),
            devices_request,
            devices_response,
            flash_request,
            flash_response,
        }
    }

    pub fn set_as_complete(&self) {
        self.flash_state.store(2, Ordering::SeqCst);
        while 3 != self.flash_state.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(16));
        }
    }

    pub fn reset_after_reacquiring_image(&self) {
        if 3 == self.flash_state.load(Ordering::SeqCst) {
            self.flash_state.store(4, Ordering::SeqCst);
            while 5 != self.flash_state.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(16));
            }
            self.reset();
        }
    }

    fn reset(&self) {
        self.bars.borrow_mut().clear();
        if let Ok(ref mut devices) = self.devices.lock() {
            devices.clear();
        }

        if let Ok(ref mut handles) = self.task_handles.lock() {
            if !handles.is_empty() {
                handles.drain(..).for_each(|x| { let _ = x.join(); });
            }
        }

        if let Ok(ref mut tasks) = self.tasks.lock() {
            tasks.clear();
        }

        self.flash_state.store(0, Ordering::SeqCst);
    }

    pub fn async_reattain_image_data(&self, image_data: Arc<Vec<u8>>) {
        let buffer = self.buffer.clone();
        let flash_state = self.flash_state.clone();
        thread::spawn(move || loop {
            if flash_state.load(Ordering::SeqCst) == 2 {
                flash_state.store(3, Ordering::SeqCst);

                // Wait for the main GTK event loop to sleep so that we have exclusive state lock access.
                while 4 != flash_state.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(16))
                }

                // This will be 1 once the device flashing threads have exited.
                while 1 != Arc::strong_count(&image_data) {
                    thread::sleep(Duration::from_millis(16));
                }

                let (_, ref mut data) = *buffer.data.lock().expect("failed to get lock on buffer.data");
                let mut replace_with = Arc::try_unwrap(image_data).expect("image_data is still shared");
                mem::swap(data, &mut replace_with);

                flash_state.store(5, Ordering::SeqCst);
                break
            }

            thread::sleep(Duration::from_millis(16));
        });
    }
}

pub struct FlashTask {
    pub progress: Arc<AtomicUsize>,
    pub previous: Arc<Mutex<[usize; 7]>>,
    pub finished: Arc<AtomicUsize>,
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
        self.connect_hash_generator();
        self.connect_back_button();
        self.connect_next_button();
        self.connect_check_all();
        self.watch_flashing_devices();

        Connected(self)
    }

    fn connect_image_chooser(&self) {
        let state = self.state.clone();
        self.widgets.content.image_view.chooser.connect_clicked(move |_| {
            if let Some(path) = OpenDialog::new(None).run() {
                let _ = state.image_sender.send(path);
            }
        });
    }

    fn connect_hash_generator(&self) {
        let state = self.state.clone();
        let hash_label = self.widgets.content.image_view.hash_label.clone();
        self.widgets.content
            .image_view
            .hash
            .connect_changed(move |hash_kind| {
                let hash_kind = match hash_kind.get_active() {
                    1 => Some("SHA256"),
                    2 => Some("MD5"),
                    _ => None,
                };

                if let Some(hash_kind) = hash_kind {
                    if hash_kind != "Type" {
                        let hash = state.hash.clone();
                        thread::spawn(move || {
                            while hash.is_busy() {
                                thread::sleep(Duration::from_millis(16));
                            }

                            hash.request(hash_kind);
                        });

                        let hash = state.hash.clone();
                        let hash_label = hash_label.clone();
                        gtk::timeout_add(16, move || {
                            if !hash.is_ready() {
                                return Continue(true);
                            }

                            hash_label.set_text(hash.obtain().as_str());
                            Continue(false)
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
            state.buffer.state.store(image::SLEEPING, Ordering::SeqCst);

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
        let next = self.widgets.header.next.clone();
        let state = self.state.clone();
        let image_label = self.widgets.content.image_view.image_path.clone();
        let chooser_container = self.widgets.content.image_view.chooser_container.clone();
        let widgets = self.widgets.clone();

        gtk::timeout_add(500, move || {
            let tasks = &state.tasks;
            let bars = &state.bars;
            let image_length = &state.image_length;

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

            // Ensure that the image has been loaded before continuing.
            match state.buffer.state.load(Ordering::SeqCst) {
                0 => {
                    return Continue(true);
                }
                image::PROCESSING => {
                    chooser_container.set_visible_child_name("loader");
                    next.set_sensitive(false);
                    return Continue(true);
                }
                image::COMPLETED => {
                    chooser_container.set_visible_child_name("chooser");

                    if state.hash.is_busy() {
                        return Continue(true);
                    }

                    let (ref path, ref data) = *try_or_error!(
                        state.buffer.data.lock(),
                        "image buffer mutex lock failure",
                        Continue(false)
                    );
                    next.set_sensitive(true);
                    image_label.set_text(&path.file_name()
                        .expect("file chooser can't select directories")
                        .to_string_lossy());
                    image_length.set(data.len());
                }
                image::ERRORED => {
                    chooser_container.set_visible_child_name("chooser");
                    next.set_sensitive(false);
                    return Continue(true);
                }
                image::SLEEPING => (),
                _ => unreachable!(),
            }

            let image_length = image_length.get();
            let mut all_tasks_finished = true;
            let ntasks;

            {
                let tasks = try_or_error!(
                    tasks.lock(),
                    "tasks mutex lock failure",
                    Continue(false)
                );

                ntasks = tasks.len();
                if ntasks == 0 {
                    return Continue(true);
                }

                for (task, &(ref pbar, ref label)) in tasks.deref().iter().zip(bars.borrow().iter()) {
                    let raw_value = task.progress.load(Ordering::SeqCst);
                    let task_is_finished = task.finished.load(Ordering::SeqCst) == 1;
                    let value = if task_is_finished {
                        1.0f64
                    } else {
                        all_tasks_finished = false;
                        raw_value as f64 / image_length as f64
                    };

                    pbar.set_fraction(value);

                    if task_is_finished {
                        label.set_label("Complete");
                    } else if let Ok(mut prev_values) = task.previous.lock() {
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

            if all_tasks_finished && widgets.switch_to_summary(&state, ntasks).is_err() {
                return Continue(false);
            }

            state.reset_after_reacquiring_image();

            Continue(true)
        });
    }
}
