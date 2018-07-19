mod content;
mod dialogs;
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
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{Sender, Receiver};
use std::fs::File;
use std::thread::JoinHandle;

use flash::FlashRequest;
use popsicle::mnt::MountEntry;
use popsicle::{self, DiskError};

use gtk;
use gtk::*;

const CSS: &str = include_str!("ui.css");

pub struct App {
    pub widgets: Rc<AppWidgets>,
    pub state:   Arc<State>,
}

pub struct AppWidgets {
    pub window:  Window,
    pub header:  Header,
    pub content: Content,
}

impl AppWidgets {
    pub fn switch_to_main(&self) {
        let stack = &self.content.container;
        let back = &self.header.back;
        let next = &self.header.next;

        self.content.devices_view.list.select_all.set_active(false);

        stack.set_transition_type(StackTransitionType::SlideRight);
        stack.set_visible_child_name("image");

        back.set_visible(true);
        back.set_label("Cancel");
        back.get_style_context().map(|c| {
            c.remove_class("back-button");
        });

        next.set_visible(true);
        next.set_label("Next");
        next.set_sensitive(true);
        next.get_style_context().map(|c| {
            c.remove_class("destructive-action");
            c.add_class("suggested-action");
        });
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
    }
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
