use gtk::*;

pub struct Content {
    pub container: Box,
    pub flash:     FlashElement,
    pub image:     ImageElement,
    pub usb:       UsbElement,
}

impl Content {
    pub fn new() -> Content {
        let container = Box::new(Orientation::Horizontal, 15);

        let flash = FlashElement::new();
        let image = ImageElement::new();
        let usb = UsbElement::new();

        let size_group = SizeGroup::new(SizeGroupMode::Both);
        size_group.add_widget(&image.image);
        size_group.add_widget(&usb.image);
        size_group.add_widget(&flash.image);

        container.pack_start(&image.container, true, true, 0);
        container.pack_start(&usb.container, true, true, 0);
        container.pack_start(&flash.container, true, true, 0);

        Content {
            container,
            flash,
            image,
            usb,
        }
    }
}

pub struct ImageElement {
    pub container: Box,
    pub title:     Label,
    pub image:     Image,
}

impl ImageElement {
    fn new() -> ImageElement {
        let container = Box::new(Orientation::Vertical, 0);
        let title = Label::new("Select Image");
        let image = Image::new_from_file("/usr/local/share/muff/image.png");

        container.pack_start(&title, false, false, 0);
        container.pack_start(&image, false, false, 0);

        ImageElement {
            container,
            title,
            image,
        }
    }
}

pub struct UsbElement {
    pub container: Box,
    pub title:     Label,
    pub image:     Image,
}

impl UsbElement {
    fn new() -> UsbElement {
        let container = Box::new(Orientation::Vertical, 0);
        let title = Label::new("Select Drives");
        let image = Image::new_from_file("/usr/local/share/muff/usb.png");

        container.pack_start(&title, false, false, 0);
        container.pack_start(&image, false, false, 0);

        UsbElement {
            container,
            title,
            image,
        }
    }
}

pub struct FlashElement {
    pub container: Box,
    pub title:     Label,
    pub image:     Image,
}

impl FlashElement {
    fn new() -> FlashElement {
        let container = Box::new(Orientation::Vertical, 0);
        let title = Label::new("Flash Drives");
        let image = Image::new_from_file("/usr/local/share/muff/flash.png");

        container.pack_start(&title, false, false, 0);
        container.pack_start(&image, false, false, 0);

        FlashElement {
            container,
            title,
            image,
        }
    }
}
