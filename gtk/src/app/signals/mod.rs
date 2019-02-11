mod devices;
mod images;

use app::App;
use app::events::{BackgroundEvent, UiEvent, PrivilegedEvent};
use app::state::ActiveView;
use atomic::Atomic;
use crossbeam_channel::TryRecvError;
use flash::{FlashRequest, FlashTask, FlashStatus};
use gtk::{self, prelude::*};
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::time::Instant;

impl App {
    pub fn connect_back(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();

        self.ui.header.connect_back(move || {
            let back = match state.active_view.get() {
                ActiveView::Images => {
                    gtk::main_quit();
                    return;
                },
                _ => ActiveView::Images,
            };

            let _ = state.ui_event_tx.send(UiEvent::Reset);
            ui.content.devices_view.reset();

            ui.switch_to(&state, back);
        });
    }

    pub fn connect_next(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();

        self.ui.header.connect_next(move || {
            let next = match state.active_view.get() {
                ActiveView::Images => ActiveView::Devices,
                ActiveView::Devices => ActiveView::Flashing,
                _ => {
                    gtk::main_quit();
                    return;
                }
            };

            ui.switch_to(&state, next);
        });
    }

    pub fn connect_ui_events(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();

        let mut last_device_refresh = Instant::now();
        let mut flashing_devices: Vec<(gtk::ProgressBar, gtk::Label)> = Vec::new();
        let flash_status = Arc::new(Atomic::new(FlashStatus::Inactive));
        let mut flash_handles = None;
        let mut tasks = None;

        gtk::timeout_add(16, move || {
            match state.ui_event_rx.try_recv() {
                Err(TryRecvError::Disconnected) => return gtk::Continue(false),
                Err(TryRecvError::Empty) => (),
                Ok(UiEvent::SetHash(hash)) => {
                    ui.content.image_view.set_hash(&match hash {
                        Ok(hash) => hash,
                        Err(why) => format!("error: {}", why)
                    });

                    ui.content.image_view.chooser_container
                        .set_visible_child_name("chooser");
                },
                Ok(UiEvent::SetImageLabel(path)) => {
                    if let Ok(file) = File::open(&path) {
                        let image_size = file.metadata().ok().map_or(0, |m| m.len());

                        ui.content.image_view.set_image_path(&path);
                        ui.content.image_view.hash.set_sensitive(true);
                        ui.content.image_view.hash_label.set_sensitive(true);
                        ui.header.next.set_sensitive(true);

                        state.image_size.store(image_size, Ordering::SeqCst);
                        *state.image_path.borrow_mut() = path;
                    }
                }
                Ok(UiEvent::RefreshDevices(devices)) => {
                    let size = state.image_size.load(Ordering::SeqCst);
                    ui.content.devices_view.refresh(&devices, size);
                    *state.available_devices.borrow_mut() = devices;
                }
                Ok(UiEvent::Flash(handle)) => flash_handles = Some(handle),
                Ok(UiEvent::Reset) => {
                    match flash_status.load(Ordering::SeqCst) {
                        FlashStatus::Active => flash_status.store(FlashStatus::Killing, Ordering::SeqCst),
                        FlashStatus::Inactive
                        | FlashStatus::Killing => (),
                    }

                    flash_handles = None;
                    tasks = None;
                    flashing_devices.clear();
                }
            }

            match state.active_view.get() {
                ActiveView::Devices => {
                    let now = Instant::now();

                    // Only attempt to refresh the devices if the last refresh was >= 3 seconds ago.
                    if now.duration_since(last_device_refresh).as_secs() >= 3 {
                        last_device_refresh = now;
                        let _ = state.back_event_tx.send(BackgroundEvent::RefreshDevices);
                    }
                },
                ActiveView::Flashing => match state.image.borrow_mut().take() {
                    // When the flashing view is active, and an image has not started flashing.
                    Some(image) => {
                        let summary_grid = &ui.content.flash_view.progress_list;
                        summary_grid.get_children().iter().for_each(|c| c.destroy());
                        let mut destinations = Vec::new();

                        let mut selected_devices = state.selected_devices.borrow_mut();
                        for (id, device) in selected_devices.iter().enumerate() {
                            let id = id as i32;

                            let pbar = cascade! {
                                gtk::ProgressBar::new();
                                ..set_hexpand(true);
                            };

                            let label = cascade! {
                                gtk::Label::new(device.label().as_str());
                                ..set_justify(gtk::Justification::Right);
                                ..get_style_context().map(|c| c.add_class("bold"));
                            };

                            let bar_label = cascade! {
                                gtk::Label::new(None);
                                ..set_halign(gtk::Align::Center);
                            };

                            let bar_container = cascade! {
                                gtk::Box::new(gtk::Orientation::Vertical, 0);
                                ..add(&pbar);
                                ..add(&bar_label);
                            };

                            summary_grid.attach(&label, 0, id, 1, 1);
                            summary_grid.attach(&bar_container, 1, id, 1, 1);

                            flashing_devices.push((pbar, bar_label));
                            destinations.push(device.path.clone());
                        }

                        summary_grid.show_all();
                        let ndestinations = destinations.len();
                        let progress = Arc::new((0..ndestinations).map(|_| Atomic::new(0u64)).collect::<Vec<_>>());
                        let finished = Arc::new((0..ndestinations).map(|_| Atomic::new(false)).collect::<Vec<_>>());

                        let _ = state.priv_event_tx.send(PrivilegedEvent::Flash(
                            FlashRequest::new(
                                image,
                                destinations,
                                flash_status.clone(),
                                progress.clone(),
                                finished.clone()
                            )
                        ));

                        tasks = Some(FlashTask {
                            previous: Arc::new(Mutex::new(vec![[0; 7]; ndestinations])),
                            progress,
                            finished
                        });
                    }
                    // When the flashing view is active, and thus an image is flashing.
                    None => {
                        let now = Instant::now();

                        // Only attempt to refresh the devices if the last refresh was >= 500ms ago.
                        let time_since = now.duration_since(last_device_refresh);
                        if time_since.as_secs() > 1 || time_since.subsec_millis() >= 500 {
                            last_device_refresh = now;

                            let mut all_tasks_finished = true;
                            let length = state.image_size.load(Ordering::SeqCst);
                            let tasks = tasks.as_mut().expect("no flash task");
                            let mut previous = tasks.previous.lock().expect("mutex lock");

                            for (id, &(ref pbar, ref label)) in flashing_devices.iter().enumerate() {
                                let prev_values = &mut previous[id];
                                let progress = &tasks.progress[id];
                                let finished = &tasks.finished[id];

                                let raw_value = progress.load(Ordering::SeqCst);
                                let task_is_finished = finished.load(Ordering::SeqCst);
                                let value = if task_is_finished {
                                    1.0f64
                                } else {
                                    all_tasks_finished = false;
                                    raw_value as f64 / length as f64
                                };

                                pbar.set_fraction(value);

                                if task_is_finished {
                                    label.set_label("Complete");
                                } else {
                                    prev_values[1] = prev_values[2];
                                    prev_values[2] = prev_values[3];
                                    prev_values[3] = prev_values[4];
                                    prev_values[4] = prev_values[5];
                                    prev_values[5] = prev_values[6];
                                    prev_values[6] = raw_value - prev_values[0];
                                    prev_values[0] = raw_value;

                                    let sum: u64 = prev_values.iter().skip(1).sum();
                                    let per_second = sum / 3;
                                    label.set_label(&if per_second > (1024 * 1024) {
                                        format!("{} MiB/s", per_second / (1024 * 1024))
                                    } else {
                                        format!("{} KiB/s", per_second / 1024)
                                    });
                                }
                            }

                            drop(previous);

                            if all_tasks_finished {
                                eprintln!("all tasks finished");
                                let results = flash_handles
                                    .take()
                                    .expect("flash handles did not exist")
                                    .join()
                                    .expect("failed to join flash thread")
                                    .expect("flashing process failed");

                                let mut errors = Vec::new();
                                let mut selected_devices = state.selected_devices.borrow_mut();
                                let ntasks = selected_devices.len();

                                for (device, result) in selected_devices.drain(..).zip(results.into_iter()) {
                                    if let Err(why) = result {
                                        errors.push((device, why));
                                    }
                                }

                                ui.switch_to(&state, ActiveView::Summary);
                                let list = &ui.content.summary_view.list;
                                let description = &ui.content.summary_view.view.description;

                                if errors.is_empty() {
                                    let desc = format!("{} devices successfully flashed", ntasks);
                                    description.set_text(&desc);
                                    list.hide();
                                } else {
                                    let desc = format!(
                                        "{} of {} devices successfully flashed",
                                        ntasks - errors.len(),
                                        ntasks
                                    );

                                    description.set_text(&desc);
                                    list.show();

                                    for (device, why) in errors {
                                        let device = gtk::Label::new(device.label().as_str());
                                        let why = gtk::Label::new(format!("{}", why).as_str());

                                        let container = cascade! {
                                            gtk::Box::new(gtk::Orientation::Horizontal, 0);
                                            ..pack_start(&device, false, false, 0);
                                            ..pack_start(&why, true, true, 0);
                                        };

                                        list.insert(&container, -1);
                                    }
                                }
                            }
                        }
                    }
                }
                _ => ()
            }

            gtk::Continue(true)
        });
    }
}
