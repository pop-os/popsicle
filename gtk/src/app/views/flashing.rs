use super::View;
use gtk::*;

pub struct FlashView {
    pub view:          View,
    pub progress_list: Grid,
}

impl FlashView {
    pub fn new() -> FlashView {
        let progress_list = cascade! {
            Grid::new();
            ..set_row_spacing(6);
            ..set_column_spacing(6);
        };

        let progress_scroller = cascade! {
            ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
            ..add(&progress_list);
        };

        let view = View::new(
            "drive-removable-media-usb",
            "Flashing Devices",
            "Do not unplug devices while they are being flashed.",
            |right_panel| right_panel.pack_start(&progress_scroller, true, true, 0),
        );

        FlashView {
            view,
            progress_list,
        }
    }
}
