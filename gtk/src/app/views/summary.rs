use super::View;
use crate::fl;
use gtk::*;

pub struct SummaryView {
    pub view: View,
    pub list: ListBox,
}

impl SummaryView {
    pub fn new() -> SummaryView {
        let list = cascade! {
            ListBox::new();
            ..get_style_context().add_class("frame");
        };

        let view = View::new("process-completed", &fl!("flashing-completed"), "", |right_panel| {
            right_panel.pack_start(&list, true, true, 0);
        });

        SummaryView { view, list }
    }
}
