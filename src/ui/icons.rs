#[allow(unused_imports)]
pub use egui_phosphor::regular::{
    ARROW_UP_RIGHT, CARET_LEFT, CHECK, CHECK_CIRCLE, CHECK_SQUARE, CIRCLE, CLOCK, COLUMNS, COPY,
    CORNERS_IN, CORNERS_OUT, FILE_TEXT, FOLDER, HOURGLASS, LINK, LIST, MAGNIFYING_GLASS, MINUS,
    PLUS, SQUARE, USER, WARNING, X,
};

use crate::model::Filter;

pub fn with_icon(icon: &str, text: &str) -> String {
    format!("{icon}  {text}")
}

pub const fn lane_icon(filter: Filter) -> &'static str {
    match filter {
        Filter::Ready => CHECK_CIRCLE,
        Filter::Open => CIRCLE,
        Filter::Wip => HOURGLASS,
        Filter::Blocked => WARNING,
        Filter::Done => CHECK,
    }
}
