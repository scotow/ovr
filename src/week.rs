use std::mem;
use std::ops::AddAssign;

use itertools::Itertools;
use lopdf::Document;
use pdf_extract::HTMLOutput;
use regex::Regex;

use crate::{day::Day, error::Error};

const MIN_WORDS_PER_LINE: usize = 3;

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
            if !(100..530).contains(&div.top) {
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
                if last.top == div.top && last.end.abs_diff(div.left) < 30 {
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

    let lines_to_clear = words
        .iter()
        .map(|w| w.top)
        .counts()
        .into_iter()
        .filter_map(|(t, n)| (n < MIN_WORDS_PER_LINE).then_some(t))
        .collect::<Vec<_>>();
    words.retain(|w| !lines_to_clear.contains(&w.top));

    let mut columns = Vec::<Vec<DishBuilder>>::with_capacity(5);
    for word in words {
        match columns
            .iter_mut()
            .find(|ow| ow.iter().any(|ow| ow.center().abs_diff(word.center()) < 30))
        {
            Some(column) => column.push(word),
            None => columns.push(vec![word]),
        }
    }
    // Remove duplicates.
    for column in &mut columns {
        *column = mem::take(column).into_iter().unique_by(|d| d.text.to_lowercase()).collect();
    }
    // Remove too frequent dishes.
    if columns.len() >= 4 {
        let mut dishes_counts = columns.iter().flat_map(|c| c.iter()).map(|d| d.text.to_lowercase()).counts();
        dishes_counts.retain(|_, n| *n >= columns.len() - 1);
        for column in &mut columns {
            column.retain(|tg| !dishes_counts.contains_key(&tg.text.to_lowercase()));
        }
    }
    // Discard empty days.
    columns.retain(|c| c.len() >= 2);

    columns
        .into_iter()
        .filter_map(|column| {
            Day::new(column.into_iter().map(|tg| tg.text).collect()).transpose()
        })
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
            end: value.left + text.chars().count() as u32 * 4,
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
        self.end = rhs.left + rhs.text.chars().count() as u32 * 4;
        self.text += rhs.text;
    }
}
