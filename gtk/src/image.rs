use super::ui::State;
use gtk;
use gtk::prelude::*;
use muff::Image;
use std::path::Path;
use std::sync::Arc;

pub fn load_image<P: AsRef<Path>>(path: P, state: &State, label: &gtk::Label, next: &gtk::Button) {
    let path = path.as_ref();
    let mut new_image = match Image::new(path) {
        Ok(image) => image,
        Err(why) => {
            eprintln!("muff: unable to open image: {}", why);
            return;
        }
    };

    let new_data = match new_image.read(|_| ()) {
        Ok(data) => data,
        Err(why) => {
            eprintln!("muff: unable to read image: {}", why);
            return;
        }
    };

    *state.image_data.borrow_mut() = Some(Arc::new(new_data));
    label.set_label(&path.file_name().unwrap().to_string_lossy());
    next.set_sensitive(true);
}
