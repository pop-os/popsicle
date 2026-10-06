// SPDX-License-Identifier: MIT

use cosmic::{
    Apply, Element,
    iced::{Alignment, Length},
    theme::spacing,
    widget,
};

use crate::{app::ActiveView, fl};

#[derive(Debug, Default)]
pub struct ErrorView {
    description: String,
}

#[derive(Debug, Clone)]
pub enum Message {}

impl ErrorView {
    pub fn set_error(&mut self, description: impl Into<String>) {
        self.description = description.into();
    }

    pub fn view(&self) -> Element<'_, Message> {
        let space = spacing();

        widget::column([])
            .push(widget::icon::from_name("dialog-error-symbolic").size(64).symbolic(true))
            .push(widget::text::title2(fl!("critical-error")))
            .push(widget::text::body(self.description.as_str()).align_x(Alignment::Center))
            .align_x(Alignment::Center)
            .spacing(space.space_s)
            .width(Length::Fill)
            .apply(widget::scrollable)
            .into()
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, crate::app::Message>> {
        if *view != ActiveView::Error {
            return None;
        }

        let flash_again =
            widget::button::standard(fl!("flash-again")).on_press(crate::app::Message::Restart);

        let close = widget::button::standard(fl!("close")).on_press(crate::app::Message::Next);

        Some(
            widget::row([])
                .push(flash_again)
                .push(widget::space::horizontal())
                .push(close)
                .spacing(spacing().space_xs)
                .padding(spacing().space_xs)
                .into(),
        )
    }
}
