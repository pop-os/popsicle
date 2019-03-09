use crate::app::App;
use gtk::prelude::*;

impl App {
    pub fn connect_view_ready(&self) {
        let next = self.ui.header.next.clone();
        self.ui.content.devices_view.connect_view_ready(move |ready| next.set_sensitive(ready));
    }
}
