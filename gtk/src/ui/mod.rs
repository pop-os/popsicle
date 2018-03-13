mod content;
mod state;
mod dialogs;
mod hash;
mod header;

use self::content::Content;
pub use self::dialogs::OpenDialog;
use self::header::Header;
pub use self::state::Connect;

// TODO: Use AtomicU64 / Bool when https://github.com/rust-lang/rust/issues/32976 is stable.

use std::cell::RefCell;
use std::mem;
use std::process;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicUsize;
use std::thread::JoinHandle;
use std::time::Instant;

use gtk;
use gtk::*;
use muff::DiskError;

const CSS: &str = include_str!("ui.css");

pub struct App {
    pub window:  Window,
    pub header:  Header,
    pub content: Content,
    pub state:   Arc<State>,
}

// TODO:
// pub struct BufferingData {
//     data: Mutex<Vec<u8>>,
//     state: AtomicUsize,
// }

// impl BufferingData {
//     fn new() -> BufferingData {
//         BufferingData { data: Vec::new().into(), state: 0.into() }
//     }
// }

/// Contains all of the state that needs to be shared across the program's lifetime.
pub struct State {
    /// Contains all of the progress bars in the flash view.
    pub bars: RefCell<Vec<(ProgressBar, Label)>>,
    /// Contains a list of devices detected, and their check buttons.
    pub devices: Mutex<Vec<(String, CheckButton)>>,
    /// Contains a buffered vector of the ISO data, to be shared across threads.
    pub image_data: RefCell<Option<Arc<Vec<u8>>>>,
    /// Holds the task threads that write the image to each device.
    /// The handles may contain errors when joined, for printing on the summary page.
    pub task_handles: Mutex<Vec<JoinHandle<Result<(), DiskError>>>>,
    /// Contains progress data regarding each active flash task -- namely the progress.
    pub tasks: Mutex<Vec<FlashTask>>,
    /// Stores an integer which defines the currently-active view.
    pub view: RefCell<u8>,
    /// Stores the time when the flashing process began.
    pub start: RefCell<Instant>,
    /* TODO:
     * buffer: BufferingData */
}

impl State {
    fn new() -> State {
        State {
            bars:         RefCell::new(Vec::new()),
            devices:      Mutex::new(Vec::new()),
            image_data:   RefCell::new(None),
            task_handles: Mutex::new(Vec::new()),
            tasks:        Mutex::new(Vec::new()),
            view:         RefCell::new(0),
            start:        RefCell::new(unsafe { mem::uninitialized() }),
            // buffer:       BufferingData::new(),
        }
    }
}

pub struct FlashTask {
    progress: Arc<AtomicUsize>,
    previous: Arc<Mutex<[usize; 7]>>,
    finished: Arc<AtomicUsize>,
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
            state: Arc::new(State::new()),
        }
    }
}
