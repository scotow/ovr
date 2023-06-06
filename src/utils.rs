use std::sync::OnceLock;

use time::{
    format_description, format_description::FormatItem, macros::offset, util::days_in_year_month,
    Date, Duration, Month, OffsetDateTime,
};

static FORMATTER: OnceLock<Vec<FormatItem<'static>>> = OnceLock::new();

pub fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| {
        let now = OffsetDateTime::now_utc().to_offset(offset!(+1));
        let start = last_sunday_of_month(now.date(), Month::March)
            .with_hms(2, 0, 0)
            .expect("failed to calculate local date")
            .assume_offset(offset!(+1));
        let end = last_sunday_of_month(now.date(), Month::October)
            .with_hms(3, 0, 0)
            .expect("failed to calculate local date")
            .assume_offset(offset!(+2));

        let offset = if (start..end).contains(&now) {
            offset!(+2)
        } else {
            offset!(+1)
        };
        now.to_offset(offset)
    })
}

fn last_sunday_of_month(date: Date, month: Month) -> Date {
    let month_end =
        Date::from_calendar_date(date.year(), month, days_in_year_month(date.year(), month))
            .expect("failed to calculate local date");
    month_end.saturating_sub(Duration::days((month_end.weekday() as i64 + 1) % 7))
}

pub fn format_date(date: Date) -> String {
    date.format(FORMATTER.get_or_init(|| {
        format_description::parse("[year]-[month]-[day]").expect("invalid date formatter")
    }))
    .expect("date formatting failed")
}

pub fn parse_date(input: &str) -> Option<Date> {
    Date::parse(
        input,
        FORMATTER.get_or_init(|| {
            format_description::parse("[year]-[month]-[day]").expect("invalid date formatter")
        }),
    )
    .ok()
}
