use std::{cell::RefCell, sync::Arc};

use cosmic::{Element, widget};
use dbus_udisks2::DiskDevice;

#[derive(Debug, Default)]
pub struct DevicesView {
    pub available_devices: RefCell<Box<[Arc<DiskDevice>]>>,
    pub selected_devices: RefCell<Vec<Arc<DiskDevice>>>,
}

#[derive(Debug, Clone)]
pub enum Message {}

impl DevicesView {
    pub fn view<'a>(&self) -> impl Into<Element<'a, Message>> {
        widget::text("Devices...")
    }

    pub fn devices_selected(&self) -> bool {
        !self.selected_devices.borrow().is_empty()
    }
}
