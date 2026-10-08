// SPDX-License-Identifier: MIT

use crate::fl;
use crate::views::devices::{self, DevicesView};
use crate::views::error::{self, ErrorView};
use crate::views::flashing::{self, FlashingView};
use crate::views::images::{self, ImagesView};
use crate::views::summary::{self, SummaryView};
use cosmic::app::context_drawer;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget::menu::{ItemHeight, ItemWidth};
use cosmic::widget::{self, about::About, menu};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// The about page for this app.
    about: About,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Currently active view.
    view: ActiveView,
    images: ImagesView,
    devices: DevicesView,
    flashing: FlashingView,
    summary: SummaryView,
    error: ErrorView,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    LaunchUrl(String),
    ToggleContextPage(ContextPage),
    Images(images::Message),
    Devices(devices::Message),
    Flashing(flashing::Message),
    Summary(summary::Message),
    Error(error::Message),
    Next,
    Back,
    Restart,
}

pub struct Flags {
    pub iso_argument: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveView {
    Images,
    Devices,
    Flashing,
    Summary,
    Error,
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = Flags;

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.system76.Popsicle";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: cosmic::Core, flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let open_image = match flags.iso_argument {
            Some(path) => {
                cosmic::task::message(Message::Images(images::Message::FileDropped(path)))
            }
            None => Task::none(),
        };

        // Create the about widget
        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            about,
            key_binds: HashMap::new(),
            view: ActiveView::Images,
            images: ImagesView::new(),
            devices: DevicesView::default(),
            flashing: FlashingView::default(),
            summary: SummaryView::default(),
            error: ErrorView::default(),
        };

        // Create a startup command that sets the window title and opens optional image file.
        let command = Task::batch([app.update_title(), open_image]);

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
            ),
        )])
        .item_height(ItemHeight::Dynamic(40))
        .item_width(ItemWidth::Uniform(360))
        .spacing(4.0);

        vec![menu_bar.into()]
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            ),
        })
    }

    fn footer(&self) -> Option<Element<'_, Self::Message>> {
        match self.view {
            ActiveView::Images => self.images.footer(&self.view),
            ActiveView::Devices => self.devices.footer(&self.view),
            ActiveView::Flashing => self.flashing.footer(&self.view),
            ActiveView::Summary => self.summary.footer(&self.view),
            ActiveView::Error => self.error.footer(&self.view),
        }
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        let view: Element<'_, Self::Message> = match self.view {
            ActiveView::Images => self.images.view().map(Message::Images),
            ActiveView::Devices => self.devices.view().map(Message::Devices),
            ActiveView::Flashing => self.flashing.view().map(Message::Flashing),
            ActiveView::Summary => self.summary.view().map(Message::Summary),
            ActiveView::Error => self.error.view().map(Message::Error),
        };

        widget::container(view)
            .width(600)
            .height(Length::Fill)
            .apply(widget::container)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They can be dynamically
    /// stopped and started conditionally based on application state, or persist
    /// indefinitely.
    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subscriptions = vec![];

        // Poll for USB drives only while the device picker is on screen.
        if self.view == ActiveView::Devices {
            subscriptions.push(self.devices.subscription().map(Message::Devices));
        }

        if self.view == ActiveView::Flashing {
            subscriptions.push(self.flashing.subscription().map(Message::Flashing));
        }

        Subscription::batch(subscriptions)
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },
            Message::Images(message) => {
                if let Some(task) = self.images.update(message) {
                    return task.map(|action| action.map(Message::Images));
                }
            }
            Message::Devices(message) => {
                if let Some(task) = self.devices.update(message) {
                    return task.map(|action| action.map(Message::Devices));
                }
            }
            Message::Flashing(message) => {
                self.flashing.update(message);

                match self.flashing.take_outcome() {
                    Some(Ok(outcome)) => {
                        self.summary.set_outcome(outcome);
                        self.view = ActiveView::Summary;
                    }
                    Some(Err(why)) => {
                        self.error.set_error(why);
                        self.view = ActiveView::Error;
                    }
                    None => {}
                }
            }
            Message::Summary(message) => match message {},
            Message::Error(message) => match message {},
            Message::Next => match self.view {
                ActiveView::Images => {
                    self.view = ActiveView::Devices;
                    self.devices.set_image_size(self.images.image_size());

                    return self.devices.refresh().map(|action| action.map(Message::Devices));
                }
                ActiveView::Devices => {
                    let Some(path) = self.images.image_path().map(Path::to_path_buf) else {
                        return Task::none();
                    };

                    self.view = ActiveView::Flashing;

                    return self
                        .flashing
                        .start(path, self.images.image_size(), self.devices.selected_devices())
                        .map(|action| action.map(Message::Flashing));
                }
                _ => return cosmic::iced::exit(),
            },
            Message::Back => {
                if self.view == ActiveView::Devices {
                    self.view = ActiveView::Images;
                }
            }
            Message::Restart => {
                self.flashing.cancel();
                self.devices.reset();
                self.view = ActiveView::Images;
            }
        }
        Task::none()
    }
}

impl AppModel {
    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let window_title = fl!("app-title");

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}
