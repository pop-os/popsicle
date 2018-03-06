use gtk::*;
use std::path::PathBuf;

/// A wrapped FileChooserDialog that automatically destroys itself upon being dropped.
pub struct OpenDialog(FileChooserDialog);

impl OpenDialog {
    pub fn new(path: Option<PathBuf>) -> OpenDialog {
        // Create a new file chooser dialog for opening a file.
        let open_dialog = FileChooserDialog::new(
            Some("Open"),
            Some(&Window::new(WindowType::Popup)),
            FileChooserAction::Open,
        );

        let filter = FileFilter::new();
        filter.add_pattern("*.iso");
        filter.add_pattern("*.img");

        // Add the cancel and open buttons to that dialog.
        open_dialog.add_button("Cancel", ResponseType::Cancel.into());
        open_dialog.add_button("Open", ResponseType::Ok.into());
        open_dialog.set_filter(&filter);

        // Set the default path to open this with.
        path.map(|p| open_dialog.set_current_folder(p));

        OpenDialog(open_dialog)
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
