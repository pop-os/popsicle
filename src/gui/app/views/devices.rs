use super::{App, Message, page};
use crate::fl;
use crate::gui::misc;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::{Element, theme};

pub fn devices(app: &App) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let devices = app.devices();
    let image_size = app.image().size;

    let select_all =
        widget::checkbox(devices.select_all).label(fl!("select-all")).on_toggle(Message::SelectAll);

    let mut list = widget::list::list_column::with_capacity(devices.list.len());

    for (id, (device, selected)) in devices.list.iter().zip(&devices.selected).enumerate() {
        let valid_size = device.parent.size >= image_size;

        let size = bytesize::to_string(device.parent.size, true);
        let detail =
            if valid_size { size } else { format!("{}: {}", size, fl!("device-too-small")) };

        // Devices smaller than the image cannot be selected.
        let checkbox = widget::checkbox(*selected).on_toggle_maybe(
            valid_size.then_some(move |checked| Message::ToggleDevice(id, checked)),
        );

        let label = widget::column::with_capacity(2)
            .push(widget::text::heading(misc::device_label(device)))
            .push(widget::text::caption(detail));

        list = list.add(
            widget::row::with_capacity(2)
                .push(checkbox)
                .push(label)
                .spacing(spacing.space_xs)
                .align_y(Alignment::Center),
        );
    }

    let content = widget::column::with_capacity(2)
        .push(select_all)
        .push(widget::scrollable(list).width(Length::Fill).height(Length::Fill))
        .spacing(spacing.space_xxs);

    page(&app.icons().drive, fl!("devices-view-title"), fl!("devices-view-description"), content)
}
