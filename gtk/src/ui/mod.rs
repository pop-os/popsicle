use gtk;
use gtk::*;
use std::process;

mod content;
mod dialogs;
mod header;

use self::content::Content;
use self::dialogs::OpenDialog;
use self::header::Header;

use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use digest::{Digest, Input};
use md5::Md5;
use sha3::Sha3_256;

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

        {
            // Programs the image chooser button.
            let image = image.clone();
            let path_label = self.content.image_view.image_path.clone();
            let hash_button = self.content.image_view.hash_button.clone();
            let next = self.header.next.clone();
            self.content.image_view.chooser.connect_clicked(move |_| {
                if let Some(path) = OpenDialog::new(None).run() {
                    *image.borrow_mut() = path.clone();
                    path_label.set_label(&path.file_name().unwrap().to_string_lossy());
                    next.set_sensitive(true);
                    hash_button.set_sensitive(true);
                }
            });
        }

        {
            fn md5_hasher(path: &Path) -> io::Result<String> {
                let mut hasher = Md5::default();
                let mut buffer = [0; 16 * 1024];
                File::open(path).and_then(|mut file| {
                    let mut read = file.read(&mut buffer)?;
                    while read != 0 {
                        hasher.process(&buffer[..read]);
                        read = file.read(&mut buffer)?;
                    }
                    Ok(format!("{:x}", hasher.result()))
                })
            }

            fn sha256_hasher(path: &Path) -> io::Result<String> {
                let mut hasher = Sha3_256::default();
                let mut buffer = [0; 16 * 1024];
                File::open(path).and_then(|mut file| {
                    let mut read = file.read(&mut buffer)?;
                    while read != 0 {
                        hasher.process(&buffer[..read]);
                        read = file.read(&mut buffer)?;
                    }
                    Ok(format!("{:x}", hasher.result()))
                })
            }

            // Programs the hash generator button.
            let hash = self.content.image_view.hash.clone();
            let hash_label = self.content.image_view.hash_label.clone();
            self.content
                .image_view
                .hash_button
                .connect_clicked(move |_| {
                    let file = image.borrow();
                    if file.is_file() {
                        let result = match hash.get_active_text().unwrap().as_str() {
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
            let view = view.clone();
            next.connect_clicked(move |next| {
                next.set_sensitive(false);
                stack.set_transition_type(StackTransitionType::SlideLeft);
                stack.set_visible_child_name("devices");
                back.set_label("Back");
                next.set_label("Flash");
                next.get_style_context().map(|c| {
                    c.remove_class("suggested-action");
                    c.add_class("destructive-action");
                });
                *view.borrow_mut() += 1;
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
