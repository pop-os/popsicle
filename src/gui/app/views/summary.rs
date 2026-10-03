use super::{App, Message, page};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::{Element, theme};

pub fn summary(app: &App) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let summary = app.summary();

    // Lists the devices which failed to flash, with the reason for each.
    let mut list = widget::list::list_column::with_capacity(summary.errors.len());

    for (device, why) in &summary.errors {
        list = list.add(
            widget::row::with_capacity(2)
                .push(widget::text::body(device))
                .push(widget::text::heading(why).width(Length::Fill))
                .spacing(spacing.space_xxs)
                .align_y(Alignment::Center),
        );
    }

    let content: Element<'_, Message> = if summary.errors.is_empty() {
        widget::space::vertical().into()
    } else {
        widget::scrollable(list).width(Length::Fill).height(Length::Fill).into()
    };

    page(&app.icons().done, &summary.topic, &summary.description, content)
}
