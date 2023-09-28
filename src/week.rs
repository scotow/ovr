use std::{
    mem,
    ops::{AddAssign, Range},
};

use itertools::Itertools;
use lopdf::{Document, Object};
use pdf_extract::HTMLOutput;
use regex::Regex;

use crate::{day::Day, error::Error};

const MAIN_CONTENT_AREA: Range<u32> = 120..525;
const CATEGORIES_AREAS: &[(DocumentDimensions, &[Range<u32>])] = &[
    (
        DocumentDimensions {
            width: 792,
            height: 612,
        },
        &[(136..166), (196..226), (296..326), (376..406), (416..446)],
    ),
    (
        DocumentDimensions {
            width: 841,
            height: 595,
        },
        &[(139..169), (197..227), (293..323), (370..400), (408..438)],
    ),
];
const EXPECTED_CHAR_WIDTH: u32 = 4;
const COLUMN_ALLOWED_DRIFT: u32 = 30;
const MULTILINE_DISH_MAX_DISTANCE: u32 = 15;

pub fn parse_json(json_data: &[u8]) -> Result<Vec<Day>, Error> {
    serde_json::from_slice::<Vec<Vec<String>>>(json_data).map_err(|_| Error::InvalidJson)?
        .into_iter()
        .filter_map(|f| Day::new(f).transpose())
        .collect::<Result<Vec<_>, _>>()
}

