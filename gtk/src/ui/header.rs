use gtk::*;

pub struct Header {
    pub container: HeaderBar,
    pub back:      Button,
    pub next:      Button,
}

impl Header {
    pub fn new() -> Header {
        // Creates the main header bar container widget.
        let container = HeaderBar::new();

        // Sets the text to display in the title section of the header bar.
        container.set_title("Multiple USB File Flasher");

        let back = Button::new_with_label("Cancel");
        back.get_style_context().map(|c| c.add_class("back"));

        let next = Button::new_with_label("Next");
        next.get_style_context()
            .map(|c| c.add_class("suggested-action"));
        next.set_sensitive(false);

        container.pack_start(&back);
        container.pack_end(&next);

        // Returns the header and all of it's state
        Header {
            container,
            back,
            next,
        }
    }
}
