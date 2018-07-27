mod content;
mod dialogs;
mod header;
mod misc;
mod state;

use self::content::{Content, DeviceList};
pub use self::dialogs::OpenDialog;
use self::header::Header;
pub use self::misc::*;
pub use self::state::{Connect, FlashTask, State};

// TODO: Use AtomicU64 / Bool when https://github.com/rust-lang/rust/issues/32976 is stable.

use std::mem;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use std::fs::File;
use std::thread::JoinHandle;

use block::BlockDevice;
use flash::FlashRequest;
use popsicle::mnt::{self, MountEntry};
use popsicle::{self, DiskError};

use gtk;
use gtk::*;

const CSS: &str = include_str!("ui.css");

pub struct App {
    pub widgets: Rc<AppWidgets>,
    pub state:   Arc<State>,
}

impl App {
    pub fn new(
        sender: Sender<PathBuf>,
        devices_request: Sender<(Vec<String>, Vec<MountEntry>)>,
        devices_response: Receiver<Result<Vec<(String, File)>, DiskError>>,
        flash_request: Sender<FlashRequest>,
        flash_response: Receiver<JoinHandle<Result<(), DiskError>>>,
    ) -> App {
        // Initialize GTK before proceeding.
        if gtk::init().is_err() {
            eprintln!("failed to initialize GTK Application");
            process::exit(1);
        }

        // Create a new top level window.
        let window = Window::new(WindowType::Toplevel);
        // Create a the headerbar and it's associated content.
        let header = Header::new();
        // Create the content container and all of it's widgets.
        let content = Content::new();

        // Add a custom CSS style
        let screen = window.get_screen().unwrap();
        let style = CssProvider::new();
        let _ = CssProviderExt::load_from_data(&style, CSS.as_bytes());
        StyleContext::add_provider_for_screen(&screen, &style, STYLE_PROVIDER_PRIORITY_USER);

        // Set the headerbar as the title bar widget.
        window.set_titlebar(&header.container);
        // Set the title of the window.
        window.set_title("Popsicle");
        // Set the window manager class.
        window.set_wmclass("popsicle", "Popsicle");
        // The default size of the window to create.
        window.set_default_size(500, 250);
        // The icon the app will display.
        Window::set_default_icon_name("iconname");
        // Add the content to the window.
        window.add(&content.container);

        // Programs what to do when the exit button is used.
        window.connect_delete_event(move |_, _| {
            main_quit();
            Inhibit(false)
        });

        // Return the application structure.
        App {
            widgets: Rc::new(AppWidgets { window, header, content }),
            state: Arc::new(State::new(sender, devices_request, devices_response, flash_request, flash_response)),
        }
    }
}

pub struct AppWidgets {
    pub window:  Window,
    pub header:  Header,
    pub content: Content,
}

impl AppWidgets {
    pub fn switch_to_main(&self, state: &State) {
        let stack = &self.content.container;
        let back = &self.header.back;
        let next = &self.header.next;

        // If tasks are running, signify that tasks should be considered as completed.
        if 1 == state.flash_state.load(Ordering::SeqCst) {
            state.flash_state.store(2, Ordering::SeqCst);
        }

        self.content.devices_view.list.select_all.set_active(false);

        stack.set_transition_type(StackTransitionType::SlideRight);
        stack.set_visible_child_name("image");

        back.set_visible(true);
        back.set_label("Cancel");
        back.get_style_context().map(|c| {
            c.remove_class("back-button");
            c.remove_class("destructive-action");
        });

        next.set_visible(true);
        next.set_label("Next");
        next.set_sensitive(true);
        next.get_style_context().map(|c| {
            c.remove_class("destructive-action");
            c.add_class("suggested-action");
        });

        state.view.set(0)
    }

