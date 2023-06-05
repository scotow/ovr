use time::Date;
use crate::day::Day;

#[derive(Debug)]
pub struct Catalogue {
    days: Vec<Day>,
}

impl Catalogue {
    pub fn new() -> Self {
        Self {
            days: Vec::new(),
        }
    }

    pub fn insert(&mut self, days: Vec<Day>) -> CatalogueInsert {
        let mut inserts = CatalogueInsert::default();
        for day in days {
            match self.days.binary_search_by_key(&day.date(), |d| d.date()) {
                Ok(to_replace) => {
                    inserts.replaced.push(day.date());
                    self.days[to_replace].replace_dishes(day.dishes())
                }
                Err(insert_position) => {
                    inserts.inserted.push(day.date());
                    self.days.insert(insert_position, day);
                }
            }
        }
        inserts.sort();
        inserts
    }
}

#[derive(Default, Debug)]
pub struct CatalogueInsert {
    pub inserted: Vec<Date>,
    pub replaced: Vec<Date>,
}

impl CatalogueInsert {
    fn sort(&mut self) {
        self.inserted.sort();
        self.replaced.sort();
    }
}