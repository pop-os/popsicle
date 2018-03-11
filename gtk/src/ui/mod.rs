use gtk;
use gtk::*;
use std::process;

mod content;
mod dialogs;
mod hash;
mod header;

use self::content::Content;
use self::dialogs::OpenDialog;
use self::header::Header;

// TODO: Use AtomicU64 / Bool when https://github.com/rust-lang/rust/issues/32976 is stable.

use std::cell::RefCell;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use muff::{self, DiskError, Image};

const CSS: &str = r#"stack {
	padding: 0.5em;
}

.h2 {
	font-size: 1.25em;
	font-weight: bold;
	padding-bottom: 1em;
}

.bold {
    font-weight: bold;    
}

.desc {
    padding-bottom: 1em;
}

.hash-box button {
    border-radius: 0;
    border-top-left-radius: 10px;
    border-bottom-left-radius: 10px;
}

.devices {
    border-width:0.2em;
    border-style: solid;
    border-color: rgba(0,0,0,0.2);
}

.devices > row:nth-child(1) {
    border-bottom-width:0.2em;
    border-style: solid;
    border-color: rgba(0,0,0,0.2);
    font-weight: bold;
}

.progress-label {
    font-weight: bold;
    padding-right: 1em;
}"#;

struct FlashTask {
    progress: Arc<AtomicUsize>,
    finished: Arc<AtomicUsize>,
}

pub struct App {
    pub window:  Window,
    pub header:  Header,
    pub content: Content,
}

