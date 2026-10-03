mod devices;
mod error;
mod flashing;
mod images;
mod summary;

pub use self::devices::devices;
pub use self::error::error;
pub use self::flashing::flashing;
pub use self::images::images;
pub use self::summary::summary;

use super::{ActiveView, App, Message};
use crate::fl;
use cosmic::iced::Length;
use cosmic::widget::{self, icon};
use cosmic::{Element, theme};
use std::borrow::Cow;

const ICON_SIZE: u16 = 64;

/// The icons heading each view, from the icon theme with the bundled images as fallback.
pub struct Icons {
    pub image: icon::Handle,
    pub drive: icon::Handle,
    pub done: icon::Handle,
    pub error: icon::Handle,
}

impl Icons {
    pub fn load() -> Self {
        Icons {
            image: named_or("application-x-cd-image", || {
                icon::from_raster_bytes(
                    &include_bytes!("../../../../res/application-x-cd-image.png")[..],
                )
            }),
            drive: named_or("drive-removable-media-usb", || {
                icon::from_raster_bytes(
                    &include_bytes!("../../../../res/drive-removable-media-usb.png")[..],
                )
            }),
            done: named_or("process-completed", || {
                icon::from_svg_bytes(
                    &include_bytes!("../../../../res/process-completed-symbolic.svg")[..],
                )
                .symbolic(true)
            }),
            error: icon::from_name("dialog-error").size(ICON_SIZE).handle(),
        }
    }
}

fn named_or(name: &str, fallback: impl FnOnce() -> icon::Handle) -> icon::Handle {
    let named = icon::from_name(name).size(ICON_SIZE);
    if named.clone().path().is_some() { named.handle() } else { fallback() }
}

/// Lays out a view: its icon on the left; the title, description and content on the right.
fn page<'a>(
    handle: &icon::Handle,
    topic: impl Into<Cow<'a, str>> + 'a,
    description: impl Into<Cow<'a, str>> + 'a,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let spacing = theme::spacing();

    let panel = widget::column::with_capacity(3)
        .push(widget::text::title3(topic))
        .push(widget::text::body(description))
        .push(content)
        .spacing(spacing.space_xs)
        .width(Length::Fill)
        .height(Length::Fill);

    widget::row::with_capacity(2)
        .push(icon::icon(handle.clone()).size(ICON_SIZE))
        .push(panel)
        .spacing(spacing.space_s)
        .padding(spacing.space_s)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The header button that cancels, or returns to the image view.
pub fn back_button(view: ActiveView) -> Element<'static, Message> {
    match view {
        ActiveView::Images | ActiveView::Devices => widget::button::standard(fl!("cancel")),
        ActiveView::Flashing => widget::button::destructive(fl!("cancel")),
        ActiveView::Summary | ActiveView::Error => widget::button::standard(fl!("flash-again")),
    }
    .on_press(Message::Back)
    .into()
}

/// The header button that advances the wizard, absent while flashing.
pub fn next_button(app: &App) -> Option<Element<'_, Message>> {
    let button = match app.view_kind() {
        ActiveView::Images => widget::button::suggested(fl!("next"))
            .on_press_maybe(app.image().path.is_some().then_some(Message::Next)),
        ActiveView::Devices => widget::button::destructive(fl!("next"))
            .on_press_maybe(app.devices().any_selected().then_some(Message::Next)),
        ActiveView::Flashing => return None,
        ActiveView::Summary => widget::button::standard(fl!("done")).on_press(Message::Next),
        ActiveView::Error => widget::button::standard(fl!("close")).on_press(Message::Next),
    };

    Some(button.into())
}
