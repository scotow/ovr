use std::ops::AddAssign;

use ics::{
    properties::{Description, DtEnd, DtStart, Status, Summary},
    Event, ICalendar,
};
use itertools::Itertools;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use time::{Date, Duration, Weekday};
use uuid::Uuid;

use crate::{
    day::Day,
    error::Error,
    response::TextRepresentable,
    utils::{format_date, format_icalendar_date, now_local},
};

#[derive(Serialize, Clone, Debug)]
pub struct Catalogue {
    days: Vec<Day>,
}

impl Catalogue {
    pub fn new() -> Self {
        Self { days: Vec::new() }
    }

    pub fn insert(&mut self, days: Vec<Day>) -> CatalogueUpdate {
        let mut updates = CatalogueUpdate::default();
        for day in days {
            match self.days.binary_search_by_key(&day.date(), |d| d.date()) {
                Ok(to_replace) => {
                    updates.replaced.push(day.date());
                    self.days[to_replace].replace_dishes(day.dishes())
                }
                Err(insert_position) => {
                    updates.inserted.push(day.date());
                    self.days.insert(insert_position, day);
                }
            }
        }
        updates.sort();
        updates
    }

    pub fn today(&self) -> Option<Day> {
        let today = now_local().date();
        self.days.iter().find(|day| day.date() == today).cloned()
    }

    pub fn next(&self) -> Option<Day> {
        let mut now = now_local();
        if now.time().hour() >= 14 {
            now += Duration::days(1);
        }
        self.days
            .iter()
            .find(|day| day.date() >= now.date())
            .cloned()
    }

    pub fn find_dish_next<'a>(&self, mut search: Vec<String>) -> Option<Day> {
        search.iter_mut().for_each(|d| *d = d.to_lowercase());
        let mut now = now_local();
        if now.time().hour() >= 14 {
            now += Duration::days(1);
        }
        self.days
            .iter()
            .find(|day| {
                day.date() >= now.date()
                    && search.iter().all(|search_dish| {
                        day.dishes_ref()
                            .into_iter()
                            .any(|day_dish| day_dish.to_lowercase().contains(search_dish))
                    })
            })
            .cloned()
    }

    pub fn weeks(&self) -> WeeksList {
        WeeksList::from(self.days.as_slice())
    }

    pub fn week(&self, year: i32, week: u8) -> Result<Self, Error> {
        let days = self
            .days
            .iter()
            .filter(|d| d.date().year() == year && d.date().iso_week() == week)
            .cloned()
            .collect_vec();
        if days.is_empty() {
            Err(Error::WeekNotFound)
        } else {
            Ok(Self { days })
        }
    }

    pub fn day(&self, date: Date) -> Result<Day, Error> {
        self.days
            .iter()
            .find(|d| d.date() == date)
            .cloned()
            .ok_or(Error::DayNotFound)
    }

    pub fn ics(&self) -> Vec<u8> {
        let mut calendar =
            ICalendar::new("2.0", "-//xyz Corp//NONSGML PDA Calendar Version 1.0//EN");
        for day in &self.days {
            let start = day.date().with_hms(12, 00, 00).unwrap();
            let start_str = format_icalendar_date(start);
            let mut event = Event::new(
                Uuid::new_v5(&Uuid::nil(), start_str.as_bytes()).to_string(),
                start_str.clone(),
            );
            event.push(DtStart::new(start_str));
            event.push(DtEnd::new(format_icalendar_date(
                start + Duration::hours(1),
            )));
            event.push(Status::confirmed());
            event.push(Summary::new("Pause déjeuner"));
            event.push(Description::new(ics::escape_text(day.as_plain_text(false))));
            calendar.add_event(event);
        }

        let mut data = Vec::new();
        calendar.write(&mut data).expect("ics file creation failed");
        data
    }
}

impl TextRepresentable for Catalogue {
    fn as_plain_text(&self, human: bool) -> String {
        self.days
            .iter()
            .map(|day| {
                format!(
                    "{} :\n{}",
                    format_date(day.date()),
                    day.as_plain_text(human)
                )
            })
            .join("\n\n")
    }

    fn as_html(&self) -> String {
        self.days.iter().map(Day::as_html).collect()
    }
}

#[derive(Default, Debug)]
pub struct CatalogueUpdate {
    pub inserted: Vec<Date>,
    pub replaced: Vec<Date>,
}

impl CatalogueUpdate {
    pub fn is_empty(&self) -> bool {
        self.inserted.is_empty() && self.replaced.is_empty()
    }

    fn sort(&mut self) {
        self.inserted.sort();
        self.replaced.sort();
    }
}

impl AddAssign<Self> for CatalogueUpdate {
    fn add_assign(&mut self, rhs: Self) {
        self.inserted.extend(rhs.inserted);
        for replaced in rhs.replaced {
            if !self.inserted.contains(&replaced) {
                self.replaced.push(replaced);
            }
        }
    }
}

impl Serialize for CatalogueUpdate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CatalogueUpdate", 2)?;
        state.serialize_field(
            "inserted",
            &self
                .inserted
                .iter()
                .map(|&date| format_date(date))
                .collect_vec(),
        )?;
        state.serialize_field(
            "replaced",
            &self
                .replaced
                .iter()
                .map(|&date| format_date(date))
                .collect_vec(),
        )?;
        state.end()
    }
}

impl TextRepresentable for CatalogueUpdate {
    fn as_plain_text(&self, _human: bool) -> String {
        let mut text = String::new();
        if !self.inserted.is_empty() {
            text += "Inserted:\n";
            text += &self
                .inserted
                .iter()
                .map(|&date| format_date(date))
                .join("\n");
        }
        if !self.replaced.is_empty() {
            if !text.is_empty() {
                text += "\n\n";
            }
            text += "Replaced:\n";
            text += &self
                .replaced
                .iter()
                .map(|&date| format_date(date))
                .join("\n");
        }
        text
    }
}

pub struct WeeksList {
    weeks: Vec<Date>,
}

impl From<&[Day]> for WeeksList {
    fn from(days: &[Day]) -> Self {
        Self {
            weeks: days
                .into_iter()
                .map(|d| {
                    Date::from_iso_week_date(d.date().year(), d.date().iso_week(), Weekday::Monday)
                        .expect("week list creation failed")
                })
                .unique()
                .collect(),
        }
    }
}

impl Serialize for WeeksList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Week {
            from: String,
            to: String,
        }

        let mut state = serializer.serialize_struct("WeeksList", 1)?;
        state.serialize_field(
            "weeks",
            &self
                .weeks
                .iter()
                .map(|w| Week {
                    from: format_date(*w),
                    to: format_date(*w + (Duration::days(4))),
                })
                .collect_vec(),
        )?;
        state.end()
    }
}

impl TextRepresentable for WeeksList {
    fn as_plain_text(&self, _human: bool) -> String {
        self.weeks
            .iter()
            .map(|week| format!("{}-{}", week.year(), week.iso_week()))
            .join("\n")
    }

    fn as_html(&self) -> String {
        let today = now_local().date();
        let current = (today.year(), today.iso_week());
        self.weeks
            .iter()
            .map(|week| {
                let class_str = if (week.year(), week.iso_week()) == current {
                    "current"
                } else {
                    ""
                };
                format!(
                    r#"<a href="/weeks/{}-{}" class="week {class_str}">Semaine {} - {}</a>"#,
                    week.year(),
                    week.iso_week(),
                    week.iso_week(),
                    week.year()
                )
            })
            .collect()
    }
}
