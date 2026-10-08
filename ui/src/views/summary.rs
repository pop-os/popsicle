// SPDX-License-Identifier: MIT

use cosmic::{
    Apply, Element,
    iced::{Alignment, Length},
    theme::spacing,
    widget,
};

use crate::{
    app::ActiveView,
    fl,
    views::{devices::device_label, flashing::FlashOutcome},
};

#[derive(Debug, Default)]
pub struct SummaryView {
    outcome: Option<FlashOutcome>,
}

#[derive(Debug, Clone)]
pub enum Message {}

impl SummaryView {
    pub fn set_outcome(&mut self, outcome: FlashOutcome) {
        self.outcome = Some(outcome);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let Some(outcome) = &self.outcome else {
            return widget::column([]).into();
        };

        let space = spacing();
        let success = outcome.is_success();

        let icon = widget::icon::from_name(if success {
            "process-completed-symbolic"
        } else {
            "dialog-warning-symbolic"
        })
        .size(64)
        .symbolic(true);

        let title = widget::text::title2(if success {
            fl!("flashing-completed")
        } else {
            fl!("flashing-completed-with-errors")
        });

        let description = if success {
            fl!("successful-flash", total = outcome.total)
        } else {
            let mut description =
                fl!("partial-flash", number = outcome.succeeded(), total = outcome.total);

            if let Some(error) = &outcome.error {
                description.push_str(": ");
                description.push_str(error);
            }

            description
        };

        let failures = (!outcome.failed.is_empty()).then(|| {
            outcome.failed.iter().fold(widget::list_column(), |list, (device, why)| {
                list.add(
                    widget::column([])
                        .push(widget::text::body(device_label(device)).font(cosmic::font::bold()))
                        .push(widget::text::caption(why.as_str()))
                        .spacing(space.space_xxs),
                )
            })
        });

        widget::column([])
            .push(icon)
            .push(title)
            .push(widget::text::body(description).align_x(Alignment::Center))
            .push_maybe(failures)
            .align_x(Alignment::Center)
            .spacing(space.space_s)
            .width(Length::Fill)
            .apply(widget::scrollable)
            .into()
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, crate::app::Message>> {
        if *view != ActiveView::Summary {
            return None;
        }

        let flash_again =
            widget::button::standard(fl!("flash-again")).on_press(crate::app::Message::Restart);

        let done = widget::button::suggested(fl!("done")).on_press(crate::app::Message::Next);

        Some(
            widget::row([])
                .push(flash_again)
                .push(widget::space::horizontal())
                .push(done)
                .spacing(spacing().space_xs)
                .padding(spacing().space_xs)
                .into(),
        )
    }
}
