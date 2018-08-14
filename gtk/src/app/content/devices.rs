use super::View;
use gtk::prelude::*;
use gtk;
use popsicle;
use block::BlockDevice;
use std::path::Path;
use app::state::State;
use std::sync::Arc;

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
        self.select_all.connect_clicked(move |all| result(
            state.devices.lock()
                .map_err(|why| format!("devices mutex lock failed: {}", why))
                .map(|ref devices| {
                    devices.iter()
                        .for_each(|&(_, ref device)| device.set_active(
                            all.get_active() && device.is_sensitive()
                        ))
                }
            )
        ));
    }

    fn devices_differ(devices: &[String], device_list: &[(String, gtk::CheckButton)]) -> bool {
        devices.iter()
            .zip(device_list.iter())
            .any(|(ref x, &(ref y, _))| x.as_str() != y.as_str())
    }

    pub fn refresh(
        &self,
        device_list: &mut Vec<(String, gtk::CheckButton)>,
        devices: &[String],
        image_sectors: u64,
    ) -> Result<(), String> {
        device_list.clear();
        self.clear();

        let mut all_is_sensitive = false;
        for device in devices {
            // Attempt to get the canonical path of the device.
            // Display the error view if this fails.
            let name = Path::new(&device).canonicalize()
                .map_err(|why| format!("unable to get canonical path of '{}': {}", device, why))?;

            let button = if let Some(block) = BlockDevice::new(&name) {
                let too_small = block.sectors() < image_sectors;

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
                    all_is_sensitive = true;
                }
                button
            } else {
                gtk::CheckButton::new_with_label(&name.to_string_lossy())
            };

            self.list.insert(&button, -1);
            device_list.push((device.clone(), button));
        }

        self.list.show_all();
        self.select_all.set_sensitive(all_is_sensitive);
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
