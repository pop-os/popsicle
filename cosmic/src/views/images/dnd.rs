use std::{borrow::Cow, path::PathBuf};

use cosmic::{dialog::ashpd::url::Url, iced::clipboard::mime::AllowedMimeTypes};

#[derive(Clone, Debug)]
pub(crate) struct DroppedFiles {
    pub paths: Vec<PathBuf>,
}

impl AllowedMimeTypes for DroppedFiles {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(vec!["x-special/gnome-copied-files".into(), "text/uri-list".into()])
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

fn parse_file_url(line: &str) -> Result<PathBuf, String> {
    let url = Url::parse(line).map_err(|error| error.to_string())?;

    url.to_file_path().map_err(|_| format!("invalid file URL {url:?}"))
}
