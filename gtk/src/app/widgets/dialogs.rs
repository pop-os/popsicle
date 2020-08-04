use gtk::*;
use std::path::PathBuf;

/// A wrapped FileChooserNative that automatically destroys itself upon being dropped.
pub struct OpenDialog(FileChooserNative);

impl OpenDialog {
    pub fn new(path: Option<PathBuf>) -> OpenDialog {
        #[allow(unused_mut)]
        OpenDialog(cascade! {
            dialog: FileChooserNative::new(
                Some("Open"),
                Some(&Window::new(WindowType::Popup)),
                FileChooserAction::Open,
                Some("Open"),
                Some("Cancel"),
            );
            ..set_filter(&cascade! {
                FileFilter::new();
                ..add_pattern("*.[Ii][Ss][Oo]");
                ..add_pattern("*.[Ii][Mm][Gg]");
            });
            | if let Some(p) = path {
                dialog.set_current_folder(p);
            };
        })
    }

    pub fn run(&self) -> Option<PathBuf> {
        if self.0.run() == ResponseType::Accept {
            self.0.get_filename()
        } else {
            None
        }
    }
}

impl Drop for OpenDialog {
    fn drop(&mut self) {
        self.0.destroy();
    }
}
