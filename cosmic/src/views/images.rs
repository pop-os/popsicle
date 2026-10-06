use std::{cell::RefCell, fs::File, path::PathBuf};

use cosmic::{
    Element, Task,
    dialog::{ashpd::url::Url, file_chooser},
    iced::Alignment,
    theme::spacing,
    widget,
};

use crate::{
    app::{self, ActiveView},
    fl,
};

pub struct ImagesView {
    image: RefCell<Option<File>>,
    image_path: Option<PathBuf>,
    image_name: Option<String>,
    image_size: Option<String>,
    error: Option<String>,
    hashes: Vec<String>,
    selected_hash: usize,
    hash_input: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetImage { path: PathBuf, size: u64, warning: Option<String> },
    SetHash(usize),
    HashInput(String),
    CheckHash,
    ChooseImage,
    FilePicked(Vec<Url>),
    PickCancelled,
    PickFailed,
}

impl ImagesView {
    pub fn new() -> Self {
        Self {
            image: RefCell::new(None),
            image_path: None,
            image_name: None,
            image_size: None,
            error: None,
            hashes: vec![
                fl!("none"),
                "SHA512".into(),
                "SHA256".into(),
                "SHA1".into(),
                "MD5".into(),
                "BLAKE2b".into(),
            ],
            selected_hash: 0,
            hash_input: String::new(),
        }
    }

    pub fn view<'a>(&'a self) -> impl Into<Element<'a, Message>> {
        let spacing = cosmic::theme::spacing();
        let title = widget::text::title2(fl!("image-view-title")).into();
        let description = widget::text::body(fl!("image-view-description")).into();
        let instructions = widget::column(vec![title, description]).into();

        let choose_image_button = widget::button::standard(fl!("choose-image-button"))
            .on_press(Message::ChooseImage)
            .into();
        let image_name_caption = widget::text::caption_heading(
            self.image_name.clone().unwrap_or(fl!("no-image-selected")),
        )
        .into();
        let image_size_caption = self.image_size.as_ref().map(|size| widget::text::caption(size));
        let content = widget::column(vec![choose_image_button, image_name_caption])
            .push_maybe(image_size_caption)
            .align_x(Alignment::Center)
            .spacing(spacing.space_xs)
            .into();

        let hash_label = widget::text(fl!("hash-label")).into();

        let hash_combo_box =
            widget::dropdown(&self.hashes, Some(self.selected_hash), Message::SetHash).into();

        let hash_input_active = self.selected_hash > 0;
        let mut hash_text_input = widget::text_input("", &self.hash_input);

        if hash_input_active {
            hash_text_input = hash_text_input.on_input(Message::HashInput);
            hash_text_input = hash_text_input.on_paste(Message::HashInput);
        }

        let hash_text_input = hash_text_input.into();

        let hash_check_button_enabled = !self.hash_input.is_empty();

        let hash_check_button = widget::button::standard(fl!("check-label"))
            .on_press_maybe(hash_check_button_enabled.then(|| Message::CheckHash))
            .into();

        let hash_spacer = widget::space::horizontal().into();

        let hash_row =
            widget::row(vec![hash_label, hash_combo_box, hash_spacer, hash_check_button])
                .spacing(spacing.space_xxs)
                .align_y(Alignment::Center)
                .into();

        let hash_col =
            widget::column(vec![hash_row, hash_text_input]).spacing(spacing.space_xxs).into();

        widget::column(vec![instructions, content, hash_col])
            .align_x(Alignment::Center)
            .spacing(spacing.space_m)
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, app::Message>> {
        let can_press = *view == ActiveView::Images && self.image.borrow().is_some();
        let next = widget::button::suggested(fl!("next"))
            .on_press_maybe(can_press.then(|| app::Message::Next))
            .into();

        let cancel = widget::button::standard(fl!("cancel")).on_press(app::Message::Cancel).into();
        let spacer = widget::space::horizontal().into();
        let row = widget::row(vec![spacer, cancel, next])
            .spacing(spacing().space_xs)
            .padding(spacing().space_xs)
            .into();
        Some(row)
    }

    pub fn update(&mut self, message: Message) -> Option<Task<cosmic::Action<Message>>> {
        match message {
            Message::SetImage { path, size, warning } => self.set_image(&path, size, warning),
            Message::SetHash(idx) => self.selected_hash = idx,
            Message::HashInput(text) => self.hash_input = text,
            Message::CheckHash => todo!(),
            Message::ChooseImage => {
                let task = cosmic::task::future(async {
                    let dialog = file_chooser::open::Dialog::new().title("Choose a file");
                    match dialog.open_file().await {
                        Ok(response) => Message::FilePicked(response.0.uris().to_vec().clone()),
                        Err(file_chooser::Error::Cancelled) => Message::PickCancelled,
                        Err(why) => {
                            eprintln!("{why:?}");
                            Message::PickFailed
                        }
                    }
                });
                return Some(task);
            }
            Message::FilePicked(urls) => {
                let Some(path) = urls.first().and_then(|url| url.to_file_path().ok()) else {
                    eprintln!("picked URI is not a local file: {urls:?}");
                    return None;
                };
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                self.set_image(&path, size, None)
            }
            Message::PickCancelled => todo!("Use a toaster to inform the user"),
            Message::PickFailed => todo!("Use a toaster to inform the user"),
        }
        None
    }

    pub fn set_image(&mut self, path: &PathBuf, size: u64, warning: Option<String>) {
        let size_str = bytesize::to_string(size, true);
        match path.file_name() {
            Some(name) => {
                self.image_name = Some(name.to_string_lossy().to_string());
                self.image_size = Some(size_str);
            }
            None => self.error = Some(fl!("cannot-select-directories")),
        }

        if let Some(warning) = warning {
            self.error = Some(warning);
        };

        if let Ok(file) = File::open(&path) {
            self.image.replace(Some(file));
        } else {
            self.error = Some(fl!("iso-open-failed"));
        }

        self.image_path = Some(path.clone());
    }
}
