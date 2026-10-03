use super::{App, Message, page};
use crate::fl;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::{Element, theme};

pub fn flashing(app: &App) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let rows = app.flash_rows();

    let mut progress_list = widget::column::with_capacity(rows.len()).spacing(spacing.space_xxs);

    for row in rows {
        let bar = widget::column::with_capacity(2)
            .push(widget::progress_bar::determinate_linear(row.fraction).width(Length::Fill))
            .push(widget::text::caption(&row.status))
            .spacing(spacing.space_xxxs)
            .align_x(Alignment::Center)
            .width(Length::FillPortion(3));

        let label =
            widget::text::heading(&row.label).width(Length::FillPortion(2)).align_x(Alignment::End);

        progress_list = progress_list.push(
            widget::row::with_capacity(2)
                .push(label)
                .push(bar)
                .spacing(spacing.space_xs)
                .align_y(Alignment::Center),
        );
    }

    page(
        &app.icons().drive,
        fl!("flash-view-title"),
        fl!("flash-view-description"),
        widget::scrollable(progress_list).width(Length::Fill).height(Length::Fill),
    )
}
