use cosmic::{Element, widget};

pub struct SummaryView;

#[derive(Debug, Clone)]
pub enum Message {}

impl SummaryView {
    pub fn view<'a>(&self) -> impl Into<Element<'a, Message>> {
        widget::text("Summary...")
    }
}
