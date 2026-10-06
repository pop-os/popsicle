// SPDX-License-Identifier: MIT

use crate::config::Config;
use crate::fl;
use crate::views::devices::{self, DevicesView};
use crate::views::error::{self, ErrorView};
use crate::views::flashing::{self, FlashingView};
use crate::views::images::{self, ImagesView};
use crate::views::summary::{self, SummaryView};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{self, about::About, menu};
use std::collections::HashMap;
use std::path::PathBuf;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

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
    /// Configuration data that persists between application runs.
    config: Config,
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
    UpdateConfig(Config),
    Images(images::Message),
    Devices(devices::Message),
    Flashing(flashing::Message),
    Summary(summary::Message),
    Error(error::Message),
    Next,
    Cancel,
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
    const APP_ID: &'static str = "dev.mmurphy.Test";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
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
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((errors, config)) => {
                        for why in errors {
                            tracing::error!(%why, "error loading app config");
                        }

                        config
                    }
                })
                .unwrap_or_default(),
            view: ActiveView::Images,
            images: ImagesView::new(),
            devices: DevicesView::default(),
            flashing: FlashingView,
            summary: SummaryView,
            error: ErrorView,
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
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
            ActiveView::Devices => self.devices.view().into().map(Message::Devices),
            ActiveView::Flashing => self.flashing.view().into().map(Message::Flashing),
            ActiveView::Summary => self.summary.view().into().map(Message::Summary),
            ActiveView::Error => self.error.view().into().map(Message::Error),
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
        // Add subscriptions which are always active.
        let subscriptions = vec![
            // Watch for application configuration changes.
            self.core().watch_config::<Config>(Self::APP_ID).map(|update| {
                for why in update.errors {
                    tracing::error!(?why, "app config error");
                }

                Message::UpdateConfig(update.config)
            }),
        ];

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

            Message::UpdateConfig(config) => {
                self.config = config;
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
            Message::Devices(_message) => todo!(),
            Message::Flashing(_message) => todo!(),
            Message::Summary(_message) => todo!(),
            Message::Error(_message) => todo!(),
            Message::Next => match self.view {
                ActiveView::Images => self.view = ActiveView::Devices,
                ActiveView::Devices => self.view = ActiveView::Flashing,
                _ => return cosmic::iced::exit(),
            },
            Message::Cancel => match self.view {
                ActiveView::Images => return cosmic::iced::exit(),
                _ => self.view = ActiveView::Images,
            },
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
