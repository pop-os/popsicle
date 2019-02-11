use gtk::prelude::*;
use gtk::{self, Align, Image, Label, Orientation};

pub struct View {
    pub container:   gtk::Box,
    pub icon:        Image,
    pub topic:       Label,
    pub description: Label,
    pub panel:       gtk::Box,
}

impl View {
    pub fn new<F: Fn(&gtk::Box)>(
        icon: &str,
        topic: &str,
        description: &str,
        configure_panel: F,
    ) -> View {
        let icon = Image::new_from_icon_name(icon, 6);
        icon.set_valign(Align::Start);

        let topic = cascade! {
            Label::new(topic);
            ..set_halign(Align::Start);
            ..get_style_context().map(|c| c.add_class("h2"));
            ..set_margin_bottom(6);
        };

        let description = cascade! {
            Label::new(description);
            ..set_line_wrap(true);
            ..set_xalign(0.0);
            ..get_style_context().map(|c| c.add_class("desc"));
            ..set_margin_bottom(6);
        };

        let left_panel = cascade! {
            gtk::Box::new(Orientation::Vertical, 0);
            ..add(&icon);
            ..get_style_context().map(|c| c.add_class("left-panel"));
        };

        let right_panel = cascade! {
            panel: gtk::Box::new(Orientation::Vertical, 0);
            ..add(&topic);
            ..add(&description);
            ..get_style_context().map(|c| c.add_class("right-panel"));
            | configure_panel(&panel);
        };

        View {
            container: cascade! {
                gtk::Box::new(Orientation::Horizontal, 12);
                ..pack_start(&left_panel, false, false, 0);
                ..pack_start(&right_panel, true, true, 0);
            },
            icon,
            topic,
            description,
            panel: right_panel,
        }
    }
}
