use gtk::*;

pub struct Header {
    pub container: HeaderBar,
}

impl Header {
    pub fn new() -> Header {
        // Creates the main header bar container widget.
        let container = HeaderBar::new();

        // Sets the text to display in the title section of the header bar.
        container.set_title("Multiple USB File Flasher");
        // Enable the window controls within this headerbar.
        container.set_show_close_button(true);

        // Returns the header and all of it's state
        Header { container }
    }
}
