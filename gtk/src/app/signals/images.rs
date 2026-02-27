use crate::app::events::{BackgroundEvent, UiEvent};
use crate::app::state::State;
use crate::app::widgets::OpenDialog;
use crate::app::{App, GtkUi};
use gtk::prelude::*;
use std::path::{Path, PathBuf};

fn open_image_dialog(state: &State, ui: &GtkUi) {
    if let Some(path) = OpenDialog::new(None).run() {
        let _ = state.ui_event_tx.send(UiEvent::SetImageLabel(path));
        set_hash_widget(state, ui);
    }
}

impl App {
    pub fn connect_image_chooser(&self) {
        // Drop zone click opens file chooser via EventBox
        let state = self.state.clone();
        let ui = self.ui.clone();
        let event_box = self.ui.content.image_view.drop_zone_event_box.clone();

        let state2 = state.clone();
        let ui2 = ui.clone();
        event_box.connect_button_press_event(move |_, _| {
            open_image_dialog(&state2, &ui2);
            gtk::Inhibit(false)
        });

        // Change button also opens file chooser
        let state3 = state.clone();
        let ui3 = ui.clone();
        self.ui.content.image_view.change_button.connect_clicked(move |_| {
            open_image_dialog(&state3, &ui3);
        });

        // Keep legacy chooser button wired up (hidden but functional)
        self.ui.content.image_view.chooser.connect_clicked(move |_| {
            open_image_dialog(&state, &ui);
        });
    }

    pub fn connect_hash(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();
        self.ui.content.image_view.check.connect_clicked(move |_| {
            set_hash_widget(&state, &ui);
        });
    }

    pub fn connect_image_drag_and_drop(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();
        let image_view = ui.content.image_view.view.container.clone();
        let drop_zone = ui.content.image_view.drop_zone.clone();

        // Add drag-enter/leave visual feedback
        let dz_enter = drop_zone.clone();
        image_view.connect_drag_motion(move |_view, ctx, _x, _y, time| {
            dz_enter.style_context().add_class("drop-zone-active");
            ctx.drag_status(gdk::DragAction::COPY, time);
            true
        });

        let dz_leave = drop_zone.clone();
        image_view.connect_drag_leave(move |_view, _ctx, _time| {
            dz_leave.style_context().remove_class("drop-zone-active");
        });

        // Set up drag dest and handle drops
        image_view.drag_dest_set(gtk::DestDefaults::empty(), &[], gdk::DragAction::empty());

        let dz_drop = drop_zone.clone();
        image_view.connect_drag_drop(move |view, ctx, _x, _y, time| {
            dz_drop.style_context().remove_class("drop-zone-active");
            ctx.list_targets().last().map_or(false, |target| {
                view.drag_get_data(ctx, target, time);
                true
            })
        });

        image_view.connect_drag_data_received(move |_view, _ctx, _x, _y, data, _info, _time| {
            if let Some(uri) = data.text() {
                if uri.starts_with("file://") {
                    let path = Path::new(&uri[7..uri.len() - 1]);
                    if path.extension().map_or(false, |ext| ext == "iso" || ext == "img")
                        && path.exists()
                    {
                        let _ = state.ui_event_tx.send(UiEvent::SetImageLabel(path.to_path_buf()));
                        set_hash_widget(&state, &ui);
                    }
                }
            }
        });
    }
}

fn set_hash_widget(state: &State, ui: &GtkUi) {
    let hash = &ui.content.image_view.hash;

    let path = state.image_path.borrow();
    let kind = match hash.active() {
        Some(1) => "SHA512",
        Some(2) => "SHA256",
        Some(3) => "SHA1",
        Some(4) => "MD5",
        Some(5) => "BLAKE2b",
        _ => return,
    };

    ui.content.image_view.chooser_container.set_visible_child_name("checksum");
    ui.content.image_view.set_hash_sensitive(false);

    let _ = state.back_event_tx.send(BackgroundEvent::GenerateHash(PathBuf::from(&*path), kind));
}
