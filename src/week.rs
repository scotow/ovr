use std::ops::AddAssign;
use itertools::Itertools;
use lopdf::Document;
use pdf_extract::HTMLOutput;
use regex::Regex;
use crate::day::Day;

const MIN_WORDS_PER_LINE: usize = 3;

pub fn parse_pdf(pdf_data: &[u8]) -> Result<Vec<Day>, ()> {
    let document = Document::load_mem(pdf_data).map_err(|_| ())?;
    let mut out_buffer = Vec::new();
    let mut parser = HTMLOutput::new(&mut out_buffer);
    pdf_extract::output_doc(&document, &mut parser).map_err(|_| ())?;

    let html = String::from_utf8(out_buffer)
        .unwrap()
        .replace("&nbsp;", " ");
    let div_regex = Regex::new(r#"<div style='(.+?)'>(.+?)</div>"#).unwrap();
    let top_regex = Regex::new(r#"top:\s?(\d+)(?:\.\d+)?px"#).unwrap();
    let left_regex = Regex::new(r#"left:\s?(\d+)(?:\.\d+)?px"#).unwrap();

    let mut groups = div_regex
        .captures_iter(&html)
        .filter_map(|capture| {
            let style = &capture[1];
            if style.contains("color: red") {
                return None;
            }
            let group = Div {
                top: top_regex.captures(style).unwrap()[1].parse().unwrap(),
                left: left_regex.captures(style).unwrap()[1].parse().unwrap(),
                text: capture.get(2).unwrap().as_str(),
            };
            if !(100..530).contains(&group.top) {
                return None;
            }
            Some(Div {
                top: top_regex.captures(style).unwrap()[1].parse().unwrap(),
                left: left_regex.captures(style).unwrap()[1].parse().unwrap(),
                text: capture.get(2).unwrap().as_str(),
            })
        })
        .collect::<Vec<_>>();
    groups.sort_by_key(|d| (d.top, d.left));

    let mut words = Vec::<TextGroup>::new();
    for g in groups {
        match words.last_mut() {
            Some(last) => {
                if last.top == g.top && g.left.abs_diff(last.end) < 30 {
                    *last += g;
                } else {
                    last.trim();
                    words.push(TextGroup::from(g));
                }
            }
            None => words.push(TextGroup::from(g)),
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

    let mut dishes_counts = words
        .iter()
        .map(|w| w.text.to_lowercase())
        .counts();

    let mut columns = Vec::<Vec<TextGroup>>::with_capacity(5);
    for word in words {
        match columns
            .iter_mut()
            .find(|ow| ow.iter().any(|ow| ow.center().abs_diff(word.center()) < 30))
        {
            Some(column) => column.push(word),
            None => columns.push(vec![word]),
        }
    }
    // Discard empty days.
    columns.retain(|c| c.len() >= 2);

    // Remove lines containing the same dish.
    if columns.len() >= 4 {
        dishes_counts.retain(|_, n| *n >= columns.len() - 1);
        for column in &mut columns {
            column.retain(|tg| !dishes_counts.contains_key(&tg.text.to_lowercase()));
        }
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
pub struct TextGroup {
    top: u32,
    start: u32,
    end: u32,
    text: String,
}

impl TextGroup {
    fn center(&self) -> u32 {
        self.start + (self.end - self.start) / 2
    }

    fn trim(&mut self) {
        self.text = self.text.trim().to_owned();
    }
}

impl<'a> From<Div<'a>> for TextGroup {
    fn from(value: Div<'a>) -> Self {
        TextGroup {
            top: value.top,
            start: value.left,
            end: value.left,
            text: value.text.trim_start().to_owned(),
        }
    }
}

impl<'a> AddAssign<Div<'a>> for TextGroup {
    fn add_assign(&mut self, mut rhs: Div<'a>) {
        self.end = rhs.left;
        while rhs.text.ends_with("  ") {
            rhs.text = rhs.text.trim_end_matches(' ');
        }
        if self.text.ends_with(' ') {
            self.text += rhs.text.trim_start();
        } else {
            self.text += rhs.text;
        }
    }
}
