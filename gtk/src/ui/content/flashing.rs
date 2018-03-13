use gtk::*;

pub struct FlashView {
    pub container:     Box,
    pub progress_list: Grid,
}

impl FlashView {
    pub fn new() -> FlashView {
        let image = Spinner::new();
        image.start();
        image.set_valign(Align::Start);

        let topic = Label::new("Flashing Devices");
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));

        let description = Label::new("Do not unplug devices while they are being flashed.");
        description.set_halign(Align::Start);

        let progress_list = Grid::new();
        let progress_scroller = ScrolledWindow::new(None, None);
        progress_scroller.add(&progress_list);

        let left_panel = Box::new(Orientation::Vertical, 0);
        left_panel
            .get_style_context()
            .map(|c| c.add_class("left-panel"));
        left_panel.pack_start(&image, false, false, 0);

        let right_panel = Box::new(Orientation::Vertical, 0);
        right_panel
            .get_style_context()
            .map(|c| c.add_class("right-panel"));
        right_panel.pack_start(&topic, false, false, 0);
        right_panel.pack_start(&description, false, false, 0);
        right_panel.pack_start(&progress_scroller, true, true, 0);

        let container = Box::new(Orientation::Horizontal, 0);
        container.pack_start(&left_panel, false, false, 0);
        container.pack_start(&right_panel, true, true, 0);

        FlashView {
            container,
            progress_list,
        }
    }
}
