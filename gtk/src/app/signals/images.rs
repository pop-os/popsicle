use app::{App, GtkUi};
use app::events::{BackgroundEvent, UiEvent};
use app::state::State;
use app::widgets::OpenDialog;
use gtk::prelude::*;
use std::path::PathBuf;

impl App {
    pub fn connect_image_chooser(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();
        self.ui.content.image_view.chooser.connect_clicked(move |_| {
            if let Some(path) = OpenDialog::new(None).run() {
                let _ = state.ui_event_tx.send(UiEvent::SetImageLabel(path));
                set_hash_widget(&state, &ui);
            }
        });
    }

    pub fn connect_hash(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();
        self.ui.content.image_view.hash.connect_changed(move |_| {
            set_hash_widget(&state, &ui);
        });
    }
}

fn set_hash_widget(state: &State, ui: &GtkUi) {
    let hash = &ui.content.image_view.hash;

    let path = state.image_path.borrow();
    let kind = match hash.get_active() {
        1 => "SHA256",
        2 => "MD5",
        _ => return
    };

    ui.content.image_view.chooser_container.set_visible_child_name("checksum");

    let _ = state.back_event_tx.send(BackgroundEvent::GenerateHash(
        PathBuf::from(&*path),
        kind
    ));
}
