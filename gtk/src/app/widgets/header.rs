use crate::fl;
use gtk::{prelude::*, *};

pub struct Header {
    pub container: HeaderBar,
    pub back: Button,
    pub next: Button,
}

impl Header {
    pub fn new() -> Header {
        let back = cascade! {
            Button::with_label(&fl!("cancel"));
            ..style_context().add_class("back");
        };

        let next = cascade! {
            Button::with_label(&fl!("next"));
            ..set_sensitive(false);
            ..style_context().add_class(&STYLE_CLASS_SUGGESTED_ACTION);
        };

        // Returns the header and all of it's state
        Header {
            container: cascade! {
                HeaderBar::new();
                ..set_title(Some(&fl!("app-title")));
                ..pack_start(&back);
                ..pack_end(&next);
            },
            back,
            next,
        }
    }

    pub fn connect_back<F: Fn() + 'static>(&self, signal: F) {
        self.back.connect_clicked(move |_| signal());
    }

    pub fn connect_next<F: Fn() + 'static>(&self, signal: F) {
        self.next.connect_clicked(move |_| signal());
    }
}
