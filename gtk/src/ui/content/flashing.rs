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

        let topic = Label::new("Flashing");
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));

        let progress_list = Grid::new();
        let progress_scroller = ScrolledWindow::new(None, None);
        progress_scroller.add(&progress_list);

        let inner_container = Box::new(Orientation::Vertical, 0);
        inner_container.pack_start(&topic, false, false, 0);
        inner_container.pack_start(&progress_scroller, true, true, 0);

        let container = Box::new(Orientation::Horizontal, 0);
        container.pack_start(&image, false, false, 0);
        container.pack_start(&inner_container, true, true, 0);

        FlashView {
            container,
            progress_list,
        }
    }
}
