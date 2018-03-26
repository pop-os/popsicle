use super::View;
use gtk::*;

pub struct DevicesView {
    pub view:       View,
    pub list:       ListBox,
    pub select_all: CheckButton,
}

impl DevicesView {
    pub fn new() -> DevicesView {
        let view = View::new(
            "drive-removable-media-usb",
            "Select Drives",
            "Flashing will erase all data on the selected drives.",
        );

        let select_all = CheckButton::new_with_label("Select all");
        let list = ListBox::new();
        list.insert(&select_all, -1);
        list.get_style_context().map(|c| c.add_class("devices"));

        let select_scroller = ScrolledWindow::new(None, None);
        select_scroller.add(&list);

        {
            let right_panel = &view.panel;
            right_panel.pack_start(&select_scroller, true, true, 0);
        }

        DevicesView {
            view,
            list,
            select_all,
        }
    }
}
