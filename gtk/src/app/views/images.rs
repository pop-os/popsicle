use super::View;
use crate::fl;
use bytesize;
use gtk::prelude::*;
use gtk::*;
use pango::{AttrList, Attribute, EllipsizeMode};
use std::path::Path;

pub struct ImageView {
    pub view: View,
    pub check: Button,
    pub chooser_container: Stack,
    pub chooser: Button,
    pub image_path: Label,
    pub hash: ComboBoxText,
    pub hash_label: Entry,
}

impl ImageView {
    pub fn new() -> ImageView {
        let chooser = cascade! {
            Button::with_label(&fl!("choose-image-button"));
            ..set_halign(Align::Center);
            ..set_margin_bottom(6);
        };

        let image_label = format!("<b>{}</b>", fl!("no-image-selected"));

        let image_path = cascade! {
            Label::new(Some(&image_label));
            ..set_use_markup(true);
            ..set_justify(Justification::Center);
            ..set_ellipsize(EllipsizeMode::End);
        };

        let button_box = cascade! {
            Box::new(Orientation::Vertical, 0);
            ..pack_start(&chooser, false, false, 0);
            ..pack_start(&image_path, false, false, 0);
        };

        let spinner = Spinner::new();
        spinner.start();

        let spinner_label = cascade! {
            Label::new(Some(&fl!("generating-checksum")));
            ..get_style_context().add_class("bold");
        };

        let spinner_box = cascade! {
            Box::new(Orientation::Vertical, 0);
            ..pack_start(&spinner, false, false, 0);
            ..pack_start(&spinner_label, false, false, 0);
        };

        let hash = cascade! {
            ComboBoxText::new();
            ..append_text(&fl!("none"));
            ..append_text("SHA256");
            ..append_text("MD5");
            ..set_active(Some(0));
            ..set_sensitive(false);
        };

        let hash_label = cascade! {
            Entry::new();
            ..set_sensitive(false);
        };

        let label = cascade! {
            Label::new(Some(&fl!("hash-label")));
            ..set_margin_end(6);
        };

        let check = cascade! {
            Button::with_label(&fl!("check-label"));
            ..get_style_context().add_class(&STYLE_CLASS_SUGGESTED_ACTION);
            ..set_sensitive(false);
        };

        let hash_label_clone = hash_label.clone();
        let check_clone = check.clone();
        hash.connect_changed(move |combo_box| {
            let sensitive = match combo_box.get_active_text() {
                Some(text) if text.as_str() != "None" => true,
                _ => false,
            };
            hash_label_clone.set_sensitive(sensitive);
            check_clone.set_sensitive(sensitive);
        });

        let combo_container = cascade! {
            Box::new(Orientation::Horizontal, 0);
            ..add(&hash);
            ..pack_start(&hash_label, true, true, 0);
            ..get_style_context().add_class("linked");
        };

        let hash_container = cascade! {
            let tmp = Box::new(Orientation::Horizontal, 0);
            ..pack_start(&label, false, false, 0);
            ..pack_start(&combo_container, true, true, 0);
            ..pack_start(&check, false, false, 0);
            ..set_border_width(6);
        };

        let chooser_container = cascade! {
            Stack::new();
            ..add_named(&button_box, "chooser");
            ..add_named(&spinner_box, "checksum");
            ..set_visible_child_name("chooser");
            ..set_margin_top(12);
            ..set_margin_bottom(24);
        };

        let view = View::new(
            "application-x-cd-image",
            &fl!("image-view-title"),
            &fl!("image-view-description"),
            |right_panel| {
                right_panel.pack_start(&chooser_container, true, false, 0);
                right_panel.pack_start(&hash_container, false, false, 0);
            },
        );

        ImageView { view, check, chooser_container, chooser, image_path, hash, hash_label }
    }

    pub fn set_hash_sensitive(&self, sensitive: bool) {
        self.hash.set_sensitive(sensitive);
    }

    pub fn set_hash(&self, hash: &str) {
        let text = self.hash_label.get_text();
        if !text.is_empty() {
            if let Some(fg) = if text.eq_ignore_ascii_case(hash) {
                Attribute::new_foreground(0, std::u16::MAX, 0)
            } else {
                Attribute::new_foreground(std::u16::MAX, 0, 0)
            } {
                let attrs = AttrList::new();
                attrs.insert(fg);
                self.hash_label.set_attributes(&attrs);
            }
        } else {
            self.hash_label.set_text(hash);
        }
    }

    pub fn set_image(&self, path: &Path, size: u64, warning: Option<&str>) {
        let size_str = bytesize::to_string(size, true);
        let mut label: String = match path.file_name() {
            Some(name) => format!("<b>{}</b>\n{}", name.to_string_lossy(), size_str),
            None => format!("<b>{}</b>", fl!("cannot-select-directories")),
        };

        if let Some(warning) = warning {
            let subject = fl!("warning");
            label += &format!("\n<span foreground='red'><b>{}</b>: {}</span>", subject, warning);
        };

        self.image_path.set_markup(&label);
    }
}
