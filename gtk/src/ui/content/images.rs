use gtk::*;
use pango::EllipsizeMode;

pub struct ImageView {
    pub container:  Box,
    pub chooser:    Button,
    pub image_path: Label,
    pub hash:       ComboBoxText,
    pub hash_label: Entry,
}

impl ImageView {
    pub fn new() -> ImageView {
        let image = Image::new_from_icon_name("application-x-cd-image", 6);
        image.set_valign(Align::Start);

        let topic = Label::new("Choose an Image");
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));

        let description = Label::new(
            "Select the .iso or .img that you want to flash. You can also plug your USB drives in \
             now.",
        );
        description.set_line_wrap(true);
        description.set_halign(Align::Start);
        description.get_style_context().map(|c| c.add_class("desc"));

        let chooser = Button::new_with_label("Choose Image");
        chooser.set_halign(Align::Center);
        chooser.set_halign(Align::Center);

        let image_path = Label::new("No image selected");
        image_path.set_ellipsize(EllipsizeMode::End);
        image_path.get_style_context().map(|c| c.add_class("bold"));

        let hash = ComboBoxText::new();
        hash.append_text("Type");
        hash.append_text("SHA256");
        hash.append_text("MD5");
        hash.set_active(0);

        let hash_label = Entry::new();
        hash_label.set_editable(false);

        let hash_container = Box::new(Orientation::Horizontal, 0);
        hash_container
            .get_style_context()
            .map(|c| c.add_class("hash-box"));
        hash_container.pack_start(&hash, false, false, 0);
        hash_container.pack_start(&hash_label, true, true, 0);

        let chooser_container = Box::new(Orientation::Vertical, 5);
        chooser_container.pack_start(&chooser, false, false, 0);
        chooser_container.pack_start(&image_path, false, false, 0);

        let inner_container = Box::new(Orientation::Vertical, 0);
        inner_container.pack_start(&topic, false, false, 0);
        inner_container.pack_start(&description, false, false, 0);
        inner_container.pack_start(&chooser_container, true, false, 0);
        inner_container.pack_start(&hash_container, false, false, 0);

        let container = Box::new(Orientation::Horizontal, 5);
        container.pack_start(&image, false, false, 0);
        container.pack_start(&inner_container, true, true, 0);

        ImageView {
            container,
            chooser,
            image_path,
            hash,
            hash_label,
        }
    }
}
