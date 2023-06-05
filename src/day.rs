use itertools::Itertools;
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use time::{
    format_description, Date, Month, OffsetDateTime,
    Weekday,
};

use crate::response::TextRepresentable;

#[derive(Clone, Debug)]
pub struct Day {
    date: Date,
    dishes: Vec<String>,
}

impl Day {
    pub fn new(fields: Vec<String>) -> Result<Option<Day>, ()> {
        match fields.len() {
            0 => return Err(()),
            1 => return Ok(None),
            _ => (),
        };
        let (weekday, day, month) = fields[0].splitn(3, ' ').collect_tuple().unwrap();
        let weekday = parse_fr_weekday_str(weekday).unwrap();
        let day = day.parse().unwrap();
        let month = parse_fr_month_str(month).unwrap();

        let now = OffsetDateTime::now_utc();
        let date = (now.year() - 1..=now.year() + 1)
            .filter_map(|year| {
                let date = Date::from_calendar_date(year, month, day).ok()?;
                (date.weekday() == weekday).then_some(date)
            })
            .min_by_key(|date| (*date - now.date()).abs())
            .unwrap();

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
}

impl Serialize for Day {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Day", 3)?;
        state.serialize_field("date", &self
            .date
            .format(&format_description::parse("[year]-[month]-[day]").unwrap())
            .unwrap())?;
        state.serialize_field("dishes", &self.dishes)?;
        state.end()
    }
}

impl TextRepresentable for Day {
    fn as_text(&self, human: bool) -> String {
        if human {
            if self.dishes.len() >= 2 {
                format!(
                    "Au menu : {} et {}.",
                    self.dishes.iter().dropping_back(1).join(", "),
                    self.dishes.last().unwrap(),
                )
            } else {
                format!("Au menu : {}.", self.dishes.iter().join(", "),)
            }
        } else {
            self.dishes.iter().map(|d| format!("- {d}")).join("\n")
        }
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
