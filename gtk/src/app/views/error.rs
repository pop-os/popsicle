use super::View;
use crate::fl;

pub struct ErrorView {
    pub view: View,
}

impl ErrorView {
    pub fn new() -> ErrorView {
        ErrorView { view: View::new("dialog-error", &fl!("critical-error"), "", |_| ()) }
    }
}
