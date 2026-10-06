use cosmic::{Element, widget};

pub struct ErrorView;

#[derive(Debug, Clone)]
pub enum Message {}

impl ErrorView {
    pub fn view<'a>(&self) -> impl Into<Element<'a, Message>> {
        widget::text("Error...")
    }
}
