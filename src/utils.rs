use std::sync::OnceLock;

use time::{format_description, format_description::FormatItem, Date};

static FORMATTER: OnceLock<Vec<FormatItem<'static>>> = OnceLock::new();

pub fn format_date(date: Date) -> String {
    date.format(FORMATTER.get_or_init(|| {
        format_description::parse("[year]-[month]-[day]").expect("invalid date formatter")
    }))
    .expect("date formatting failed")
}
