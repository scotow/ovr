use std::{
    mem,
    ops::{AddAssign, Range},
};

use itertools::Itertools;
use lopdf::Document;
use pdf_extract::HTMLOutput;
use regex::Regex;

use crate::{day::Day, error::Error};

const MAIN_CONTENT_AREA: Range<u32> = 120..525;
const CATEGORIES_AREAS: &[Range<u32>] =
    &[(145..160), (205..220), (305..320), (385..400), (425..440)];
const EXPECTED_CHAR_WIDTH: u32 = 4;
const COLUMN_ALLOWED_DRIFT: u32 = 30;
const MULTILINE_DISH_MAX_DISTANCE: u32 = 15;

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
                || CATEGORIES_AREAS.into_iter().any(|r| r.contains(&div.top))
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
                    column.last_mut().unwrap().text += " ";
                    column.last_mut().unwrap().text += &word.text;
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
    fn add_assign(&mut self, mut rhs: Div<'a>) {
        while rhs.text.ends_with("  ") {
            rhs.text = rhs.text.trim_end_matches(' ');
        }
        if self.text.ends_with(' ') {
            rhs.text = rhs.text.trim_start();
        }
        self.end = rhs.left + rhs.text.chars().count() as u32 * EXPECTED_CHAR_WIDTH;
        self.text += rhs.text;
    }
}
