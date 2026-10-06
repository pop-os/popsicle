use cosmic::{Element, widget};

pub struct FlashingView;

#[derive(Debug, Clone)]
pub enum Message {}

impl FlashingView {
    pub fn view<'a>(&self) -> impl Into<Element<'a, Message>> {
        widget::text("Flashing...")
    }
}
