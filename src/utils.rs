use std::sync::OnceLock;

use time::{format_description, format_description::FormatItem, Date, OffsetDateTime, UtcOffset};

static FORMATTER: OnceLock<Vec<FormatItem<'static>>> = OnceLock::new();

pub fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| {
        OffsetDateTime::now_utc()
            .to_offset(UtcOffset::from_hms(1, 0, 0).expect("invalid datetime offset"))
    })
}

pub fn format_date(date: Date) -> String {
    date.format(FORMATTER.get_or_init(|| {
        format_description::parse("[year]-[month]-[day]").expect("invalid date formatter")
    }))
    .expect("date formatting failed")
}
