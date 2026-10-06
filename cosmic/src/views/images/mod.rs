use std::{
    fs::File,
    path::{Path, PathBuf},
};

use blake2::Blake2b512;
use cosmic::{
    Apply, Element, Task,
    dialog::{ashpd::url::Url, file_chooser},
    iced::{Alignment, Length},
    theme::spacing,
    widget::{self},
};
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512};

use crate::{
    app::{self, ActiveView},
    fl,
    hash::{HashResult, hasher},
    views::images::{
        dnd::DroppedFiles,
        style::{colored_text_input, drag_area_active},
    },
};

pub mod dnd;
pub mod style;

#[derive(Debug, Default)]
pub struct ImageEntry {
    file: Option<File>,
    path: Option<PathBuf>,
    size: u64,
}

impl ImageEntry {
    pub fn is_loaded(&self) -> bool {
        self.file.is_some() && self.path.is_some() && self.size > 0
    }

    pub fn name(&self) -> Option<String> {
        let Some(path) = &self.path else {
            return None;
        };

        let Some(file_name) = path.file_name() else {
            return None;
        };

        Some(file_name.to_string_lossy().to_string())
    }

    pub fn size_str(&self) -> String {
        bytesize::to_string(self.size, true)
    }
}

pub struct ImagesView {
    image: ImageEntry,
    error: Option<String>,
    hashes: Vec<String>,
    selected_hash: usize,
    hash_input: String,
    dragging: bool,
    hash_result: Option<HashResult>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetHash(usize),
    HashInput(String),
    CheckHash,
    ChooseImage,
    HashCalculated { expected: String, result: Result<String, String> },

    FileDropped(PathBuf),
    DropFailed,
    DragEntered,
    DragLeft,

    FilePicked(Vec<Url>),
    PickCancelled,
    PickFailed,
}

impl ImagesView {
    pub fn new() -> Self {
        Self {
            image: ImageEntry::default(),
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
            dragging: false,
            hash_result: None,
        }
    }

    pub fn image_path(&self) -> Option<&Path> {
        self.image.path.as_deref()
    }

    pub fn image_size(&self) -> u64 {
        self.image.size
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let image_selected = self.image.is_loaded();

        widget::column([])
            .push(self.instructions())
            .push(self.image_drop_area(image_selected))
            .push_maybe(image_selected.then(|| self.hash_section()))
            .align_x(Alignment::Center)
            .spacing(spacing().space_xs)
            .apply(widget::scrollable)
            .spacing(spacing().space_xs)
            .into()
    }

    fn instructions(&self) -> Element<'_, Message> {
        let title = widget::text::title2(fl!("image-view-title"));
        let description = widget::text::body(fl!("image-view-description"));

