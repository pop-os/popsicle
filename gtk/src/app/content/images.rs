use super::View;
use gtk::*;
use pango::EllipsizeMode;
use super::set_margins;

pub struct ImageView {
    pub view:              View,
    pub chooser_container: Stack,
    pub chooser:           Button,
    pub image_path:        Label,
    pub hash:              ComboBoxText,
    pub hash_label:        Entry,
}

impl ImageView {
    pub fn new() -> ImageView {
        let chooser = Button::new_with_label("Choose Image");
        chooser.set_halign(Align::Center);
        chooser.set_margin_bottom(6);

        let image_path = Label::new("No image selected");
        image_path.set_ellipsize(EllipsizeMode::End);
        image_path.get_style_context().map(|c| c.add_class("bold"));

        let button_box = Box::new(Orientation::Vertical, 0);
        button_box.pack_start(&chooser, false, false, 0);
        button_box.pack_start(&image_path, false, false, 0);

        let spinner = Spinner::new();
        spinner.start();
        let spinner_label = Label::new("Loading Image");
        spinner_label
            .get_style_context()
            .map(|c| c.add_class("bold"));

        let spinner_box = Box::new(Orientation::Vertical, 0);
        spinner_box.pack_start(&spinner, false, false, 0);
        spinner_box.pack_start(&spinner_label, false, false, 0);

        let hash = ComboBoxText::new();
        hash.append_text("Type");
        hash.append_text("SHA256");
        hash.append_text("MD5");
        hash.set_active(0);

        let hash_label = Entry::new();
        hash_label.set_editable(false);

        let hash_container = Box::new(Orientation::Horizontal, 0);
        set_margins(&hash_container, 6);
    
        {
            let label = Label::new("Hash:");
            label.set_margin_right(6);

            let combo_container = Box::new(Orientation::Horizontal, 0);
            combo_container.get_style_context().map(|c| c.add_class("linked"));
            combo_container.add(&hash);
            combo_container.pack_start(&hash_label, true, true, 0);

            hash_container.pack_start(&label, false, false, 0);
            hash_container.pack_start(&combo_container, true, true, 0);
        }

        let chooser_container = Stack::new();
        chooser_container.add_named(&button_box, "chooser");
        chooser_container.add_named(&spinner_box, "loader");
        chooser_container.set_visible_child_name("chooser");
        chooser_container.set_margin_top(12);
        chooser_container.set_margin_bottom(24);

        let view = View::new(
            "application-x-cd-image",
            "Choose an Image",
            "Select the .iso or .img that you want to flash. You can also plug your USB drives in \
             now.",
            |right_panel| {
                right_panel.pack_start(&chooser_container, true, false, 0);
                right_panel.pack_start(&hash_container, false, false, 0);
            },
        );

        ImageView {
            view,
            chooser_container,
            chooser,
            image_path,
            hash,
            hash_label,
        }
    }
}
