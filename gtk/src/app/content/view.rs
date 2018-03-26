use gtk::prelude::*;
use gtk::{self, Align, Image, Label, Orientation};

pub struct View {
    pub container:   gtk::Box,
    pub icon:        gtk::Image,
    pub topic:       gtk::Label,
    pub description: gtk::Label,
    pub panel:       gtk::Box,
}

impl View {
    pub fn new(icon: &str, topic: &str, description: &str) -> View {
        let icon = Image::new_from_icon_name(icon, 6);
        icon.set_valign(Align::Start);

        let topic = Label::new(topic);
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));
        topic.set_margin_bottom(6);

        let description = Label::new(description);
        description.set_line_wrap(true);
        description.set_halign(Align::Start);
        description.get_style_context().map(|c| c.add_class("desc"));
        description.set_margin_bottom(6);

        let left_panel = gtk::Box::new(Orientation::Vertical, 0);
        left_panel
            .get_style_context()
            .map(|c| c.add_class("left-panel"));
        left_panel.pack_start(&icon, false, false, 0);

        let right_panel = gtk::Box::new(Orientation::Vertical, 0);
        right_panel
            .get_style_context()
            .map(|c| c.add_class("right-panel"));
        right_panel.pack_start(&topic, false, false, 0);
        right_panel.pack_start(&description, false, false, 0);

        let container = gtk::Box::new(Orientation::Horizontal, 12);
        container.pack_start(&left_panel, false, false, 0);
        container.pack_start(&right_panel, true, true, 0);

        View {
            container,
            icon,
            topic,
            description,
            panel: right_panel,
        }
    }
}
