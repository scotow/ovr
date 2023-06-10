use itertools::Itertools;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use time::{Date, Duration, Month, OffsetDateTime, Weekday};

use crate::{
    error::Error,
    response::TextRepresentable,
    utils::{format_date, now_local, to_titlecase},
};

#[derive(Clone, Debug)]
pub struct Day {
    date: Date,
    dishes: Vec<String>,
}

impl Day {
    pub fn new(fields: Vec<String>) -> Result<Option<Day>, Error> {
        match fields.len() {
            0 => return Err(Error::InvalidPdf),
            1 => return Ok(None),
            _ => (),
        };
        let (weekday, day, month) = fields[0]
            .splitn(3, ' ')
            .collect_tuple()
            .ok_or(Error::InvalidPdf)?;
        let weekday = parse_fr_weekday_str(weekday).ok_or(Error::InvalidPdf)?;
        let day = day.parse().map_err(|_| Error::InvalidPdf)?;
        let month = parse_fr_month_str(month).ok_or(Error::InvalidPdf)?;

        let now = OffsetDateTime::now_utc();
        let date = (now.year() - 1..=now.year() + 1)
            .filter_map(|year| {
                let date = Date::from_calendar_date(year, month, day).ok()?;
                (date.weekday() == weekday).then_some(date)
            })
            .min_by_key(|date| (*date - now.date()).abs())
            .ok_or(Error::InvalidPdf)?;

        Ok(Some(Self {
            date,
            dishes: fields[1..].to_vec(),
        }))
    }

    pub fn date(&self) -> Date {
        self.date
    }

    pub fn replace_dishes(&mut self, dishes: Vec<String>) {
        self.dishes = dishes;
    }

    pub fn dishes(self) -> Vec<String> {
        self.dishes
    }

    pub fn dishes_ref(&self) -> &[String] {
        &self.dishes
    }
}

impl Serialize for Day {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Day", 3)?;
        state.serialize_field("date", &format_date(self.date))?;
        state.serialize_field("dishes", &self.dishes)?;
        state.end()
    }
}

impl TextRepresentable for Day {
    fn as_plain_text(&self, human: bool) -> String {
        if human {
            let dishes_str = if self.dishes.len() >= 2 {
                format!(
                    "{} et {}",
                    self.dishes.iter().dropping_back(1).join(", "),
                    self.dishes.last().unwrap(),
                )
            } else {
                self.dishes.iter().join(", ")
            };
            format!("Au menu {} : {}.", format_human_date(self.date), dishes_str)
        } else {
            self.dishes.iter().join("\n")
        }
    }

    fn as_html(&self) -> String {
        let class_str = if self.date == now_local().date() {
            "current"
        } else {
            ""
        };

        format!(
            r#"
            <div class="day {class_str}">
                <a href="/days/{}">{} {} {} {}</a>
                {}
            </div>
        "#,
            format_date(self.date()),
            to_titlecase(weekday_as_fr_str(self.date.weekday())),
            self.date.day(),
            month_as_fr_str(self.date.month()),
            self.date.year(),
            self.dishes
                .iter()
                .map(|dish| format!(r#"<div class="dish">{dish}</div>"#))
                .collect::<String>()
        )
    }
}

fn parse_fr_weekday_str(weekday: &str) -> Option<Weekday> {
    match weekday.to_lowercase().as_str() {
        "lundi" => Some(Weekday::Monday),
        "mardi" => Some(Weekday::Tuesday),
        "mercredi" => Some(Weekday::Wednesday),
        "jeudi" => Some(Weekday::Thursday),
        "vendredi" => Some(Weekday::Friday),
        "samedi" => Some(Weekday::Saturday),
        "dimanche" => Some(Weekday::Sunday),
        _ => None,
    }
}

fn weekday_as_fr_str(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "lundi",
        Weekday::Tuesday => "mardi",
        Weekday::Wednesday => "mercredi",
        Weekday::Thursday => "jeudi",
        Weekday::Friday => "vendredi",
        Weekday::Saturday => "samedi",
        Weekday::Sunday => "dimanche",
    }
}

fn parse_fr_month_str(month: &str) -> Option<Month> {
    match month.to_lowercase().as_str() {
        "janvier" => Some(Month::January),
        "février" | "fevrier" => Some(Month::February),
        "mars" => Some(Month::March),
        "avril" => Some(Month::April),
        "mai" => Some(Month::May),
        "juin" => Some(Month::June),
        "juillet" => Some(Month::July),
        "août" | "aout" => Some(Month::August),
        "septembre" => Some(Month::September),
        "octobre" => Some(Month::October),
        "novembre" => Some(Month::November),
        "décembre" | "decembre" => Some(Month::December),
        _ => None,
    }
}

fn month_as_fr_str(month: Month) -> &'static str {
    match month {
        Month::January => "janvier",
        Month::February => "février",
        Month::March => "mars",
        Month::April => "avril",
        Month::May => "mai",
        Month::June => "juin",
        Month::July => "juillet",
        Month::August => "août",
        Month::September => "septembre",
        Month::October => "octobre",
        Month::November => "novembre",
        Month::December => "décembre",
    }
}

fn format_human_date(date: Date) -> String {
    let today = now_local().date();
    if date == today {
        return "aujourd'hui".to_owned();
    }
    if today.next_day().expect("failed to compute human date") == date {
        return "demain".to_owned();
    }
    let diff = date - today;
    if diff <= Duration::days(7) {
        return format!("{} prochain", weekday_as_fr_str(date.weekday()));
    }
    format!(
        "le {} {} {}",
        weekday_as_fr_str(date.weekday()),
        date.day(),
        month_as_fr_str(date.month())
    )
}
