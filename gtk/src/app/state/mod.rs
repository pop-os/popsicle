mod device_selection;
mod flash_devices;
#[macro_use]
mod try;

use self::device_selection::device_selection;
use self::flash_devices::flash_devices;

use super::super::BlockDevice;
use super::{App, OpenDialog};

use hash::HashState;
use image::{self, BufferingData};
use std::cell::{Cell, RefCell};
use std::mem;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use gtk;
use gtk::*;
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
}

impl State {
    /// Initailizes a new structure for managing the state of the application.
    pub fn new(image_sender: Sender<PathBuf>) -> State {
        State {
            bars: RefCell::new(Vec::new()),
            devices: Mutex::new(Vec::new()),
            hash: Arc::new(HashState::new()),
            task_handles: Mutex::new(Vec::new()),
            tasks: Mutex::new(Vec::new()),
            view: Cell::new(0),
            start: RefCell::new(unsafe { mem::uninitialized() }),
            buffer: Arc::new(BufferingData::new()),
            image_sender,
            image_length: Cell::new(0),
        }
    }
}

pub struct FlashTask {
    progress: Arc<AtomicUsize>,
    previous: Arc<Mutex<[usize; 7]>>,
    finished: Arc<AtomicUsize>,
}




pub struct Connected(App);

impl Connected {
    /// Display the window, and execute the gtk main event loop.
    pub fn then_execute(self) {
        self.0.window.show_all();
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
        self.content.image_view.chooser.connect_clicked(move |_| {
            if let Some(path) = OpenDialog::new(None).run() {
                let _ = state.image_sender.send(path);
            }
        });
    }

    fn connect_hash_generator(&self) {
        let state = self.state.clone();
        let hash_label = self.content.image_view.hash_label.clone();
        self.content
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
        let stack = self.content.container.clone();
        let back = self.header.back.clone();
        let next = self.header.next.clone();
        let state = self.state.clone();
        back.connect_clicked(move |back| {
            let view = state.view.get();
            match view {
                0 => {
                    gtk::main_quit();
                    return;
                },
                1 => {
                    stack.set_transition_type(StackTransitionType::SlideRight);
                    stack.set_visible_child_name("image");
                    back.set_label("Cancel");
                    back.get_style_context().map(|c| {
                        c.remove_class("back-button");
                    });
                    next.set_label("Next");
                    next.set_sensitive(true);
                    next.get_style_context().map(|c| {
                        c.remove_class("destructive-action");
                        c.add_class("suggested-action");
                    });
                }
                _ => unreachable!(),
            }

            state.view.set(view - 1);
        });
    }

    fn connect_next_button(&self) {
        #[allow(unused_variables)]
        let back = self.header.back.clone();
        let list = self.content.devices_view.list.clone();
        let back = self.header.back.clone();
        let next = self.header.next.clone();
        let stack = self.content.container.clone();
        let summary_grid = self.content.flash_view.progress_list.clone();
        let state = self.state.clone();
        let error = self.content.error_view.view.description.clone();

        fn watch_device_selection(state: Arc<State>, next: gtk::Button) {
            gtk::timeout_add(16, move || {
                if state.view.get() == 1 {
                    if let Ok(ref mut devices) = state.devices.try_lock() {
                        next.set_sensitive(devices.iter().any(|x| x.1.get_active()));
                    }
                    gtk::Continue(true)
                } else {
                    gtk::Continue(false)
                }
            });
        }

        next.connect_clicked(move |next| {
            state.buffer.state.store(image::SLEEPING, Ordering::SeqCst);

            let view = &state.view;
            let view_value = view.get();
            stack.set_transition_type(StackTransitionType::SlideLeft);

            match view_value {
                0 => device_selection(&state, &back, &error, &list, &next, &stack),
                1 => flash_devices(&state, &back, &error, &next, &stack, &summary_grid),
                2 => gtk::main_quit(),
                _ => unreachable!(),
            }

            view.set(view_value + 1);

            if view.get() == 1 {
                watch_device_selection(state.clone(), next.clone());
            }
        });
    }

