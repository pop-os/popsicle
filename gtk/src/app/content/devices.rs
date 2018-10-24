use super::View;
use gtk::prelude::*;
use gtk;
use popsicle;
use block::BlockDevice;
use std::path::Path;
use app::state::State;
use std::sync::Arc;
use std::thread;
use std::path::PathBuf;
use std::cell::Cell;
use crossbeam_channel::{unbounded, Receiver, TryRecvError};

pub struct DevicesView {
    pub view: View,
    pub list: DeviceList,
}

impl DevicesView {
    pub fn new() -> DevicesView {
        let select_all = gtk::CheckButton::new_with_label("Select all");
        let list = gtk::ListBox::new();
        list.get_style_context().map(|c| c.add_class("devices"));

        let list_box = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 0);
            ..pack_start(&select_all, false, false, 0);
            ..pack_start(&gtk::Separator::new(gtk::Orientation::Horizontal), false, false, 0);
            ..pack_start(&list, true, true, 0);
        };

        let select_scroller = cascade! {
            gtk::ScrolledWindow::new(None, None);
            ..add(&list_box);
        };

        let view = View::new(
            "drive-removable-media-usb",
            "Select Drives",
            "Flashing will erase all data on the selected drives.",
            |right_panel| right_panel.pack_start(&select_scroller, true, true, 0),
        );

        DevicesView {
            view,
            list: DeviceList {
                list,
                select_all,
            }
        }
    }
}

pub struct DeviceList {
    pub list:       gtk::ListBox,
    pub select_all: gtk::CheckButton,
}

impl DeviceList {
    pub fn clear(&self) {
        self.list.get_children()
            .into_iter()
            .for_each(|widget| widget.destroy());
    }

    pub fn connect_select_all<F>(&self, state: Arc<State>, result: F )
        where F: 'static + Fn(Result<(), String>)
    {
        self.select_all.connect_clicked(move |all| result({
            let devices = state.devices.lock();
            devices.iter()
                .for_each(|&(_, ref device)| device.set_active(
                    all.get_active() && device.is_sensitive()
                ));
            Ok(())
        }));
    }

    fn devices_differ(devices: &[String], device_list: &[(String, gtk::CheckButton)]) -> bool {
        devices.iter()
            .zip(device_list.iter())
            .any(|(ref x, &(ref y, _))| x.as_str() != y.as_str())
    }

    fn create_device_button(
        name: PathBuf,
        block: Option<BlockDevice>,
        image_sectors: u64,
        select_all: &gtk::CheckButton
    ) -> gtk::CheckButton {
        if let Some(block) = block {
            let too_small = block.sectors < image_sectors;

            let button = gtk::CheckButton::new_with_label(&{
                if too_small {
                    [ &block.label(), " (", &name.to_string_lossy(), "): Device is too small" ].concat()
                } else {
                    [ &block.label(), " (", &name.to_string_lossy(), ")" ].concat()
                }
            });

            if too_small {
                button.set_tooltip_text("Device is too small");
                button.set_has_tooltip(true);
                button.set_sensitive(false);
            } else {
                select_all.set_sensitive(true);
            }
            button
        } else {
            gtk::CheckButton::new_with_label(&name.to_string_lossy())
        }
    }

    fn scan_block_devices(devices: Arc<Vec<String>>) -> Receiver<(PathBuf, Option<BlockDevice>)> {
        let (tx, rx) = unbounded();
        thread::spawn(move || {
            for device in &*devices {
                match Path::new(device).canonicalize() {
                    Ok(name) => {
                        let _ = tx.send((name.clone(), BlockDevice::new(&name)));
                    },
                    Err(why) => {
                        eprintln!("unable to get canonical path of '{}': {}", device, why);
                    }
                }
            }
        });
        rx
    }

    pub fn refresh(
        &self,
        state: Arc<State>,
        devices: Vec<String>,
        image_sectors: u64,
    ) -> Result<(), String> {
        state.refreshing_devices.set(true);
        self.select_all.set_sensitive(false);
        (&mut state.devices.lock()).clear();
        self.clear();

        let devices = Arc::new(devices);
        let rx = Self::scan_block_devices(devices.clone());

        // Asynchronously add new devices as they're discovered when refreshing.
        eprintln!("refreshing device selection");
        let select_all = self.select_all.clone();
        let list = self.list.clone();
        let nth_device = Cell::new(0);
        gtk::timeout_add(16, move || {
            loop {
                match rx.try_recv() {
                    Ok((name, block)) => {
                        let button = Self::create_device_button(name, block, image_sectors, &select_all);

                        list.insert(&button, -1);
                        list.show_all();

                        let device_list = &mut state.devices.lock();
                        let nth = nth_device.get();
                        device_list.push((devices[nth].clone(), button));
                        nth_device.set(nth + 1);
                    }
                    Err(TryRecvError::Empty) => return gtk::Continue(true),
                    Err(TryRecvError::Disconnected) => {
                        eprintln!("finished refreshing device selection");
                        state.refreshing_devices.set(false);
                        return gtk::Continue(false);
                    },
                }
            }
        });

        Ok(())
    }

    pub fn requires_refresh(device_list: &[(String, gtk::CheckButton)]) -> Option<Vec<String>> {
        let mut devices = vec![];
        if let Err(why) = popsicle::get_disk_args(&mut devices) {
            eprintln!("popsicle: unable to get devices: {}", why);
            return None;
        }

        if devices.len() != device_list.len() || Self::devices_differ(&devices, &device_list) {
            Some(devices)
        } else {
            None
        }
    }
}
