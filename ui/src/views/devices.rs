// SPDX-License-Identifier: GPL-2.0-only

use std::{collections::HashSet, ffi::CStr, sync::Arc, time::Duration};

use cosmic::{
    Apply, Element,
    iced::{
        Alignment, Length, Subscription,
        futures::{FutureExt, SinkExt, Stream, StreamExt, channel::mpsc::Sender, stream::select},
    },
    theme::spacing,
    widget::{self, settings},
};
use tokio::sync::OnceCell;
use udisks2::{
    Client,
    zbus::{MatchRule, MessageStream, ObjectPath, message::Type},
};

use crate::{app::ActiveView, fl};

static CLIENT: OnceCell<Client> = OnceCell::const_new();

pub async fn client() -> udisks2::Result<&'static Client> {
    CLIENT.get_or_try_init(Client::new).await
}

#[derive(Debug, Clone)]
pub struct DiskDevice {
    pub id: String,
    pub label: String,
    pub size: u64,
    pub path: String,
}

#[derive(Debug, Default)]
pub struct DevicesView {
    available_devices: Box<[Arc<DiskDevice>]>,
    selected: HashSet<String>,
    image_size: u64,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refreshed(Result<Box<[Arc<DiskDevice>]>, String>),
    Toggle(String, bool),
    SelectAll(bool),
}

impl DevicesView {
    pub fn set_image_size(&mut self, size: u64) {
        self.image_size = size;
        // Drop selections that no longer fit the new image.
        let too_small: Vec<_> = self
            .available_devices
            .iter()
            .filter(|device| device.size < size)
            .map(|device| &device.id)
            .collect();
        for key in too_small {
            self.selected.remove(key);
        }
    }

    pub fn devices_selected(&self) -> bool {
        !self.selected.is_empty()
    }

    pub fn selected_devices(&self) -> Vec<Arc<DiskDevice>> {
        self.available_devices
            .iter()
            .filter(|device| self.selected.contains(&device.id))
            .cloned()
            .collect()
    }

    pub fn reset(&mut self) {
        self.selected.clear();
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(|| cosmic::iced::stream::channel(1, watch))
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Refreshed(result) => match result {
                Ok(devices) => {
                    self.error = None;

                    let changed = devices.len() != self.available_devices.len()
                        || devices.iter().zip(&self.available_devices).any(|(a, b)| a.id != b.id);

                    if changed {
                        let present: HashSet<_> = devices.iter().map(|d| &d.id).collect();
                        self.selected.retain(|k| present.contains(k));
                        self.available_devices = devices;
                    }
                }
                Err(error) => {
                    eprintln!("failed to refresh devices: {error}");
                    self.error = Some(error);
                }
            },

            Message::Toggle(key, on) => {
                if on {
                    self.selected.insert(key);
                } else {
                    self.selected.remove(&key);
                }
            }

            Message::SelectAll(on) => {
                self.selected = if on {
                    self.selectable().map(|device| device.id.clone()).collect()
                } else {
                    HashSet::new()
                };
            }
        }
    }

    fn selectable(&self) -> impl Iterator<Item = &Arc<DiskDevice>> {
        self.available_devices.iter().filter(|d| d.size >= self.image_size)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let space = spacing();

        let header = widget::column!(
            widget::text::title2(fl!("devices-view-title")),
            widget::text::body(fl!("devices-view-description"))
        )
        .width(Length::Fill)
        .spacing(space.space_xxs);

        let content =
            if self.available_devices.is_empty() { self.empty_state() } else { self.device_list() };

        widget::column::with_capacity(3)
            .push(header)
            .push(content)
            .push_maybe(self.error.as_deref().map(widget::text::caption))
            .align_x(Alignment::Center)
            .spacing(space.space_s)
            .apply(widget::scrollable)
            .into()
    }

    fn empty_state(&self) -> Element<'_, Message> {
        widget::column!(
            widget::icon::from_name("drive-removable-media-symbolic").size(48).symbolic(true),
            widget::text::heading(fl!("no-devices-found"))
        )
        .align_x(Alignment::Center)
        .spacing(spacing().space_xs)
        .width(Length::Fill)
        .padding(spacing().space_xl)
        .into()
    }

    fn device_list(&self) -> Element<'_, Message> {
        let selectable = self.selectable().count();
        let selected_selectable =
            self.selectable().filter(|d| self.selected.contains(&d.id)).count();
        let all_selected = selectable > 0 && selected_selectable == selectable;

        let select_all = settings::item::builder(fl!("select-all"))
            .checkbox_maybe(all_selected, (selectable > 0).then_some(Message::SelectAll));

        let mut section = settings::section().add(select_all);

        for device in &self.available_devices {
            let valid = device.size >= self.image_size;
            let size = bytesize::ByteSize(device.size).display().si().to_string();
            let description =
                if valid { size } else { format!("{size} · {}", fl!("device-too-small")) };

            let device_key = device.id.clone();
            let checked = self.selected.contains(&device_key);

            let row =
                settings::item::builder(&device.label).description(description).checkbox_maybe(
                    checked,
                    valid.then_some(move |on: bool| Message::Toggle(device_key.clone(), on)),
                );

            section = section.add(row);
        }

        section.into()
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, crate::app::Message>> {
        let can_press = *view == ActiveView::Devices && self.devices_selected();

        let back = widget::button::standard(fl!("back")).on_press(crate::app::Message::Back);

        let next = widget::button::suggested(fl!("next"))
            .on_press_maybe(can_press.then_some(crate::app::Message::Next));

        Some(
            widget::row!(back, widget::space::horizontal(), next)
                .spacing(spacing().space_xs)
                .padding(spacing().space_xs)
                .into(),
        )
    }
}

