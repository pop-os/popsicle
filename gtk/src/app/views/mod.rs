mod devices;
mod error;
mod flashing;
mod images;
mod summary;
mod view;

pub use self::devices::DevicesView;
pub use self::error::ErrorView;
pub use self::flashing::FlashView;
pub use self::images::ImageView;
pub use self::summary::SummaryView;
pub use self::view::View;

use gtk::{prelude::*, *};

pub struct Content {
    pub container: Stack,
    pub image_view: ImageView,
    pub devices_view: DevicesView,
    pub error_view: ErrorView,
    pub flash_view: FlashView,
    pub summary_view: SummaryView,
}

impl Content {
    pub fn new() -> Content {
        let image_view = ImageView::new();
        let devices_view = DevicesView::new();
        let flash_view = FlashView::new();
        let summary_view = SummaryView::new();
        let error_view = ErrorView::new();

        let container = cascade! {
            Stack::new();
            ..add(&image_view.view.container);
            ..add(&devices_view.view.container);
            ..add(&flash_view.view.container);
            ..add(&summary_view.view.container);
            ..add(&error_view.view.container);
            ..set_visible_child(&image_view.view.container);
            ..set_border_width(12);
        };

        Content { container, image_view, devices_view, flash_view, summary_view, error_view }
    }
}
