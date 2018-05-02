use super::View;
use gtk::*;

pub struct FlashView {
    pub view:          View,
    pub progress_list: Grid,
}

impl FlashView {
    pub fn new() -> FlashView {
        let progress_list = Grid::new();
        progress_list.set_row_spacing(6);
        progress_list.set_column_spacing(6);
        let progress_scroller = ScrolledWindow::new(None, None);
        progress_scroller.add(&progress_list);

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
