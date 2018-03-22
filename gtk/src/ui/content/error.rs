use gtk::*;

pub struct ErrorView {
    pub container: Box,
    pub buffer:    TextBuffer,
}

impl ErrorView {
    pub fn new() -> ErrorView {
        let image = Image::new_from_icon_name("dialog-error", 6);
        image.set_valign(Align::Start);

        let topic = Label::new("Critical Error Occurred");
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));

        let description = TextView::new();
        description.set_halign(Align::Start);
        description.get_style_context().map(|c| c.add_class("desc"));

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
        right_panel.pack_start(&description, true, true, 0);

        let container = Box::new(Orientation::Horizontal, 0);
        container.pack_start(&left_panel, false, false, 0);
        container.pack_start(&right_panel, true, true, 0);

        ErrorView {
            container,
            buffer: description.get_buffer().unwrap(),
        }
    }
}
