use gtk::*;

pub struct DevicesView {
    pub container:  Box,
    pub list:       ListBox,
    pub select_all: CheckButton,
}

impl DevicesView {
    pub fn new() -> DevicesView {
        let image = Image::new_from_icon_name("drive-removable-media-usb", 6);
        image.set_valign(Align::Start);

        let topic = Label::new("Select Drives");
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));

        let description = Label::new("Flashing will erase all data on the selected drives.");
        description.set_line_wrap(true);
        description.set_halign(Align::Start);
        description.get_style_context().map(|c| c.add_class("desc"));

        let select_all = CheckButton::new_with_label("Select All");
        let list = ListBox::new();
        list.insert(&select_all, -1);
        list.get_style_context().map(|c| c.add_class("devices"));

        let select_scroller = ScrolledWindow::new(None, None);
        select_scroller.add(&list);

        let left_panel = Box::new(Orientation::Vertical, 0);
        left_panel
            .get_style_context()
            .map(|c| c.add_class("left-panel"));
        left_panel.pack_start(&image, false, false, 0);

        let right_panel = Box::new(Orientation::Vertical, 0);
        right_panel
            .get_style_context()
            .map(|c| c.add_class("right-panel"));
        right_panel.pack_start(&topic, false, false, 0);
        right_panel.pack_start(&description, false, false, 0);
        right_panel.pack_start(&select_scroller, true, true, 0);

        let container = Box::new(Orientation::Horizontal, 0);
        container.pack_start(&left_panel, false, false, 0);
        container.pack_start(&right_panel, true, true, 0);

        DevicesView {
            container,
            list,
            select_all,
        }
    }
}
