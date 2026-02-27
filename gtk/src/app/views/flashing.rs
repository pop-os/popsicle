use super::View;
use crate::fl;
use gtk::{prelude::*, *};

pub struct FlashView {
    pub view: View,
    pub progress_list: Grid,
    pub overall_bar: ProgressBar,
    pub overall_label: Label,
    pub elapsed_label: Label,
}

impl FlashView {
    pub fn new() -> FlashView {
        let overall_bar = cascade! {
            ProgressBar::new();
            ..set_hexpand(true);
        };

        let overall_label = cascade! {
            Label::new(Some(&fl!("overall-progress", percent = 0)));
            ..set_halign(Align::End);
            ..style_context().add_class("bold");
        };

        let overall_container = cascade! {
            Box::new(Orientation::Horizontal, 12);
            ..pack_start(&overall_bar, true, true, 0);
            ..pack_start(&overall_label, false, false, 0);
            ..style_context().add_class("overall-progress");
            ..set_margin_bottom(12);
        };

        let elapsed_label = cascade! {
            Label::new(Some(&fl!("elapsed-time", time = "0:00")));
            ..set_halign(Align::Start);
            ..style_context().add_class("caption");
            ..set_margin_bottom(12);
        };

        let progress_list = cascade! {
            Grid::new();
            ..set_row_spacing(6);
            ..set_column_spacing(6);
            ..style_context().add_class("progress-container");
        };

        let progress_scroller = cascade! {
            ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
            ..add(&progress_list);
        };

        let view = View::new(
            "drive-removable-media-usb",
            &fl!("flash-view-title"),
            &fl!("flash-view-description"),
            |right_panel| {
                right_panel.pack_start(&overall_container, false, false, 0);
                right_panel.pack_start(&elapsed_label, false, false, 0);
                right_panel.pack_start(&progress_scroller, true, true, 0);
            },
        );

        FlashView { view, progress_list, overall_bar, overall_label, elapsed_label }
    }
}
