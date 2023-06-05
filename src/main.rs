use std::ops::AddAssign;
use itertools::Itertools;
use lopdf::Document;
use pdf_extract::HTMLOutput;
use regex::Regex;

const MIN_WORDS_PER_LINE: usize = 3;

fn main() {
    let document = Document::load_mem(include_bytes!("../S23-2023.pdf")).unwrap();
    let mut out_buffer = Vec::new();
    let mut parser = HTMLOutput::new(&mut out_buffer);
    pdf_extract::output_doc(&document, &mut parser).unwrap();

    let html = String::from_utf8(out_buffer).unwrap().replace("&nbsp;", " ");
    let div_regex = Regex::new(r#"<div style='(.+?)'>(.+?)</div>"#).unwrap();
    let top_regex = Regex::new(r#"top:\s?(\d+)(?:\.\d+)?px"#).unwrap();
    let left_regex = Regex::new(r#"left:\s?(\d+)(?:\.\d+)?px"#).unwrap();

    let mut groups = div_regex.captures_iter(&html)
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
            },
            None => {
                words.push(TextGroup::from(g))
            }
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

    let mut days = Vec::<Vec<TextGroup>>::with_capacity(5);
    for word in words {
        match days.iter_mut()
            .find(|ow| {
                ow.iter()
                    .any(|ow| ow.center().abs_diff(word.center()) < 30)
            }) {
            Some(day) => day.push(word),
            None => days.push(vec![word]),
        }
    }

    dbg!(&days);
}

#[derive(Debug)]
struct Div<'a> {
    top: u32,
    left: u32,
    text: &'a str,
}

#[derive(Debug)]
struct TextGroup {
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