use super::{App, Message, page};
use crate::fl;
use cosmic::iced::clipboard::dnd::DndAction;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::{Element, theme};
use std::path::PathBuf;

pub fn images(app: &App) -> Element<'_, Message> {
    let spacing = theme::spacing();
    let image = app.image();
    let hash = app.hash();

    let chooser: Element<'_, Message> = if hash.busy {
        widget::column::with_capacity(2)
            .push(widget::progress_bar::indeterminate_circular().size(32.0))
            .push(widget::text::heading(fl!("generating-checksum")))
            .spacing(spacing.space_xxs)
            .align_x(Alignment::Center)
            .into()
    } else {
        let mut chooser = widget::column::with_capacity(4)
            .push(
                widget::button::standard(fl!("choose-image-button")).on_press(Message::ChooseImage),
            )
            .spacing(spacing.space_xxs)
            .align_x(Alignment::Center);

        chooser = match &image.path {
            Some(path) => {
                let name = path.file_name().map_or_else(
                    || fl!("cannot-select-directories"),
                    |name| name.to_string_lossy().into_owned(),
                );

                chooser
                    .push(widget::text::heading(name))
                    .push(widget::text::body(bytesize::to_string(image.size, true)))
            }
            None => chooser.push(widget::text::heading(fl!("no-image-selected"))),
        };

        chooser
            .push_maybe(
                image
                    .warning
                    .as_deref()
                    .map(|warning| widget::warning(format!("{} {}", fl!("warning"), warning))),
            )
            .into()
    };

    let hashable = hash.selected != 0 && !hash.busy;

    let mut input = widget::text_input("", &hash.input).width(Length::Fill);
    if hashable {
        input = input.on_input(Message::HashInput);
    }
    // The error only styles the input, so the verdict is shown as helper text in both cases.
    input = match hash.matches {
        Some(true) => input.helper_text(fl!("checksum-match")),
        Some(false) => input.helper_text(fl!("checksum-mismatch")).error(""),
        None => input,
    };

    let check = widget::button::suggested(fl!("check-label"))
        .on_press_maybe((hashable && image.path.is_some()).then_some(Message::CheckHash));

    let hash_row = widget::row::with_capacity(4)
        .push(widget::text::body(fl!("hash-label")))
        .push(widget::dropdown(&hash.kinds, Some(hash.selected), Message::HashKind))
        .push(input)
        .push(check)
        .spacing(spacing.space_xxs)
        .align_y(Alignment::Center);

    let content = widget::column::with_capacity(2)
        .push(
            widget::container(chooser)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .push(hash_row)
        .spacing(spacing.space_xs);

    let view =
        page(&app.icons().image, fl!("image-view-title"), fl!("image-view-description"), content);

    // Accept images dropped from a file manager, directly or through the file transfer portal.
    widget::dnd_destination(view, vec!["text/uri-list".into()])
        .action(DndAction::Copy)
        .preferred_action(DndAction::Copy)
        .on_finish(|_mime, data, _action, _x, _y| Message::ImagesDropped(uri_list(&data)))
        .on_file_transfer(Message::FileTransfer)
        .into()
}

/// Parses the local file paths of a `text/uri-list`.
fn uri_list(data: &[u8]) -> Vec<PathBuf> {
    String::from_utf8_lossy(data)
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| url::Url::parse(line.trim()).ok()?.to_file_path().ok())
        .collect()
}
