mod devices;
mod images;

use crate::app::events::{BackgroundEvent, UiEvent};
use crate::app::state::ActiveView;
use crate::app::App;
use crate::fl;
use crate::flash::{FlashRequest, FlashStatus, FlashTask};
use crate::misc;
use atomic::Atomic;
use crossbeam_channel::TryRecvError;
use gtk::{self, prelude::*};
use iso9660::ISO9660;
use std::fmt::Write;
use std::fs::File;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

impl App {
    pub fn connect_back(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();

        self.ui.header.connect_back(move || {
            let back = match state.active_view.get() {
                ActiveView::Images => {
                    gtk::main_quit();
                    return;
                }
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
        let mut flashing_devices: Vec<(gtk::ProgressBar, gtk::Label, gtk::Label)> = Vec::new();
        let flash_status = Arc::new(Atomic::new(FlashStatus::Inactive));
        let mut flash_handles = None;
        let mut tasks = None;
        let mut flash_start_time: Option<Instant> = None;

        glib::timeout_add_local(Duration::from_millis(16), move || {
            match state.ui_event_rx.try_recv() {
                Err(TryRecvError::Disconnected) => return Continue(false),
                Err(TryRecvError::Empty) => (),
                Ok(UiEvent::SetHash(hash)) => {
                    ui.content.image_view.set_hash(&match hash {
                        Ok(hash) => hash,
                        Err(why) => fl!("error", why = format!("{}", why)),
                    });
                    ui.content.image_view.set_hash_sensitive(true);

                    ui.content.image_view.chooser_container.set_visible_child_name("selected");
                }
                Ok(UiEvent::SetImageLabel(path)) => {
                    if let Ok(file) = File::open(&path) {
                        let image_size = file.metadata().ok().map_or(0, |m| m.len());

                        let warning = if is_windows_iso(&file) {
                            Some(fl!("win-isos-not-supported"))
                        } else {
                            None
                        };

                        ui.content.image_view.set_image(&path, image_size, warning.as_deref());
                        ui.content.image_view.set_hash_sensitive(true);
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
                        FlashStatus::Active => {
                            flash_status.store(FlashStatus::Killing, Ordering::SeqCst)
                        }
                        FlashStatus::Inactive | FlashStatus::Killing => (),
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
                }
                ActiveView::Flashing => match state.image.borrow_mut().take() {
                    // When the flashing view is active, and an image has not started flashing.
                    Some(image) => {
                        let summary_grid = &ui.content.flash_view.progress_list;
                        summary_grid.foreach(|w| summary_grid.remove(w));
                        let mut destinations = Vec::new();

                        // Reset overall progress
                        ui.content.flash_view.overall_bar.set_fraction(0.0);
                        ui.content.flash_view.overall_label.set_label(
                            &fl!("overall-progress", percent = 0),
                        );
                        ui.content.flash_view.elapsed_label.set_label(
                            &fl!("elapsed-time", time = "0:00"),
                        );
                        flash_start_time = Some(Instant::now());

                        let selected_devices = state.selected_devices.borrow_mut();
                        for (id, device) in selected_devices.iter().enumerate() {
                            let id = id as i32;

                            let pbar = cascade! {
                                gtk::ProgressBar::new();
                                ..set_hexpand(true);
                            };

                            let label = cascade! {
                                gtk::Label::new(Some(&misc::device_label(device)));
                                ..set_justify(gtk::Justification::Right);
                                ..style_context().add_class("bold");
                            };

                            let percent_label = cascade! {
                                gtk::Label::new(Some("0%"));
                                ..set_halign(gtk::Align::End);
                                ..set_width_chars(5);
                            };

                            let bar_label = cascade! {
                                gtk::Label::new(None);
                                ..set_halign(gtk::Align::Center);
                                ..style_context().add_class("caption");
                            };

                            let bar_container = cascade! {
                                gtk::Box::new(gtk::Orientation::Vertical, 0);
                                ..add(&pbar);
                                ..add(&bar_label);
                            };

                            let bar_row = cascade! {
                                gtk::Box::new(gtk::Orientation::Horizontal, 8);
                                ..pack_start(&bar_container, true, true, 0);
                                ..pack_start(&percent_label, false, false, 0);
                            };

                            summary_grid.attach(&label, 0, id, 1, 1);
                            summary_grid.attach(&bar_row, 1, id, 1, 1);

                            flashing_devices.push((pbar, bar_label, percent_label));
                            destinations.push(device.clone());
                        }

                        summary_grid.show_all();
                        let ndestinations = destinations.len();
                        let progress = Arc::new(
                            (0..ndestinations).map(|_| Atomic::new(0u64)).collect::<Vec<_>>(),
                        );
                        let finished = Arc::new(
                            (0..ndestinations).map(|_| Atomic::new(false)).collect::<Vec<_>>(),
                        );

                        let _ =
                            state.back_event_tx.send(BackgroundEvent::Flash(FlashRequest::new(
                                image,
                                destinations,
                                flash_status.clone(),
                                progress.clone(),
                                finished.clone(),
                            )));

                        tasks = Some(FlashTask {
                            previous: Arc::new(Mutex::new(vec![[0; 7]; ndestinations])),
                            progress,
                            finished,
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
                            let mut total_progress = 0.0f64;
                            let ndevices = flashing_devices.len();

                            for (id, (pbar, label, percent_label)) in flashing_devices.iter().enumerate() {
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
                                total_progress += value;

                                let pct = (value * 100.0) as u32;
                                percent_label.set_label(&format!("{}%", pct));

                                if task_is_finished {
                                    label.set_label(&fl!("task-finished"));
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
                                    label.set_label(&format!(
                                        "{}/s",
                                        bytesize::to_string(per_second, true)
                                    ));
                                }
                            }

                            // Update overall progress bar
                            let overall = if ndevices > 0 { total_progress / ndevices as f64 } else { 0.0 };
                            ui.content.flash_view.overall_bar.set_fraction(overall);
                            ui.content.flash_view.overall_label.set_label(
                                &fl!("overall-progress", percent = { (overall * 100.0) as i64 }),
                            );

                            // Update elapsed time
                            if let Some(start) = flash_start_time {
                                let elapsed = now.duration_since(start);
                                let secs = elapsed.as_secs();
                                let mins = secs / 60;
                                let secs = secs % 60;
                                ui.content.flash_view.elapsed_label.set_label(
                                    &fl!("elapsed-time", time = format!("{}:{:02}", mins, secs)),
                                );
                            }

                            drop(previous);

                            if all_tasks_finished {
                                eprintln!("all tasks finished");

                                let taken_handles = match ui.errorck_option(
                                    &state,
                                    flash_handles.take(),
                                    "Taking flash handles failed",
                                ) {
                                    Ok(results) => {
                                        results.join().map_err(|why| format!("{:?}", why))
                                    }
                                    Err(()) => return Continue(true),
                                };

                                let handle = match ui.errorck(
                                    &state,
                                    taken_handles,
                                    "Failed to join flash thread",
                                ) {
                                    Ok(results) => results,
                                    Err(()) => return Continue(true),
                                };

                                let (result, results) = match ui.errorck(
                                    &state,
                                    handle,
                                    "Errored starting flashing process",
                                ) {
                                    Ok(result) => result,
                                    Err(()) => return Continue(true),
                                };

                                let mut errors = Vec::new();
                                let mut selected_devices = state.selected_devices.borrow_mut();
                                let ntasks = selected_devices.len();

                                for (device, result) in
                                    selected_devices.drain(..).zip(results.into_iter())
                                {
                                    if let Err(why) = result {
                                        errors.push((device, why));
                                    }
                                }

                                ui.switch_to(&state, ActiveView::Summary);
                                let list = &ui.content.summary_view.list;
                                let description = &ui.content.summary_view.view.description;
                                let result_container = &ui.content.summary_view.result_container;

                                // Clear previous results
                                result_container.foreach(|w| result_container.remove(w));
                                result_container.style_context().remove_class("success-accent");
                                result_container.style_context().remove_class("error-accent");

                                if result.is_ok() && errors.is_empty() {
                                    let desc = fl!("successful-flash", total = ntasks);
                                    description.set_text(&desc);
                                    result_container.style_context().add_class("success-accent");

                                    let success_label = cascade! {
                                        gtk::Label::new(Some(&desc));
                                        ..style_context().add_class("h2");
                                        ..set_halign(gtk::Align::Start);
                                    };
                                    result_container.add(&success_label);
                                    result_container.show_all();
                                    list.hide();
                                } else {
                                    ui.content
                                        .summary_view
                                        .view
                                        .topic
                                        .set_text(&fl!("flashing-completed-with-errors"));

                                    let mut desc = fl!(
                                        "partial-flash",
                                        number = { ntasks - errors.len() },
                                        total = ntasks
                                    );

                                    if let Err(why) = result {
                                        let _ = write!(desc, ": <b>{}</b>", why);
                                    }

                                    description.set_markup(&desc);

                                    for (device, why) in errors {
                                        let device_label =
                                            gtk::Label::new(Some(&misc::device_label(&device)));
                                        let why_label =
                                            gtk::Label::new(Some(format!("{}", why).as_str()));
                                        why_label.style_context().add_class("bold");

                                        let container = cascade! {
                                            gtk::Box::new(gtk::Orientation::Horizontal, 6);
                                            ..pack_start(&device_label, false, false, 0);
                                            ..pack_start(&why_label, true, true, 0);
                                            ..style_context().add_class("error-accent");
                                        };

                                        let row = cascade! {
                                            gtk::ListBoxRow::new();
                                            ..set_selectable(false);
                                            ..add(&container);
                                        };

                                        list.add(&row);
                                    }

                                    list.show_all();
                                }
                            }
                        }
                    }
                },
                _ => (),
            }

            Continue(true)
        });
    }
}

fn is_windows_iso(file: &File) -> bool {
    if let Ok(fs) = ISO9660::new(file) {
        return fs.publisher_identifier() == "MICROSOFT CORPORATION";
    }
    false
}
