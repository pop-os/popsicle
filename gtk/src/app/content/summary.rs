use gtk::*;

pub struct SummaryView {
    pub container:   Box,
    pub description: Label,
    pub list:        ListBox,
}

impl SummaryView {
    pub fn new() -> SummaryView {
        let image = Image::new_from_icon_name("process-completed", 6);
        image.set_valign(Align::Start);

        let topic = Label::new("Flashing Completed");
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));

        let description = Label::new("");
        description.get_style_context().map(|c| c.add_class("desc"));
        description.set_halign(Align::Start);

        let list = ListBox::new();
        list.set_visible(false);

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
        right_panel.pack_start(&list, true, true, 0);

        let container = Box::new(Orientation::Horizontal, 0);
        container.pack_start(&left_panel, false, false, 0);
        container.pack_start(&right_panel, true, true, 0);

        SummaryView {
            container,
            description,
            list,
        }
    }
}
