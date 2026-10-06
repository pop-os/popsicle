use std::{cell::RefCell, fs::File};

use cosmic::{Element, widget};

#[derive(Debug, Default)]
pub struct ImagesView {
    image: RefCell<Option<File>>,
}

#[derive(Debug, Clone)]
pub enum Message {}

impl ImagesView {
    pub fn view<'a>(&self) -> impl Into<Element<'a, Message>> {
        widget::text("Images...")
    }

    pub fn image_selected(&self) -> bool {
        self.image.borrow().is_some()
    }
}
