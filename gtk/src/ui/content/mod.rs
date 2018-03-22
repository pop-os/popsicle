mod devices;
mod error;
mod flashing;
mod images;
mod summary;

pub use self::devices::DevicesView;
pub use self::error::ErrorView;
pub use self::flashing::FlashView;
pub use self::images::ImageView;
pub use self::summary::SummaryView;

use gtk::*;

pub struct Content {
    pub container:    Stack,
    pub image_view:   ImageView,
    pub devices_view: DevicesView,
    pub error_view:   ErrorView,
    pub flash_view:   FlashView,
    pub summary_view: SummaryView,
}

impl Content {
    pub fn new() -> Content {
        let container = Stack::new();

        let image_view = ImageView::new();
        let devices_view = DevicesView::new();
        let flash_view = FlashView::new();
        let summary_view = SummaryView::new();
        let error_view = ErrorView::new();

        container.add_named(&image_view.container, "image");
        container.add_named(&devices_view.container, "devices");
        container.add_named(&flash_view.container, "flash");
        container.add_named(&summary_view.container, "summary");
        container.add_named(&error_view.container, "error");
        container.set_visible_child_name("image");

        Content {
            container,
            image_view,
            devices_view,
            flash_view,
            summary_view,
            error_view,
        }
    }
}
