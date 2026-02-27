use super::View;
use crate::fl;
use bytesize;
use gtk::prelude::*;
use gtk::*;
use pango::{AttrColor, AttrList, EllipsizeMode};
use std::path::Path;

pub struct ImageView {
    pub view: View,
    pub check: Button,
    pub chooser_container: Stack,
    pub chooser: Button,
    pub change_button: Button,
    pub drop_zone: Box,
    pub drop_zone_event_box: EventBox,
    pub image_path: Label,
    pub hash: ComboBoxText,
    pub hash_label: Entry,
}

impl ImageView {
    pub fn new() -> ImageView {
        // Drop zone: unselected state with icon and instructional label
        let drop_icon = Image::from_icon_name(Some("document-open-symbolic"), IconSize::Dnd);
        drop_icon.set_opacity(0.4);

        let drop_label = cascade! {
            Label::new(Some(&fl!("choose-image-button")));
            ..style_context().add_class("h2");
            ..set_opacity(0.6);
        };

        let drop_sublabel = cascade! {
            Label::new(Some(&fl!("image-view-description-drop")));
            ..style_context().add_class("subtitle");
        };

        let drop_zone = cascade! {
            Box::new(Orientation::Vertical, 8);
            ..set_halign(Align::Center);
            ..set_valign(Align::Center);
            ..add(&drop_icon);
            ..add(&drop_label);
            ..add(&drop_sublabel);
            ..style_context().add_class("drop-zone");
        };

        let drop_zone_event_box = cascade! {
            EventBox::new();
            ..add(&drop_zone);
        };

        // Drop zone: selected state with file info and change button
        let file_icon = Image::from_icon_name(Some("application-x-cd-image"), IconSize::Dnd);

        let image_path = cascade! {
            Label::new(Some(&format!("<b>{}</b>", fl!("no-image-selected"))));
            ..set_use_markup(true);
            ..set_justify(Justification::Left);
            ..set_ellipsize(EllipsizeMode::End);
            ..set_halign(Align::Start);
        };

        let change_button = cascade! {
            Button::with_label(&fl!("change-image-button"));
            ..style_context().add_class("link-button");
            ..set_halign(Align::Start);
        };

        let file_info_box = cascade! {
            Box::new(Orientation::Vertical, 4);
            ..set_valign(Align::Center);
            ..add(&image_path);
            ..add(&change_button);
        };

        let drop_zone_selected = cascade! {
            Box::new(Orientation::Horizontal, 12);
            ..set_halign(Align::Fill);
            ..set_valign(Align::Center);
            ..add(&file_icon);
            ..add(&file_info_box);
            ..style_context().add_class("drop-zone-selected");
        };

        // Chooser is still used for opening file dialog
        let chooser = cascade! {
            Button::with_label(&fl!("choose-image-button"));
            ..set_halign(Align::Center);
            ..set_no_show_all(true);
            ..hide();
        };

        let spinner = Spinner::new();
        spinner.start();

        let spinner_label = cascade! {
            Label::new(Some(&fl!("generating-checksum")));
            ..style_context().add_class("bold");
        };

        let spinner_box = cascade! {
            Box::new(Orientation::Vertical, 0);
            ..pack_start(&spinner, false, false, 0);
            ..pack_start(&spinner_label, false, false, 0);
        };

        let hash = cascade! {
            ComboBoxText::new();
            ..append_text(&fl!("none"));
            ..append_text("SHA512");
            ..append_text("SHA256");
            ..append_text("SHA1");
            ..append_text("MD5");
            ..append_text("BLAKE2b");
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
            ..style_context().add_class(&STYLE_CLASS_SUGGESTED_ACTION);
            ..set_sensitive(false);
        };

        let hash_label_clone = hash_label.clone();
        let check_clone = check.clone();
        hash.connect_changed(move |combo_box| {
            let sensitive = combo_box.active_text().is_some_and(|text| text.as_str() != "None");

            hash_label_clone.set_sensitive(sensitive);
            check_clone.set_sensitive(sensitive);
        });

        let combo_container = cascade! {
            Box::new(Orientation::Horizontal, 0);
            ..add(&hash);
            ..pack_start(&hash_label, true, true, 0);
            ..style_context().add_class("linked");
        };

        let hash_container = cascade! {
            let tmp = Box::new(Orientation::Horizontal, 0);
            ..pack_start(&label, false, false, 0);
            ..pack_start(&combo_container, true, true, 0);
            ..pack_start(&check, false, false, 0);
            ..set_border_width(6);
            ..style_context().add_class("hash-row");
        };

        let chooser_container = cascade! {
            Stack::new();
            ..add_named(&drop_zone_event_box, "chooser");
            ..add_named(&drop_zone_selected, "selected");
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

        ImageView {
            view,
            check,
            chooser_container,
            chooser,
            change_button,
            drop_zone,
            drop_zone_event_box,
            image_path,
            hash,
            hash_label,
        }
    }

    pub fn set_hash_sensitive(&self, sensitive: bool) {
        self.hash.set_sensitive(sensitive);
    }

    pub fn set_hash(&self, hash: &str) {
        let text = self.hash_label.text();
        if !text.is_empty() {
            let fg = if text.eq_ignore_ascii_case(hash) {
                AttrColor::new_foreground(0, u16::MAX, 0)
            } else {
                AttrColor::new_foreground(u16::MAX, 0, 0)
            };
            let attrs = AttrList::new();
            attrs.insert(fg);
            self.hash_label.set_attributes(&attrs);
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
        self.chooser_container.set_visible_child_name("selected");
    }
}
