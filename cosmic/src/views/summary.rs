use cosmic::{Element, widget};

use crate::app::ActiveView;

pub struct SummaryView;

#[derive(Debug, Clone)]
pub enum Message {}

impl SummaryView {
    pub fn view<'a>(&self) -> impl Into<Element<'a, Message>> {
        widget::text("Summary...")
    }

    pub fn footer(&self, view: &ActiveView) -> Option<Element<'_, crate::app::Message>> {
        None
    }
}
