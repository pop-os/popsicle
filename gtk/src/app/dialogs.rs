use gtk::*;
use std::path::PathBuf;

/// A wrapped FileChooserDialog that automatically destroys itself upon being dropped.
pub struct OpenDialog(FileChooserDialog);

impl OpenDialog {
    pub fn new(path: Option<PathBuf>) -> OpenDialog {
        #[allow(unused_mut)]
        OpenDialog(cascade! {
            dialog: FileChooserDialog::new(
                Some("Open"),
                Some(&Window::new(WindowType::Popup)),
                FileChooserAction::Open,
            );
            ..add_button("Cancel", ResponseType::Cancel.into());
            ..add_button("Open", ResponseType::Ok.into());
            ..set_filter(&cascade! {
                FileFilter::new();
                ..add_pattern("*.iso");
                ..add_pattern("*.img");
            });
            | path.map(|p| dialog.set_current_folder(p));
        })
    }

    pub fn run(&self) -> Option<PathBuf> {
        if self.0.run() == ResponseType::Ok.into() {
            self.0.get_filename()
        } else {
            None
        }
    }
}

impl Drop for OpenDialog {
    fn drop(&mut self) { self.0.destroy(); }
}
