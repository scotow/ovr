use std::ops::AddAssign;

use itertools::Itertools;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use time::{Date, Duration};

use crate::{
    day::Day,
    response::TextRepresentable,
    utils::{format_date, now_local},
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

    pub fn find_dish_next(&self, dish: &str) -> Option<Day> {
        let dish = dish.to_lowercase();
        let mut now = now_local();
        if now.time().hour() >= 14 {
            now += Duration::days(1);
        }
        self.days
            .iter()
            .find(|day| day.date() >= now.date() && day.dishes_ref().into_iter().map(|d| d.to_lowercase()).any(|d| d.contains(&dish)))
            .cloned()
    }
}

impl TextRepresentable for Catalogue {
    fn as_plain_text(&self, _human: bool) -> String {
        self.days
            .iter()
            .map(|day| format_date(day.date()))
            .join("\n")
    }
}

#[derive(Default, Debug)]
pub struct CatalogueUpdate {
    pub inserted: Vec<Date>,
    pub replaced: Vec<Date>,
}

impl CatalogueUpdate {
    fn sort(&mut self) {
        self.inserted.sort();
        self.replaced.sort();
    }
}

impl AddAssign<Self> for CatalogueUpdate {
    fn add_assign(&mut self, rhs: Self) {
        self.inserted.extend(rhs.inserted);
        self.replaced.extend(rhs.replaced);
    }
}

impl Serialize for CatalogueUpdate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CatalogueUpdate", 3)?;
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

impl TextRepresentable for CatalogueUpdate {}