impl App {
    pub fn new() -> App {
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
        window.set_title("Multiple USB File Flasher");
        // Set the window manager class.
        window.set_wmclass("muff", "Multiple USB File Flasher");
        // The icon the app will display.
        window.set_default_size(400, -1);
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
            window,
            header,
            content,
        }
    }

    /// Creates external state, and maps all of the UI functionality to the UI.
    pub fn connect_events(self) -> Connected {
        let view = Rc::new(RefCell::new(0));
        let devices: Arc<Mutex<Vec<(String, CheckButton)>>> = Arc::new(Mutex::new(Vec::new()));
        let image_data = Rc::new(RefCell::new(None));
        let tasks = Arc::new(Mutex::new(Vec::new()));
        let task_handles = Arc::new(Mutex::new(Vec::new()));
        let bars = Rc::new(RefCell::new(Vec::new()));

        {
            // Programs the image chooser button.
            let image_data = image_data.clone();
            let path_label = self.content.image_view.image_path.clone();
            let next = self.header.next.clone();
            self.content.image_view.chooser.connect_clicked(move |_| {
                if let Some(path) = OpenDialog::new(None).run() {
                    let mut new_image = match Image::new(&path) {
                        Ok(image) => image,
                        Err(why) => {
                            eprintln!("muff: unable to open image: {}", why);
                            return;
                        }
                    };

                    let new_data = match new_image.read(|_| ()) {
                        Ok(data) => data,
                        Err(why) => {
                            eprintln!("muff: unable to read image: {}", why);
                            return;
                        }
                    };

                    path_label.set_label(&path.file_name().unwrap().to_string_lossy());
                    next.set_sensitive(true);
                    *image_data.borrow_mut() = Some(Arc::new(new_data));
                }
            });
        }

        {
            let image_data = image_data.clone();
            let hash_label = self.content.image_view.hash_label.clone();
            self.content.image_view.hash.connect_changed(move |hash| {
                if let Some(ref data) = *image_data.borrow() {
                    hash_label.set_icon_from_icon_name(EntryIconPosition::Primary, "gnome-spinner");
                    hash_label.set_icon_sensitive(EntryIconPosition::Primary, true);
                    hash::set(&hash_label, hash.get_active_text().unwrap().as_str(), data);
                    hash_label.set_icon_sensitive(EntryIconPosition::Primary, false);
                }
            });
        }

        {
            // Programs the back button.
            let stack = self.content.container.clone();
            let back = self.header.back.clone();
            let next = self.header.next.clone();
            let view = view.clone();
            back.connect_clicked(move |back| {
                match *view.borrow() {
                    0 => gtk::main_quit(),
                    1 => {
                        stack.set_transition_type(StackTransitionType::SlideRight);
                        stack.set_visible_child_name("image");
                        back.set_label("Cancel");
                        next.set_label("Next");
                        next.set_sensitive(true);
                        next.get_style_context().map(|c| {
                            c.remove_class("destructive-action");
                            c.add_class("suggested-action");
                        });
                    }
                    _ => unreachable!(),
                }
                *view.borrow_mut() -= 1;
            });
        }

        {
            // Programs the next button.
            let stack = self.content.container.clone();
            let back = self.header.back.clone();
            let next = self.header.next.clone();
            let list = self.content.devices_view.list.clone();
            let view = view.clone();
            let image_data = image_data.clone();
            let device_list = devices.clone();
            let tasks = tasks.clone();
            let task_handles = task_handles.clone();
            let summary_grid = self.content.flash_view.progress_list.clone();
            let bars = bars.clone();
            next.connect_clicked(move |next| {
                stack.set_transition_type(StackTransitionType::SlideLeft);
                match *view.borrow() {
                    // Move to device selection screen
                    0 => {
                        back.set_label("Back");
                        next.set_label("Flash");
                        next.get_style_context().map(|c| {
                            c.remove_class("suggested-action");
                            c.add_class("destructive-action");
                        });
                        stack.set_visible_child_name("devices");

                        // Remove all but the first row
                        list.get_children()
                            .into_iter()
                            .skip(1)
                            .for_each(|widget| widget.destroy());

                        let mut devices = vec![];
                        if let Err(why) = muff::get_disk_args(&mut devices) {
                            eprintln!("muff: unable to get devices: {}", why);
                        }

                        let mut device_list = device_list.lock().unwrap();
                        for device in &devices {
                            let name = Path::new(&device).canonicalize().unwrap();
                            let button = CheckButton::new_with_label(&name.to_string_lossy());
                            list.insert(&button, -1);
                            device_list.push((device.clone(), button));
                        }

                        list.show_all();
                    }
                    // Begin the device flashing process
                    1 => {
                        let device_list = device_list.lock().unwrap();
                        let devs = device_list.iter().map(|x| x.0.clone());
                        // TODO: Handle Error
                        let mounts = muff::Mount::all().unwrap();
                        // TODO: Handle Error
                        let disks = muff::disks_from_args(devs, &mounts, true).unwrap();

                        back.set_visible(false);
                        next.set_visible(false);
                        stack.set_visible_child_name("flash");

                        // Clear the progress bar summaries.
                        let mut bars = bars.borrow_mut();
                        bars.clear();
                        summary_grid.get_children().iter().for_each(|c| c.destroy());

                        let mut tasks = tasks.lock().unwrap();
                        let mut task_handles = task_handles.lock().unwrap();
                        for (id, (disk_path, mut disk)) in disks.into_iter().enumerate() {
                            let id = id as i32;
                            let image_data = image_data.borrow();
                            let image_data = image_data.as_ref().unwrap().clone();
                            let progress = Arc::new(AtomicUsize::new(0));
                            let finished = Arc::new(AtomicUsize::new(0));
                            let bar = ProgressBar::new();
                            bar.set_hexpand(true);
                            let label = Label::new(
                                Path::new(&disk_path)
                                    .canonicalize()
                                    .unwrap()
                                    .to_str()
                                    .unwrap(),
                            );
                            label.set_justify(Justification::Right);
                            label
                                .get_style_context()
                                .map(|c| c.add_class("progress-label"));
                            summary_grid.attach(&label, 0, id, 1, 1);
                            summary_grid.attach(&bar, 1, id, 1, 1);
                            bars.push(bar);

                            // Spawn a thread that will update the progress value over time.
                            //
                            // The value will be stored within an intermediary atomic integer,
                            // because it is unsafe to send GTK widgets across threads.
                            task_handles.push({
                                let progress = progress.clone();
                                let finished = finished.clone();
                                thread::spawn(move || -> Result<(), DiskError> {
                                    let result = muff::write_to_disk(
                                        |_msg| (),
                                        || (),
                                        |value| progress.store(value as usize, Ordering::SeqCst),
                                        disk,
                                        disk_path,
                                        image_data.len() as u64,
                                        &image_data,
                                        false,
                                    );

                                    finished.store(1, Ordering::SeqCst);
                                    result
                                })
                            });

                            tasks.push(FlashTask { progress, finished });
                        }

                        summary_grid.show_all();
                    }
                    2 => gtk::main_quit(),
                    _ => unreachable!(),
                }

                *view.borrow_mut() += 1;
            });
        }

        {
            let all = self.content.devices_view.select_all.clone();
            let devices = devices.clone();
            all.connect_clicked(move |all| {
                if all.get_active() {
                    for &(_, ref device) in devices.lock().unwrap().iter() {
                        device.set_active(true);
                    }
                }
            });
        }

        {
            let tasks = tasks.clone();
            let bars = bars.clone();
            let stack = self.content.container.clone();
            let next = self.header.next.clone();
            let image_data = image_data.clone();
            gtk::timeout_add(1000, move || {
                let image_length = match *image_data.borrow() {
                    Some(ref data) => data.len() as f64,
                    None => {
                        return Continue(true);
                    }
                };

                let tasks = tasks.lock().unwrap();
                if tasks.is_empty() {
                    return Continue(true);
                }

                let mut finished = true;
                for (task, bar) in tasks.deref().iter().zip(bars.borrow().iter()) {
                    let value = if task.finished.load(Ordering::SeqCst) == 1 {
                        1.0f64
                    } else {
                        finished = false;
                        task.progress.load(Ordering::SeqCst) as f64 / image_length
                    };

                    bar.set_fraction(value);
                }

                if finished {
                    stack.set_visible_child_name("summary");
                    next.set_label("Finished");
                    next.get_style_context()
                        .map(|c| c.remove_class("destructive-action"));
                    next.set_visible(true);
                    Continue(false)
                } else {
                    Continue(true)
                }
            });
        }

        Connected(self)
    }
}

pub struct Connected(App);

impl Connected {
    /// Display the window, and execute the gtk main event loop.
    pub fn then_execute(self) {
        self.0.window.show_all();
        gtk::main();
    }
}
