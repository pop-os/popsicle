// SPDX-License-Identifier: MIT

use cosmic::{
    iced::{Border, Color},
    theme::TextInput,
    widget::{
        self,
        text_input::{Appearance, StyleSheet},
    },
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

pub fn colored_text_input(result: HashResult) -> TextInput {
    let color = move |theme: &cosmic::Theme, appearance: Appearance| {
        let cosmic = theme.cosmic();
        let text_color: Option<Color> = match result {
            HashResult::Checking => return appearance,
            HashResult::Match => Some(cosmic.success_color().into()),
            HashResult::Mismatch | HashResult::Error => Some(cosmic.destructive_color().into()),
        };
        Appearance { text_color, ..appearance }
    };

    TextInput::Custom {
        active: Box::new(move |t| color(t, t.active(&TextInput::Default))),
        error: Box::new(move |t| color(t, t.error(&TextInput::Default))),
        hovered: Box::new(move |t| color(t, t.hovered(&TextInput::Default))),
        focused: Box::new(move |t| color(t, t.focused(&TextInput::Default))),
        disabled: Box::new(move |t| color(t, t.disabled(&TextInput::Default))),
    }
}
