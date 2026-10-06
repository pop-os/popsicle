use std::{
    cell::RefCell,
    fs::File,
    path::{Path, PathBuf},
};

use blake2::Blake2b512;
use cosmic::{
    Apply, Element, Task,
    dialog::{ashpd::url::Url, file_chooser},
    iced::{Alignment, Border, Color, Length, Theme, clipboard::mime::AllowedMimeTypes},
    theme::spacing,
    widget,
};
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512};

use crate::{
    app::{self, ActiveView},
    fl,
    hash::{HashResult, hasher},
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
    dragging: bool,
    hash_result: Option<HashResult>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetImage { path: PathBuf, size: u64, warning: Option<String> },
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
            dragging: false,
            hash_result: None,
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let title = widget::text::title2(fl!("image-view-title"));
        let description = widget::text::body(fl!("image-view-description"));

        let instructions = widget::column([])
            .push(title)
            .push(description)
            .width(Length::Fill)
            .spacing(spacing().space_xxs);

        let image_selected = self.image.borrow().is_some();

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
            self.image_name.clone().unwrap_or(fl!("image-selected"))
        } else {
            fl!("drop-iso-file-here")
        })
        .align_x(Alignment::Center);

        let drop_description = widget::text::caption(if self.dragging {
            fl!("release-to-use-image")
        } else if image_selected {
            self.image_size.as_deref().unwrap_or("").to_string()
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

        let drop_area =
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
            .on_leave(|| Message::DragLeft);

        let hash_label = widget::text::body(fl!("hash-label")).font(cosmic::font::bold());

        let hash_dropdown =
            widget::dropdown(&self.hashes, Some(self.selected_hash), Message::SetHash);

        let hash_input_active = self.selected_hash > 0;

        let mut hash_text_input = widget::text_input("", &self.hash_input).width(Length::Fill);

        if hash_input_active {
            hash_text_input =
                hash_text_input.on_input(Message::HashInput).on_paste(Message::HashInput);
        }

        let hash_check_enabled = !self.hash_input.trim().is_empty()
            && self.selected_hash > 0
            && !matches!(self.hash_result, Some(HashResult::Checking));

        let hash_check_button = widget::button::standard(fl!("check-label"))
            .on_press_maybe(hash_check_enabled.then_some(Message::CheckHash));

        let show_hash_input = self.selected_hash > 0;

        let hash_row = widget::row([])
            .push(hash_label)
            .push(hash_dropdown)
            .push(widget::space::horizontal())
            .push_maybe(show_hash_input.then_some(hash_check_button))
            .spacing(spacing().space_xs)
            .align_y(Alignment::Center);

        let hash_result = self.hash_result.map(|result| match result {
            HashResult::Checking => widget::text::body(fl!("hash-checking")),
            HashResult::Match => {
                widget::text::body(fl!("hash-match")).class(cosmic::style::Text::Accent)
            }
            HashResult::Mismatch => widget::text::body(fl!("hash-mismatch"))
                .class(cosmic::style::Text::Color(Color::from_rgb(0.9, 0.3, 0.3))),
            HashResult::Error => widget::text::body(fl!("hash-error"))
                .class(cosmic::style::Text::Color(Color::from_rgb(0.9, 0.3, 0.3))),
        });

        let error_message = self.error.as_ref().map(|error| widget::text::caption(error));

        let hash_content = widget::column([])
            .push_maybe(show_hash_input.then_some(hash_text_input))
            .push(hash_row)
            .push_maybe(hash_result)
            .push_maybe(error_message)
            .align_x(Alignment::Center)
            .spacing(spacing().space_xs);

        let hash_section = widget::container(hash_content)
            .width(Length::Fill)
            .padding(spacing().space_m)
            .class(cosmic::theme::Container::Card);

        widget::column([])
            .push(instructions)
            .push(drop_area)
            .push_maybe(image_selected.then_some(hash_section))
            .align_x(Alignment::Center)
            .spacing(spacing().space_xs)
            .apply(widget::scrollable)
            .spacing(spacing().space_xs)
            .into()
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, app::Message>> {
        let can_press = *view == ActiveView::Images && self.image.borrow().is_some();

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
            Message::SetImage { path, size, warning } => self.set_image(&path, size, warning),

            Message::SetHash(idx) => {
                self.selected_hash = idx;
                self.hash_result = None;
            }

            Message::HashInput(text) => {
                self.hash_input = text;
                self.hash_result = None;
            }

            Message::CheckHash => {
                let Some(path) = self.image_path.clone() else {
                    return None;
                };

                let expected = self.hash_input.trim().to_ascii_lowercase();
                let algorithm = self.selected_hash;

                if expected.is_empty() || algorithm == 0 {
                    return None;
                }

                self.hash_result = Some(HashResult::Checking);

                let task = cosmic::task::future(async move {
                    let result =
                        calculate_hash(&path, algorithm).await.map_err(|error| error.to_string());

                    Message::HashCalculated { expected, result }
                })
                .map(cosmic::Action::App);

                return Some(task);
            }

            Message::HashCalculated { expected, result } => {
                eprintln!("Received HashCalculated");

                match result {
                    Ok(actual) => {
                        eprintln!("Actual:   {actual}");
                        eprintln!("Expected: {expected}");

                        self.hash_result = Some(if actual.eq_ignore_ascii_case(&expected) {
                            HashResult::Match
                        } else {
                            HashResult::Mismatch
                        });
                    }

                    Err(error) => {
                        eprintln!("Hash calculation failed: {error}");
                        self.error = Some(error);
                        self.hash_result = Some(HashResult::Error);
                    }
                }
            }

            Message::ChooseImage => {
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

                return Some(task);
            }

            Message::FilePicked(urls) => {
                let Some(path) = urls.first().and_then(|url| url.to_file_path().ok()) else {
                    eprintln!("picked URI is not a local file: {urls:?}");
                    return None;
                };

                let size = std::fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0);

                self.set_image(&path, size, None);
            }

            Message::PickCancelled => {}

            Message::PickFailed => {
                todo!("Use a toaster to inform the user");
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
        }

        None
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

        let size_str = bytesize::to_string(size, true);

        match path.file_name() {
            Some(name) => {
                self.image_name = Some(name.to_string_lossy().to_string());
                self.image_size = Some(size_str);
            }
            None => {
                self.error = Some(fl!("cannot-select-directories"));
            }
        }

        if let Some(warning) = warning {
            self.error = Some(warning);
        }

        if let Ok(file) = File::open(path) {
            self.image.replace(Some(file));
        } else {
            self.error = Some(fl!("iso-open-failed"));
        }

        self.image_path = Some(path.clone());
    }
}

