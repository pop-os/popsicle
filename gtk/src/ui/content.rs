use gtk::*;
use pango::EllipsizeMode;

pub struct Content {
    pub container:  Stack,
    pub image_view: ImageView,
    pub devices_view: DevicesView,
}

impl Content {
    pub fn new() -> Content {
        let container = Stack::new();

        let image_view = ImageView::new();
        let devices_view = DevicesView::new();

        container.add_named(&image_view.container, "image");
        container.add_named(&devices_view.container, "devices");
        container.set_visible_child_name("image");

        Content {
            container,
            image_view,
            devices_view,
        }
    }
}

pub struct DevicesView {
    pub container: Box,
    pub list: ListBox,
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

        let select_all = CheckButton::new_with_label("Select All");
        let list = ListBox::new();
        list.insert(&select_all, -1);

        let select_scroller = ScrolledWindow::new(None, None);
        select_scroller.add(&list);

        let desc_container = Box::new(Orientation::Vertical, 0);
        desc_container.pack_start(&topic, false, false, 0);
        desc_container.pack_start(&description, false, false, 0);
        desc_container.pack_start(&select_scroller, true, true, 0);

        let container = Box::new(Orientation::Horizontal, 0);
        container.pack_start(&image, false, false, 0);
        container.pack_start(&desc_container, true, true, 0);

        DevicesView { container, list, select_all }
    }
}

pub struct ImageView {
    pub container:   Box,
    pub chooser:     Button,
    pub image_path:  Label,
    pub hash:        ComboBoxText,
    pub hash_button: Button,
    pub hash_label:  Label,
}

impl ImageView {
    pub fn new() -> ImageView {
        let image = Image::new_from_icon_name("application-x-cd-image", 6);
        image.set_valign(Align::Start);

        let topic = Label::new("Choose an image");
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));

        let description = Label::new(
            "Select the .iso or .img that you want to flash. You can also plug your USB drives in \
             now.",
        );
        description.set_line_wrap(true);
        description.set_halign(Align::Start);

        let chooser = Button::new_with_label("Choose Image");
        chooser.set_halign(Align::Center);
        chooser.set_halign(Align::Center);

        let image_path = Label::new("No image selected");
        image_path.set_ellipsize(EllipsizeMode::End);
        image_path.get_style_context().map(|c| c.add_class("bold"));

        let hash = ComboBoxText::new();
        hash.append_text("SHA256");
        hash.append_text("MD5");
        hash.set_active(0);

        let hash_button = Button::new_with_label("Generate");
        hash_button.set_sensitive(false);
        let hash_label = Label::new("");
        hash_label.set_selectable(true);

        let hash_container = Box::new(Orientation::Horizontal, 0);
        hash_container.pack_start(&hash, false, false, 0);
        hash_container.pack_start(&hash_button, false, false, 0);
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
            hash_button,
            hash_label,
        }
    }
}
