use super::View;
use crate::block::BlockDevice;
use gtk;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use bytesize;

type ViewReadySignal = Rc<RefCell<Box<dyn Fn(bool)>>>;

pub struct DevicesView {
    pub view: View,
    pub list: gtk::ListBox,
    pub select_all: gtk::CheckButton,
    view_ready: ViewReadySignal,
}

impl DevicesView {
    pub fn new() -> DevicesView {
        let list = cascade! {
            gtk::ListBox::new();
            ..get_style_context().add_class("devices");
            ..set_hexpand(true);
            ..set_vexpand(true);
        };

        let list_ = list.clone();
        let select_all = cascade! {
            gtk::CheckButton::new_with_label("Select all");
            ..set_margin_start(4);
            ..set_margin_bottom(3);
            ..connect_toggled(move |all| {
                let state = all.get_active();

                for row in list_.get_children() {
                    if let Ok(row) = row.downcast::<gtk::ListBoxRow>() {
                        if let Some(widget) = row.get_children().get(0) {
                            if let Some(button) = widget.downcast_ref::<gtk::CheckButton>() {
                                button.set_active(button.get_sensitive() && state);
                            }
                        }
                    }
                }
            });
        };

        let list_box = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 0);
            ..add(&select_all);
            ..add(&gtk::Separator::new(gtk::Orientation::Horizontal));
            ..add(&list);
        };

        let select_scroller = cascade! {
            gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
            ..set_hexpand(true);
            ..set_vexpand(true);
            ..add(&list_box);
        };

        let view = View::new(
            "drive-removable-media-usb",
            "Select Drives",
            "Flashing will erase all data on the selected drives.",
            |right_panel| right_panel.add(&select_scroller),
        );

        let view_ready: ViewReadySignal = Rc::new(RefCell::new(Box::new(|_| ())));

        DevicesView { view, list, select_all, view_ready }
    }

    pub fn get_buttons(&self) -> impl Iterator<Item = gtk::CheckButton> {
        self.list
            .get_children()
            .into_iter()
            .filter_map(|row| row.downcast::<gtk::ListBoxRow>().ok())
            .filter_map(|row| row.get_children().get(0).cloned())
            .filter_map(|row| row.downcast::<gtk::CheckButton>().ok())
    }

    pub fn get_active_ids(&self) -> impl Iterator<Item = usize> {
        self.get_buttons()
            .enumerate()
            .filter_map(|(id, button)| if button.get_active() { Some(id) } else { None })
    }

    pub fn refresh(&self, devices: &[BlockDevice], image_size: u64) {
        self.list.foreach(WidgetExt::destroy);

        let nselected = Rc::new(Cell::new(0));

        for device in devices {
            let valid_size = device.size_in_bytes() >= image_size;

            let name = if valid_size {
                format!("<b>{}: {}</b>", device.label(), bytesize::to_string(device.size_in_bytes(), true))
            } else {
                format!("<b>{}: Device too small</b>", device.label())
            };

            let view_ready = self.view_ready.clone();
            let nselected = nselected.clone();

            let row = cascade! {
                gtk::CheckButton::new();
                ..set_sensitive(valid_size);
                ..add(&cascade! {
                    gtk::Label::new(name.as_str());
                    ..set_use_markup(true);
                });
                ..connect_toggled(move |button| {
                    if button.get_active() {
                        nselected.set(nselected.get() + 1);
                    } else {
                        nselected.set(nselected.get() - 1);
                    }

                    (*view_ready.borrow())(nselected.get() != 0);
                });
            };
            self.list.insert(&row, -1);
        }

        self.list.show_all();
    }

    pub fn reset(&self) {
        self.select_all.set_active(false);
        self.get_buttons().for_each(|c| c.set_active(false));
    }

    pub fn connect_view_ready<F: Fn(bool) + 'static>(&self, func: F) {
        *self.view_ready.borrow_mut() = Box::new(func);
    }
}
