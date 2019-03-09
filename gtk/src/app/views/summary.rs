use super::View;
use gtk::*;

pub struct SummaryView {
    pub view: View,
    pub list: ListBox,
}

impl SummaryView {
    pub fn new() -> SummaryView {
        let list = ListBox::new();
        list.set_visible(false);

        let view = View::new("process-completed", "Flashing Completed", "", |right_panel| {
            right_panel.pack_start(&list, true, true, 0);
        });

        SummaryView { view, list }
    }
}
