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
use std::path::PathBuf;
use std::rc::Rc;

const CSS: &str = r#"stack > box > box {
	padding: 0.5em;
}

.h2 {
	font-size: 1.25em;
	font-weight: bold;
	padding-bottom: 1em;
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
        window.set_default_size(-1, -1);
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
            let image = image.clone();
            let back = self.header.back.clone();
            let next = self.header.next.clone();
            self.content.image_view.chooser.connect_clicked(move |_| {
                if let Some(path) = OpenDialog::new(None).run() {
                    *image.borrow_mut() = path;
                    next.set_sensitive(true);
                }
            });
        }

        {
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