    pub fn switch_to_device_selection(&self, state: &State) {
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
            c.remove_class("suggested-action");
            c.add_class("destructive-action");
        });
        stack.set_visible_child_name("devices");

        let image_sectors = (state.image_length.get() / 512 + 1) as u64;
        let mut devices = vec![];
        if let Err(why) = popsicle::get_disk_args(&mut devices) {
            eprintln!("popsicle: unable to get devices: {}", why);
        }

        if let Err(why) = state.devices.lock()
            .map_err(|why| format!("mutex lock failed: {}", why))
            .and_then(|ref mut device_list| {
                list.refresh(device_list, &devices, image_sectors)
            })
        {
            self.set_error(state, &why);
        }
    }

    pub fn watch_device_selection(widgets: Rc<AppWidgets>, state: Arc<State>) {
        gtk::timeout_add(16, move || {
            let list = &widgets.content.devices_view.list;
            let next = &widgets.header.next;

            if state.view.get() != 1 {
                return gtk::Continue(false);
            }

            let mut disable_select_all = false;

            if let Ok(ref mut device_list) = state.devices.try_lock() {
                let mut check_refresh = || -> Result<(), String> {
                    match DeviceList::requires_refresh(&device_list) {
                        Some(devices) => {
                            let image_sectors = (state.image_length.get() / 512 + 1) as u64;
                            list.refresh(device_list, &devices, image_sectors)?;
                            disable_select_all = true;
                            next.set_sensitive(false);
                        }
                        None => {
                            next.set_sensitive(device_list.iter().any(|x| x.1.get_active()));
                        }
                    }

                    Ok(())
                };

                if let Err(why) = check_refresh() {
                    widgets.set_error(&state, &why);
                }
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

        let mut data = Vec::new();
        let image_data: Arc<Vec<u8>>;

        {
            let mut image_buffer_lock = try_or_error!(
                state.buffer.data.lock(),
                "failed to lock buffer.data mutex"
            );

            let device_list = try_or_error!(
                state.devices.lock(),
                "device list mutex lock failure"
            );

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
                c.add_class("destructive-action");
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
            let mut tasks = try_or_error!(
                tasks.lock(),
                "tasks mutex lock failure"
            );

            let mut task_handles = try_or_error!(
                task_handles.lock(),
                "task handles mutex lock failure"
            );

            state.flash_state.store(1, Ordering::SeqCst);

            // Take ownership of the data, so that we may wrap it within an `Arc`
            // and redistribute it across threads.
            //
            // Note: Possible optimization could be done to avoid the wrap.
            //       Avoiding the wrap could eliminate two allocations.
            image_data = {
                let (_, ref mut image_data) = *image_buffer_lock;
                mem::swap(&mut data, image_data);
                Arc::new(data)
            };

            for (id, (disk_path, mut disk)) in disks.into_iter().enumerate() {
                let id = id as i32;
                let image_data = image_data.clone();
                let progress = Arc::new(AtomicUsize::new(0));
                let finished = Arc::new(AtomicUsize::new(0));
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
                let bar_container = gtk::Box::new(Orientation::Vertical, 0);
                bar_container.pack_start(&pbar, false, false, 0);
                bar_container.pack_start(&bar_label, false, false, 0);
                summary_grid.attach(&label, 0, id, 1, 1);
                summary_grid.attach(&bar_container, 1, id, 1, 1);
                bars.push((pbar, bar_label));

                // Spawn a thread that will update the progress value over time.
                //
                // The value will be stored within an intermediary atomic integer,
                // because it is unsafe to send GTK widgets across threads.
                task_handles.push({
                    let _ = state.flash_request.send(FlashRequest::new(
                        disk,
                        disk_path,
                        image_data.len() as u64,
                        image_data,
                        state.flash_state.clone(),
                        progress.clone(),
                        finished.clone()
                    ));

                    state.flash_response.recv().expect("expected join handle to be returned")
                });

                tasks.push(FlashTask {
                    previous: Arc::new(Mutex::new([0; 7])),
                    progress,
                    finished,
                });
            }
        }

        state.async_reattain_image_data(image_data);
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
            .map(|c| c.remove_class("destructive-action"));
        next.set_label("Done");
        next.get_style_context()
            .map(|c| c.remove_class("destructive-action"));
        next.set_visible(true);
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

        let mut errored: Vec<(String, DiskError)> = Vec::new();
        let mut task_handles = try_or_error!(
            task_handles.lock(),
            "task handles mutex lock failure"
        );

        let devices = try_or_error!(
            devices.lock(),
            "devices mutex lock failure"
        );

        let handle_iter = task_handles.deref_mut().drain(..);
        let mut device_iter = devices.deref().iter();
        for handle in handle_iter {
            if let Some(&(ref device, _)) = device_iter.next() {
                let result = try_or_error!(
                    handle.join(),
                    "thread handle join failure"
                );

                if let Err(why) = result {
                    errored.push((device.clone(), why));
                }
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
                let container = Box::new(Orientation::Horizontal, 0);
                let device = Label::new(device.as_str());
                let why = Label::new(format!("{}", why).as_str());
                container.pack_start(&device, false, false, 0);
                container.pack_start(&why, true, true, 0);
                list.insert(&container, -1);
            }
        }

        state.set_as_complete();

        Ok(())
    }

    pub fn set_error(&self, state: &State, msg: &str) {
        let stack = &self.content.container;
        let back = &self.header.back;
        let next = &self.header.next;
        let error = &self.content.error_view.view.description;

        back.set_visible(false);
        next.set_visible(true);
        next.set_label("Close");
        next.get_style_context().map(|c| {
            c.remove_class("destructive-action");
            c.remove_class("suggested-action");
        });
        error.set_text(&msg);
        state.view.set(2);
        stack.set_visible_child_name("error");

        state.set_as_complete();
        state.reset_after_reacquiring_image();
    }
}
