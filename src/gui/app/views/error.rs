use super::{App, Message, page};
use crate::fl;
use cosmic::Element;
use cosmic::widget;

pub fn error(app: &App) -> Element<'_, Message> {
    page(&app.icons().error, fl!("critical-error"), app.error(), widget::space::vertical())
}
