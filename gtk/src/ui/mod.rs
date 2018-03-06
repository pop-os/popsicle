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

use self::hash::{md5_hasher, sha256_hasher};

use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use muff;

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

row:nth-child(1) {
    border-bottom-width: 0.1em;
    border-style: solid;    
}

list {
    margin-top: 1em;   
}"#;

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
        let image = Rc::new(RefCell::new(PathBuf::new()));
        let view = Rc::new(RefCell::new(0));
        let devices = Rc::new(RefCell::new(Vec::new()));

        {
            // Programs the image chooser button.
            let image = image.clone();
            let path_label = self.content.image_view.image_path.clone();
            let hash = self.content.image_view.hash.clone();
            let hash_label = self.content.image_view.hash_label.clone();
            let next = self.header.next.clone();
            self.content.image_view.chooser.connect_clicked(move |_| {
                if let Some(path) = OpenDialog::new(None).run() {
                    *image.borrow_mut() = path.clone();
                    path_label.set_label(&path.file_name().unwrap().to_string_lossy());
                    next.set_sensitive(true);

                    let result: io::Result<String> = match hash.get_active_text().unwrap().as_str()
                    {
                        "SHA256" => sha256_hasher(&path),
                        "MD5" => md5_hasher(&path),
                        _ => unimplemented!(),
                    };

                    match result {
                        Ok(hash) => hash_label.set_label(&hash),
                        Err(why) => {
                            eprintln!("muff: hash error: {}", why);
                            hash_label.set_label("error occurred");
                        }
                    }
                }
            });
        }

        {
            let image = image.clone();
            let hash_label = self.content.image_view.hash_label.clone();
            self.content.image_view.hash.connect_changed(move |hash| {
                let file = image.borrow();
                if file.is_file() {
                    let result: io::Result<String> = match hash.get_active_text().unwrap().as_str()
                    {
                        "SHA256" => sha256_hasher(&file),
                        "MD5" => md5_hasher(&file),
                        _ => unimplemented!(),
                    };

                    match result {
                        Ok(hash) => hash_label.set_label(&hash),
                        Err(why) => {
                            eprintln!("muff: hash error: {}", why);
                            hash_label.set_label("error occurred");
                        }
                    }
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
            let device_list = devices.clone();
            next.connect_clicked(move |next| {
                back.set_label("Back");
                next.set_label("Flash");
                next.get_style_context().map(|c| {
                    c.remove_class("suggested-action");
                    c.add_class("destructive-action");
                });

                stack.set_transition_type(StackTransitionType::SlideLeft);
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

                let mut device_list = device_list.borrow_mut();
                for device in &devices {
                    let device = Path::new(&device).canonicalize().unwrap();
                    let button = CheckButton::new_with_label(&device.to_string_lossy());
                    list.insert(&button, -1);
                    device_list.push(button);
                }

                list.show_all();

                *view.borrow_mut() += 1;
            });
        }

        {
            let all = self.content.devices_view.select_all.clone();
            let devices = devices.clone();
            all.connect_clicked(move |all| {
                if all.get_active() {
                    for device in devices.borrow().iter() {
                        device.set_active(true);
                    }
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
