use gtk::*;

pub struct Content {
    pub container: Box,
}

impl Content {
    pub fn new() -> Content {
        let container = Box::new(Orientation::Horizontal, 0);

        Content { container }
    }
}
