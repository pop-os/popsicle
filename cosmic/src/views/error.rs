use cosmic::{Element, widget};

use crate::app::ActiveView;

pub struct ErrorView;

#[derive(Debug, Clone)]
pub enum Message {}

impl ErrorView {
    pub fn view<'a>(&self) -> impl Into<Element<'a, Message>> {
        widget::text("Error...")
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, crate::app::Message>> {
        None
    }
}
