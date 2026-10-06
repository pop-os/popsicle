use cosmic::{Element, widget};

use crate::app::ActiveView;

pub struct FlashingView;

#[derive(Debug, Clone)]
pub enum Message {}

impl FlashingView {
    pub fn view<'a>(&self) -> impl Into<Element<'a, Message>> {
        widget::text("Flashing...")
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, crate::app::Message>> {
        None
    }
}