async fn fetch_devices() -> Result<Box<[Arc<DiskDevice>]>, udisks2::Error> {
    let managed = client().await?.object_manager().get_managed_objects().await?;
    let mut devices = managed
        .iter()
        .filter_map(|(path, interfaces)| {
            // don't iterate over partitions or loopbacks
            if interfaces.contains_key("org.freedesktop.UDisks2.Partition")
                || interfaces.contains_key("org.freedesktop.UDisks2.Loop")
            {
                return None;
            }
            let block = interfaces.get("org.freedesktop.UDisks2.Block")?;
            let size = block.get("Size").and_then(|s| u64::try_from(s).ok()).filter(|&s| s != 0)?;
            let drive_path = block.get("Drive").and_then(|d| <&ObjectPath>::try_from(d).ok())?;
            let drive = managed.get(drive_path)?.get("org.freedesktop.UDisks2.Drive")?;
            let get_str = |key| <&str>::try_from(drive.get(key)?).ok();
            if !matches!(get_str("ConnectionBus"), Some("usb" | "sdio")) {
                return None;
            }
            let id = get_str("Id")?;
            let vendor = get_str("Vendor").unwrap_or_default();
            let model = get_str("Model").unwrap_or_default();
            // TODO: upstream will eventually switch to rust strings
            let bytes =
                Vec::<u8>::try_from(block.get("PreferredDevice")?.try_clone().ok()?).ok()?;
            let device = CStr::from_bytes_until_nul(&bytes).ok()?.to_string_lossy();

            Some(Arc::new(DiskDevice {
                id: id.to_owned(),
                label: format!("{vendor} {model} ({device})").trim_start().to_owned(),
                size,
                path: path.as_str().to_owned(),
            }))
        })
        .collect::<Vec<_>>();

    devices.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(devices.into_boxed_slice())
}

async fn changes(client: &Client) -> udisks2::zbus::Result<impl Stream<Item = ()>> {
    let manager = client.object_manager();
    let added = manager.receive_interfaces_added().await?.map(|_| ());
    let removed = manager.receive_interfaces_removed().await?.map(|_| ());
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.DBus.Properties")
        .member("PropertiesChanged")
        .path_namespace("/org/freedesktop/UDisks2")
        .arg(0, "org.freedesktop.UDisks2.Block")
        .build()?;
    let properties =
        MessageStream::for_match_rule(rule, manager.inner().connection(), None).await?.map(|_| ());

    Ok(select(select(added, removed), properties))
}

async fn watch(mut output: Sender<Message>) {
    loop {
        let events = match client().await {
            Ok(client) => changes(client).await.map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };

        match events {
            Ok(mut events) => loop {
                let devices = fetch_devices().await.map_err(|e| e.to_string());
                if output.send(Message::Refreshed(devices)).await.is_err() {
                    return;
                }
                // wait for changes
                if events.next().await.is_none() {
                    break;
                }
                // ignore burst of signals on change
                tokio::time::sleep(Duration::from_millis(250)).await;
                while events.next().now_or_never().is_some() {}
            },
            Err(e) => {
                if output.send(Message::Refreshed(Err(e))).await.is_err() {
                    return;
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
