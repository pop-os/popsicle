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

        let topic = Label::new("Select drives");
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

        let desc_container = Box::new(Orientation::Vertical, 0);
        desc_container.pack_start(&topic, false, false, 0);
        desc_container.pack_start(&description, false, false, 0);
        desc_container.pack_start(&select_scroller, true, true, 0);

        let container = Box::new(Orientation::Horizontal, 0);
        container.pack_start(&image, false, false, 0);
        container.pack_start(&desc_container, true, true, 0);

        DevicesView {
            container,
            list,
            select_all,
        }
    }
}
