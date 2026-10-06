use std::{cell::RefCell, fs::File, path::PathBuf};

use cosmic::{
    Element,
    iced::Alignment,
    widget::{self, image::Handle},
};

use crate::fl;

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
        let bytes = include_bytes!("../../resources/images/application-x-cd-image.png");
        let handle = Handle::from_bytes(bytes.to_vec());
        let icon = widget::image(handle).height(50).into();
        let title = widget::text::heading(fl!("image-view-title")).into();
        let description = widget::text::body(fl!("image-view-description")).into();
        let instructions = widget::column(vec![title, description]).into();
        let header = widget::row(vec![icon, instructions]).spacing(spacing.space_s).into();

        let choose_image_button = widget::button::standard(fl!("choose-image-button")).into();
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

        let spacer = widget::space::vertical().into();

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

        let hash_row =
            widget::row(vec![hash_label, hash_combo_box, hash_text_input, hash_check_button])
                .spacing(spacing.space_xxs)
                .align_y(Alignment::Center)
                .padding(spacing.space_s)
                .into();

        widget::column(vec![header, content, spacer, hash_row])
            .align_x(Alignment::Center)
            .spacing(spacing.space_s)
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SetImage { path, size, warning } => self.set_image(&path, size, warning),
            Message::SetHash(idx) => self.selected_hash = idx,
            Message::HashInput(text) => self.hash_input = text,
            Message::CheckHash => todo!(),
        }
    }

    pub fn image_selected(&self) -> bool {
        self.image.borrow().is_some()
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