pub fn parse_pdf(pdf_data: &[u8]) -> Result<Vec<Day>, Error> {
    let document = Document::load_mem(pdf_data).map_err(|_| Error::InvalidPdf)?;
    let mut out_buffer = Vec::new();
    let mut parser = HTMLOutput::new(&mut out_buffer);
    pdf_extract::output_doc(&document, &mut parser).map_err(|_| Error::InvalidPdf)?;

    let html = String::from_utf8(out_buffer)
        .map_err(|_| Error::Internal)?
        .replace("&nbsp;", " ");
    let div_regex = Regex::new(r#"<div style='(.+?)'>(.+?)</div>"#).map_err(|_| Error::Internal)?;
    let top_regex = Regex::new(r#"top:\s?(\d+)(?:\.\d+)?px"#).map_err(|_| Error::Internal)?;
    let left_regex = Regex::new(r#"left:\s?(\d+)(?:\.\d+)?px"#).map_err(|_| Error::Internal)?;

    let categories = DocumentDimensions::new(&document)?.categories_area();
    let mut divs = div_regex
        .captures_iter(&html)
        .filter_map(|capture| {
            let style = &capture[1];
            if style.contains("color: red") {
                return None;
            }
            let div = Div {
                top: top_regex.captures(style)?[1].parse().ok()?,
                left: left_regex.captures(style)?[1].parse().ok()?,
                text: capture.get(2).unwrap().as_str(),
            };
            if !MAIN_CONTENT_AREA.contains(&div.top)
                || categories.into_iter().any(|r| r.contains(&div.top))
            {
                return None;
            }
            Some(div)
        })
        .collect::<Vec<_>>();
    divs.sort_by_key(|d| (d.top, d.left));

    let mut words = Vec::<DishBuilder>::new();
    for div in divs {
        match words.last_mut() {
            Some(last) => {
                if last.top == div.top && last.end.abs_diff(div.left) < 12 {
                    *last += div;
                } else {
                    last.trim();
                    words.push(DishBuilder::from(div));
                }
            }
            None => words.push(DishBuilder::from(div)),
        }
    }
    if let Some(last) = words.last_mut() {
        last.trim();
    }

    // Repeating lines.
    let lines_to_clear = words
        .iter()
        .map(|w| (w.top, w.text.to_lowercase()))
        .into_group_map()
        .into_iter()
        .filter_map(|(t, dishes)| {
            dishes
                .iter()
                .counts()
                .values()
                .any(|&n| n >= dishes.len().saturating_sub(1).max(2)) // Margin error of 1 column.
                .then_some(t)
        })
        .collect::<Vec<_>>();
    words.retain(|w| !lines_to_clear.contains(&w.top));

    // Build columns.
    let mut columns = Vec::<Vec<DishBuilder>>::with_capacity(5);
    for word in words {
        match columns.iter_mut().find(|ow| {
            ow.iter()
                .any(|ow| ow.center().abs_diff(word.center()) < COLUMN_ALLOWED_DRIFT)
        }) {
            Some(column) => {
                // Multiline dishes.
                if word.top - column.last().unwrap().top <= MULTILINE_DISH_MAX_DISTANCE
                    && word.text.chars().next().is_some_and(|c| c.is_lowercase())
                {
                    *column.last_mut().unwrap() += word;
                } else {
                    column.push(word);
                }
            }
            None => columns.push(vec![word]),
        }
    }
    // Remove duplicates.
    for column in &mut columns {
        *column = mem::take(column)
            .into_iter()
            .unique_by(|d| d.text.to_lowercase())
            .collect();
    }
    // Discard empty days.
    columns.retain(|c| c.len() >= 2);

    if columns.is_empty() {
        return Err(Error::InvalidPdf);
    }

    columns
        .into_iter()
        .filter_map(|column| Day::new(column.into_iter().map(|tg| tg.text).collect()).transpose())
        .collect()
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
struct DocumentDimensions {
    width: u32,
    height: u32,
}

impl DocumentDimensions {
    fn new(document: &Document) -> Result<Self, Error> {
        let mut matrix = document
            .get_object(document.page_iter().next().ok_or(Error::InvalidPdf)?)?
            .as_dict()?
            .get("MediaBox".as_bytes())?
            .as_array()?
            .iter()
            .map(|obj| match &obj {
                Object::Integer(n) => Ok(*n as u32),
                Object::Real(r) => Ok(*r as u32),
                _ => Err(Error::InvalidPdf),
            })
            .skip(2);
        Ok(Self {
            width: matrix.next().ok_or(Error::InvalidPdf)??,
            height: matrix.next().ok_or(Error::InvalidPdf)??,
        })
    }

    fn categories_area(&self) -> &'static [Range<u32>] {
        CATEGORIES_AREAS
            .into_iter()
            .find_map(|(d, rs)| (d == self).then_some(*rs))
            .unwrap_or(CATEGORIES_AREAS[0].1)
    }
}

#[derive(Debug)]
struct Div<'a> {
    top: u32,
    left: u32,
    text: &'a str,
}

#[derive(Debug)]
pub struct DishBuilder {
    top: u32,
    start: u32,
    end: u32,
    text: String,
}

impl DishBuilder {
    fn center(&self) -> u32 {
        self.start + (self.end - self.start) / 2
    }

    fn trim(&mut self) {
        self.text = self.text.trim().to_owned();
    }

    fn absorb_text(&mut self, mut text: &str) {
        if self.text.ends_with(' ') {
            text = text.trim_start();
        } else {
            while text.ends_with("  ") {
                text = text.trim_end_matches(' ');
            }
        }
        self.text += text;
    }
}

impl<'a> From<Div<'a>> for DishBuilder {
    fn from(value: Div<'a>) -> Self {
        let text = value.text.trim_start().to_owned();
        DishBuilder {
            top: value.top,
            start: value.left,
            end: value.left + text.chars().count() as u32 * EXPECTED_CHAR_WIDTH,
            text,
        }
    }
}

impl<'a> AddAssign<Div<'a>> for DishBuilder {
    fn add_assign(&mut self, rhs: Div<'a>) {
        self.absorb_text(rhs.text);
        self.end = rhs.left + rhs.text.chars().count() as u32 * EXPECTED_CHAR_WIDTH;
    }
}

// For multiline only.
impl AddAssign<Self> for DishBuilder {
    fn add_assign(&mut self, rhs: Self) {
        if !self.text.ends_with(' ') && !rhs.text.starts_with(' ') {
            self.text.push(' ');
        }
        self.absorb_text(&rhs.text);
        self.start = self.start.min(rhs.start);
        self.end = self.end.max(rhs.end);
    }
}
