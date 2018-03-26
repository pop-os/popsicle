mod content;
mod dialogs;
mod hash;
mod header;
mod misc;
mod state;

use self::content::Content;
pub use self::dialogs::OpenDialog;
use self::header::Header;
pub use self::misc::*;
pub use self::state::{Connect, State};

// TODO: Use AtomicU64 / Bool when https://github.com/rust-lang/rust/issues/32976 is stable.

use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use gtk;
use gtk::*;

const CSS: &str = include_str!("ui.css");

pub struct App {
    pub window:  Window,
    pub header:  Header,
    pub content: Content,
    pub state:   Arc<State>,
}

impl App {
    pub fn new(sender: Sender<PathBuf>) -> App {
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
        window.set_default_size(400, -1);
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
            window,
            header,
            content,
            state: Arc::new(State::new(sender)),
        }
    }
}
