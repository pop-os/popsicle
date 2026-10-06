use std::{collections::HashSet, sync::Arc, time::Duration};

use cosmic::{
    Apply, Element, Task,
    iced::{Alignment, Length, Subscription},
    theme::spacing,
    widget::{self, settings},
};
use dbus_udisks2::{DiskDevice, Disks, UDisks2};

use crate::{app::ActiveView, fl};

/// Stable identity for a drive across refreshes. If `drive.id` isn't a `String`
/// in your dbus-udisks2 version, this is the only place to change.
type DeviceKey = String;

fn key(device: &DiskDevice) -> DeviceKey {
    device.drive.id.clone()
}

#[derive(Debug, Default)]
pub struct DevicesView {
    available_devices: Box<[Arc<DiskDevice>]>,
    selected: HashSet<DeviceKey>,
    image_size: u64,
    refreshing: bool,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    Refreshed(Result<Box<[Arc<DiskDevice>]>, String>),
    Toggle(DeviceKey, bool),
    SelectAll(bool),
}

impl DevicesView {
    /// Call when the image changes or when navigating to this view.
    pub fn set_image_size(&mut self, size: u64) {
        self.image_size = size;
        // Drop selections that no longer fit the new image.
        let size = self.image_size;
        let too_small: Vec<_> = self
            .available_devices
            .iter()
            .filter(|device| device.parent.size < size)
            .map(|device| key(device))
            .collect();
        for key in too_small {
            self.selected.remove(&key);
        }
    }

    pub fn devices_selected(&self) -> bool {
        !self.selected.is_empty()
    }

    /// For the flashing view / flash request.
    pub fn selected_devices(&self) -> Vec<Arc<DiskDevice>> {
        self.available_devices
            .iter()
            .filter(|device| self.selected.contains(&key(device)))
            .cloned()
            .collect()
    }

    pub fn reset(&mut self) {
        self.selected.clear();
    }

    /// Only include this in the app's subscription while the Devices view is active.
    pub fn subscription(&self) -> Subscription<Message> {
        cosmic::iced::time::every(Duration::from_secs(1)).map(|_| Message::Refresh)
    }

    /// Kick off a refresh right away (call when entering the view).
    pub fn refresh(&mut self) -> Task<cosmic::Action<Message>> {
        if self.refreshing {
            return Task::none();
        }
        self.refreshing = true;

        cosmic::task::future(async {
            let result = tokio::task::spawn_blocking(fetch_devices)
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);

            Message::Refreshed(result)
        })
        .map(cosmic::Action::App)
    }

    pub fn update(&mut self, message: Message) -> Option<Task<cosmic::Action<Message>>> {
        match message {
            Message::Refresh => return Some(self.refresh()),

            Message::Refreshed(result) => {
                self.refreshing = false;

                match result {
                    Ok(devices) => {
                        self.error = None;

                        let changed = devices.len() != self.available_devices.len()
                            || devices
                                .iter()
                                .zip(&self.available_devices)
                                .any(|(a, b)| key(a) != key(b));

                        if changed {
                            // Forget selections for devices that were unplugged.
                            let present: HashSet<_> = devices.iter().map(|d| key(d)).collect();
                            self.selected.retain(|k| present.contains(k));
                            self.available_devices = devices;
                        }
                    }
                    Err(error) => {
                        eprintln!("failed to refresh devices: {error}");
                        self.error = Some(error);
                    }
                }
            }

            Message::Toggle(key, on) => {
                if on {
                    self.selected.insert(key);
                } else {
                    self.selected.remove(&key);
                }
            }

            Message::SelectAll(on) => {
                self.selected = if on {
                    self.selectable().map(|device| key(device)).collect()
                } else {
                    HashSet::new()
                };
            }
        }

        None
    }

    fn selectable(&self) -> impl Iterator<Item = &Arc<DiskDevice>> {
        self.available_devices.iter().filter(|d| d.parent.size >= self.image_size)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let space = spacing();

        let header = widget::column([])
            .push(widget::text::title2(fl!("devices-view-title")))
            .push(widget::text::body(fl!("devices-view-description")))
            .width(Length::Fill)
            .spacing(space.space_xxs);

        let content: Element<'_, Message> =
            if self.available_devices.is_empty() { self.empty_state() } else { self.device_list() };

        widget::column([])
            .push(header)
            .push(content)
            .push_maybe(self.error.as_deref().map(widget::text::caption))
            .align_x(Alignment::Center)
            .spacing(space.space_s)
            .apply(widget::scrollable)
            .into()
    }

    fn empty_state(&self) -> Element<'_, Message> {
        widget::column([])
            .push(widget::icon::from_name("drive-removable-media-symbolic").size(48).symbolic(true))
            .push(widget::text::heading(fl!("no-devices-found")))
            .align_x(Alignment::Center)
            .spacing(spacing().space_xs)
            .width(Length::Fill)
            .padding(spacing().space_xl)
            .into()
    }

    fn device_list(&self) -> Element<'_, Message> {
        let selectable = self.selectable().count();
        let selected_selectable =
            self.selectable().filter(|d| self.selected.contains(&key(d))).count();
        let all_selected = selectable > 0 && selected_selectable == selectable;

        let select_all = settings::item::builder(fl!("select-all"))
            .checkbox_maybe(all_selected, (selectable > 0).then_some(Message::SelectAll));

        let mut section = settings::section().add(select_all);

        for device in &self.available_devices {
            let valid = device.parent.size >= self.image_size;
            let size = bytesize::to_string(device.parent.size, true);
            let description =
                if valid { size } else { format!("{size} · {}", fl!("device-too-small")) };

            let device_key = key(device);
            let checked = self.selected.contains(&device_key);

            let row = settings::item::builder(device_label(device))
                .description(description)
                .checkbox_maybe(
                    checked,
                    valid.then(|| move |on: bool| Message::Toggle(device_key.clone(), on)),
                );

            section = section.add(row);
        }

        section.into()
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, crate::app::Message>> {
        let can_press = *view == ActiveView::Devices && self.devices_selected();

        let next = widget::button::suggested(fl!("next"))
            .on_press_maybe(can_press.then_some(crate::app::Message::Next))
            .into();

        let spacer = widget::space::horizontal().into();

        Some(
            widget::row(vec![spacer, next])
                .spacing(spacing().space_xs)
                .padding(spacing().space_xs)
                .into(),
        )
    }
}

pub fn device_label(device: &DiskDevice) -> String {
    if device.drive.vendor.is_empty() {
        format!("{} ({})", device.drive.model, device.parent.preferred_device.display())
    } else {
        format!(
            "{} {} ({})",
            device.drive.vendor,
            device.drive.model,
            device.parent.preferred_device.display()
        )
    }
}

/// Blocking: talks to udisks2 over D-Bus. Same filters as popsicle's `refresh_devices`.
fn fetch_devices() -> Result<Box<[Arc<DiskDevice>]>, String> {
    let udisks = UDisks2::new().map_err(|error| error.to_string())?;

    let mut devices = Disks::new(&udisks)
        .devices
        .into_iter()
        .filter(|d| d.drive.connection_bus == "usb" || d.drive.connection_bus == "sdio")
        .filter(|d| d.parent.size != 0)
        .map(Arc::new)
        .collect::<Vec<_>>();

    devices.sort_by_key(|d| d.drive.id.clone());
    Ok(devices.into_boxed_slice())
}
