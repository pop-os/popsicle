use cosmic::{
    Theme,
    iced::{Border, Color, widget::text_input},
    widget,
};

use crate::hash::HashResult;

pub fn drag_area_active(theme: &cosmic::prelude::Theme) -> widget::popover::Style {
    let mut style = widget::container::Style::default();

    let cosmic = theme.cosmic();

    let mut background = cosmic.accent_color();
    background.alpha = 0.2;

    style.background = Some(Color::from(background).into());

    style.border = Border {
        color: cosmic.accent_color().into(),
        width: 1.0,
        radius: cosmic.radius_s().into(),
    };

    style
}

pub fn hash_input_style(
    result: Option<HashResult>,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |theme, status| {
        let mut style = <Theme as text_input::Catalog>::style(
            theme,
            &cosmic::style::iced::TextInput::Default,
            status,
        );

        style.value = match result {
            Some(HashResult::Match) => theme.cosmic().success.base.into(),

            Some(HashResult::Mismatch | HashResult::Error) => {
                theme.cosmic().destructive.base.into()
            }

            _ => style.value,
        };

        style
    }
}
