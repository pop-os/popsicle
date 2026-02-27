use super::View;
use crate::fl;
use gtk::prelude::*;

pub struct ErrorView {
    pub view: View,
}

impl ErrorView {
    pub fn new() -> ErrorView {
        let view = View::new("dialog-error", &fl!("critical-error"), "", |right_panel| {
            right_panel.set_margin_top(12);
        });

        view.description.set_line_wrap(true);
        view.description.set_max_width_chars(60);
        view.description.style_context().add_class("error-view-desc");

        ErrorView { view }
    }
}