#[derive(Clone, Debug)]
struct DroppedFiles {
    paths: Vec<PathBuf>,
}

impl AllowedMimeTypes for DroppedFiles {
    fn allowed() -> std::borrow::Cow<'static, [String]> {
        std::borrow::Cow::Owned(vec![
            "x-special/gnome-copied-files".to_string(),
            "text/uri-list".to_string(),
        ])
    }
}

impl TryFrom<(Vec<u8>, String)> for DroppedFiles {
    type Error = String;

    fn try_from((data, mime): (Vec<u8>, String)) -> Result<Self, Self::Error> {
        let text = std::str::from_utf8(&data).map_err(|error| error.to_string())?;
        let mut lines = text.lines();

        let paths = match mime.as_str() {
            "text/uri-list" => lines
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(parse_file_url)
                .collect::<Result<Vec<_>, _>>()?,

            "x-special/gnome-copied-files" => {
                let operation =
                    lines.next().ok_or_else(|| "missing clipboard operation".to_string())?;

                match operation {
                    "copy" | "cut" => {}
                    _ => {
                        return Err(format!("unsupported clipboard operation {operation:?}"));
                    }
                }

                lines
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .map(parse_file_url)
                    .collect::<Result<Vec<_>, _>>()?
            }

            _ => {
                return Err(format!("unsupported MIME type {mime:?}"));
            }
        };

        Ok(Self { paths })
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

fn parse_file_url(line: &str) -> Result<PathBuf, String> {
    let url = Url::parse(line).map_err(|error| error.to_string())?;

    url.to_file_path().map_err(|_| format!("invalid file URL {url:?}"))
}

fn drag_area_active(theme: &cosmic::prelude::Theme) -> widget::popover::Style {
    let mut style = widget::container::Style::default();
    let cosmic = theme.cosmic();

    let mut background = cosmic.accent_color();
    background.alpha = 0.2;

    style.background = Some(Color::from(background).into());
    style.border = Border {
        color: cosmic.accent_color().into(),
        width: 1.0,
        radius: cosmic.radius_s().into(),
    };

    style
}
