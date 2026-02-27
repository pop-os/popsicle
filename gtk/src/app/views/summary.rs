use super::View;
use crate::fl;
use gtk::{prelude::*, *};

pub struct SummaryView {
    pub view: View,
    pub list: ListBox,
    pub result_container: Box,
}

impl SummaryView {
    pub fn new() -> SummaryView {
        let list = cascade! {
            ListBox::new();
            ..style_context().add_class("frame");
        };

        let result_container = cascade! {
            Box::new(Orientation::Vertical, 0);
        };

        let view = View::new("process-completed", &fl!("flashing-completed"), "", |right_panel| {
            right_panel.pack_start(&result_container, false, false, 0);
            right_panel.pack_start(&list, true, true, 0);
        });

        SummaryView { view, list, result_container }
    }
}