    fn connect_check_all(&self) {
        let all = self.content.devices_view.select_all.clone();
        let back = self.header.back.clone();
        let next = self.header.next.clone();
        let stack = self.content.container.clone();
        let error = self.content.error_view.view.description.clone();
        let state = self.state.clone();
        all.connect_clicked(move |all| {
            let devices = try_or_error!(
                state.devices.lock(),
                state.view,
                back,
                next,
                stack,
                error,
                "devices mutex lock failure",
                ()
            );

            devices
                .iter()
                .for_each(|&(_, ref device)| device.set_active(all.get_active()));
        });
    }

    fn watch_flashing_devices(&self) {
        let stack = self.content.container.clone();
        let back = self.header.back.clone();
        let next = self.header.next.clone();
        let description = self.content.summary_view.view.description.clone();
        let list = self.content.summary_view.list.clone();
        let state = self.state.clone();
        let image_label = self.content.image_view.image_path.clone();
        let chooser_container = self.content.image_view.chooser_container.clone();
        let error = self.content.error_view.view.description.clone();

        gtk::timeout_add(500, move || {
            let tasks = &state.tasks;
            let bars = &state.bars;
            let devices = &state.devices;
            let task_handles = &state.task_handles;
            let image_length = &state.image_length;

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
                        state.view,
                        back,
                        next,
                        stack,
                        error,
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

            let tasks = try_or_error!(
                tasks.lock(),
                state.view,
                back,
                next,
                stack,
                error,
                "tasks mutex lock failure",
                Continue(false)
            );

            let ntasks = tasks.len();
            if ntasks == 0 {
                return Continue(true);
            }

            let mut finished = true;
            for (task, &(ref bar, ref label)) in tasks.deref().iter().zip(bars.borrow().iter()) {
                let raw_value = task.progress.load(Ordering::SeqCst);
                let value = if task.finished.load(Ordering::SeqCst) == 1 {
                    1.0f64
                } else {
                    finished = false;
                    raw_value as f64 / image_length as f64
                };

                bar.set_fraction(value);

                if let Ok(mut prev_values) = task.previous.lock() {
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

            if finished {
                stack.set_visible_child_name("summary");
                next.set_label("Close");
                next.get_style_context()
                    .map(|c| c.remove_class("destructive-action"));
                next.set_visible(true);

                let mut errored: Vec<(String, DiskError)> = Vec::new();
                let mut task_handles = try_or_error!(
                    task_handles.lock(),
                    state.view,
                    back,
                    next,
                    stack,
                    error,
                    "task handles mutex lock failure",
                    Continue(false)
                );

                let devices = try_or_error!(
                    devices.lock(),
                    state.view,
                    back,
                    next,
                    stack,
                    error,
                    "devices mutex lock failure",
                    Continue(false)
                );

                let handle_iter = task_handles.deref_mut().drain(..);
                let mut device_iter = devices.deref().iter();
                for handle in handle_iter {
                    if let Some(&(ref device, _)) = device_iter.next() {
                        let result = try_or_error!(
                            handle.join(),
                            state.view,
                            back,
                            next,
                            stack,
                            error,
                            "thread handle join failure",
                            Continue(false)
                        );

                        if let Err(why) = result {
                            errored.push((device.clone(), why));
                        }
                    }
                }

                if errored.is_empty() {
                    description.set_text(&format!("{} devices successfully flashed", ntasks));
                } else {
                    description.set_text(&format!(
                        "{} of {} devices successfully flashed",
                        ntasks - errored.len(),
                        ntasks
                    ));
                    list.set_visible(true);
                    for (device, why) in errored {
                        let container = Box::new(Orientation::Horizontal, 0);
                        let device = Label::new(device.as_str());
                        let why = Label::new(format!("{}", why).as_str());
                        container.pack_start(&device, false, false, 0);
                        container.pack_start(&why, true, true, 0);
                        list.insert(&container, -1);
                    }
                }

                Continue(false)
            } else {
                Continue(true)
            }
        });
    }
}