        widget::column([])
            .push(title)
            .push(description)
            .width(Length::Fill)
            .spacing(spacing().space_xxs)
            .into()
    }

    fn image_drop_area(&self, image_selected: bool) -> Element<'_, Message> {
        let drop_icon = widget::icon::from_name(if self.dragging {
            "document-save-symbolic"
        } else if image_selected {
            "drive-optical-symbolic"
        } else {
            "document-open-symbolic"
        })
        .size(40)
        .symbolic(true);

        let drop_title = widget::text::heading(if self.dragging {
            fl!("drop-iso-here")
        } else if image_selected {
            self.image.name().unwrap_or(fl!("image-selected"))
        } else {
            fl!("drop-iso-file-here")
        })
        .align_x(Alignment::Center);

        let drop_description = widget::text::caption(if self.dragging {
            fl!("release-to-use-image")
        } else if image_selected {
            self.image.size_str()
        } else {
            fl!("choose-a-file")
        })
        .align_x(Alignment::Center);

        let choose_image_label =
            if image_selected { fl!("change-image-button") } else { fl!("choose-image-button") };

        let choose_image_button =
            widget::button::standard(choose_image_label).on_press(Message::ChooseImage);

        let image_content = widget::column([])
            .push(drop_icon)
            .push(drop_title)
            .push(drop_description)
            .push(choose_image_button)
            .align_x(Alignment::Center)
            .spacing(spacing().space_xs);

        let mut drop_container = widget::container(image_content)
            .width(Length::Fill)
            .height(200)
            .center_x(Length::Fill)
            .center_y(200)
            .padding(spacing().space_m)
            .class(cosmic::theme::Container::Card);

        if self.dragging {
            drop_container = drop_container.style(drag_area_active);
        }

        widget::DndDestination::for_data::<DroppedFiles>(drop_container, |data, _action| {
            let Some(data) = data else {
                return Message::DropFailed;
            };

            let Some(path) = data.paths.into_iter().next() else {
                return Message::DropFailed;
            };

            Message::FileDropped(path)
        })
        .on_enter(|_, _, _| Message::DragEntered)
        .on_leave(|| Message::DragLeft)
        .into()
    }

    fn hash_section(&self) -> Element<'_, Message> {
        let hash_label = widget::text::body(fl!("hash-label")).font(cosmic::font::bold());

        let hash_dropdown =
            widget::dropdown(&self.hashes, Some(self.selected_hash), Message::SetHash);

        let show_hash_input = self.selected_hash > 0;

        let mut hash_text_input = widget::text_input("", &self.hash_input).width(Length::Fill);

        if let Some(hash_result) = self.hash_result {
            hash_text_input = hash_text_input.style(colored_text_input(hash_result));
        }

        if show_hash_input {
            hash_text_input =
                hash_text_input.on_input(Message::HashInput).on_paste(Message::HashInput);
        }

        match self.hash_result {
            Some(HashResult::Checking) => {
                hash_text_input = hash_text_input
                    .trailing_icon(widget::indeterminate_circular().size(16.0).into());
            }

            Some(HashResult::Match) => {}

            Some(HashResult::Mismatch) => {
                hash_text_input = hash_text_input.error(fl!("hash-mismatch"));
            }

            Some(HashResult::Error) => {
                hash_text_input = hash_text_input.error(fl!("hash-error"));
            }

            None => {}
        }

        let hash_check_enabled = !self.hash_input.trim().is_empty()
            && self.selected_hash > 0
            && !matches!(self.hash_result, Some(HashResult::Checking));

        let hash_check_button = widget::button::standard(fl!("check-label"))
            .on_press_maybe(hash_check_enabled.then_some(Message::CheckHash));

        let hash_row = widget::row([])
            .push(hash_label)
            .push(hash_dropdown)
            .push(widget::space::horizontal())
            .push_maybe(show_hash_input.then_some(hash_check_button))
            .spacing(spacing().space_xs)
            .align_y(Alignment::Center);

        let error_message = self.error.as_ref().map(|error| widget::text::caption(error));

        let hash_content = widget::column([])
            .push_maybe(show_hash_input.then_some(hash_text_input))
            .push(hash_row)
            .push_maybe(error_message)
            .align_x(Alignment::Center)
            .spacing(spacing().space_xs);

        widget::container(hash_content)
            .width(Length::Fill)
            .padding(spacing().space_m)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, app::Message>> {
        let can_press = *view == ActiveView::Images && self.image.is_loaded();

        let next = widget::button::suggested(fl!("next"))
            .on_press_maybe(can_press.then_some(app::Message::Next))
            .into();

        let spacer = widget::space::horizontal().into();

        let row = widget::row(vec![spacer, next])
            .spacing(spacing().space_xs)
            .padding(spacing().space_xs)
            .into();

        Some(row)
    }

    pub fn update(&mut self, message: Message) -> Option<Task<cosmic::Action<Message>>> {
        match message {
            Message::SetHash(idx) => {
                self.selected_hash = idx;
                self.hash_result = None;
            }

            Message::HashInput(text) => {
                self.hash_input = text;
                self.hash_result = None;
            }

            Message::CheckHash => return self.check_hash(),

            Message::HashCalculated { expected, result } => {
                self.hash_calculated(expected, result);
            }

            Message::ChooseImage => return self.choose_image(),

            Message::FilePicked(urls) => {
                self.file_picked(urls);
            }

            Message::FileDropped(path) => {
                self.dragging = false;
                self.load_file(path);
            }

            Message::DropFailed => {
                self.dragging = false;
                self.error = Some("Could not read the dropped file.".into());
            }

            Message::DragEntered => {
                self.dragging = true;
            }

            Message::DragLeft => {
                self.dragging = false;
            }

            Message::PickCancelled => {}

            Message::PickFailed => {
                todo!("Use a toaster to inform the user");
            }
        }

        None
    }

    fn check_hash(&mut self) -> Option<Task<cosmic::Action<Message>>> {
        let path = self.image.path.clone()?;

        let expected = self.hash_input.trim().to_ascii_lowercase();

        let algorithm = self.selected_hash;

        if expected.is_empty() || algorithm == 0 {
            return None;
        }

        self.hash_result = Some(HashResult::Checking);

        let task = cosmic::task::future(async move {
            let result = calculate_hash(&path, algorithm).await.map_err(|error| error.to_string());

            Message::HashCalculated { expected, result }
        })
        .map(cosmic::Action::App);

        Some(task)
    }

    fn hash_calculated(&mut self, expected: String, result: Result<String, String>) {
        match result {
            Ok(actual) => {
                self.hash_result = Some(if actual.eq_ignore_ascii_case(&expected) {
                    HashResult::Match
                } else {
                    HashResult::Mismatch
                });
            }

            Err(error) => {
                eprintln!("hash calculation failed: {error}");
                self.error = Some(error);
                self.hash_result = Some(HashResult::Error);
            }
        }
    }

    fn choose_image(&self) -> Option<Task<cosmic::Action<Message>>> {
        let task = cosmic::task::future(async {
            let dialog = file_chooser::open::Dialog::new().title("Choose a file");

            match dialog.open_file().await {
                Ok(response) => Message::FilePicked(response.0.uris().to_vec()),

                Err(file_chooser::Error::Cancelled) => Message::PickCancelled,

                Err(why) => {
                    eprintln!("{why:?}");
                    Message::PickFailed
                }
            }
        })
        .map(cosmic::Action::App);

        Some(task)
    }

    fn file_picked(&mut self, urls: Vec<Url>) {
        let Some(path) = urls.first().and_then(|url| url.to_file_path().ok()) else {
            eprintln!("picked URI is not a local file: {urls:?}");
            return;
        };

        let size = std::fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0);

        self.set_image(&path, size, None);
    }

    fn load_file(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.error = Some(fl!("cannot-select-directories"));
            return;
        }

        let size = match std::fs::metadata(&path) {
            Ok(metadata) => metadata.len(),

            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };

        self.set_image(&path, size, None);
    }

    fn set_image(&mut self, path: &PathBuf, size: u64, warning: Option<String>) {
        self.hash_result = None;
        self.error = None;

        if let Some(warning) = warning {
            self.error = Some(warning);
            return;
        }

        match File::open(path) {
            Ok(file) => {
                self.image.file.replace(file);
            }

            Err(_) => {
                self.error = Some(fl!("iso-open-failed"));
                return;
            }
        }

        self.image.size = size;
        self.image.path = Some(path.clone());
    }
}

async fn calculate_hash(path: &Path, algorithm: usize) -> std::io::Result<String> {
    match algorithm {
        1 => hasher::<Sha512>(path).await,
        2 => hasher::<Sha256>(path).await,
        3 => hasher::<Sha1>(path).await,
        4 => hasher::<Md5>(path).await,
        5 => hasher::<Blake2b512>(path).await,

        _ => {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No hash algorithm selected"))
        }
    }
}
