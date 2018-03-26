use super::View;
use gtk::*;

pub struct DevicesView {
    pub view:       View,
    pub list:       ListBox,
    pub select_all: CheckButton,
}

impl DevicesView {
    pub fn new() -> DevicesView {
        let select_all = CheckButton::new_with_label("Select all");
        let list = ListBox::new();
        list.get_style_context().map(|c| c.add_class("devices"));

        let list_box = Box::new(Orientation::Vertical, 0);
        list_box.pack_start(&select_all, false, false, 0);
        list_box.pack_start(&Separator::new(Orientation::Horizontal), false, false, 0);
        list_box.pack_start(&list, true, true, 0);

        let select_scroller = ScrolledWindow::new(None, None);
        select_scroller.add(&list_box);

        let view = View::new(
            "drive-removable-media-usb",
            "Select Drives",
            "Flashing will erase all data on the selected drives.",
            |right_panel| right_panel.pack_start(&select_scroller, true, true, 0),
        );

        DevicesView {
            view,
            list,
            select_all,
        }
    }
}
