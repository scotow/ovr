use std::{env, ops::Add, time::SystemTime};

use itertools::chain;
use reqwest::{
    multipart::{Form, Part},
    StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use time::{ext::NumericalStdDuration, macros::format_description, OffsetDateTime, Weekday};

#[derive(Deserialize, Debug)]
struct Day {
    date: String,
    #[serde(rename = "starters_without_usual")]
    starters: Vec<String>,
    mains: Vec<String>,
    sides: Vec<String>,
    #[serde(rename = "cheeses_without_usual")]
    cheeses: Vec<String>,
    #[serde(rename = "desserts_without_usual")]
    desserts: Vec<String>,
}

impl Day {
    fn into_ovr_json(self) -> Value {
        Value::Array(
            chain!(
                Some(self.date),
                self.starters,
                self.mains,
                self.sides,
                self.cheeses,
                self.desserts
            )
            .map(Value::String)
            .collect(),
        )
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let format = format_description!("[year]-[month]-[day]");

    let today = OffsetDateTime::from(SystemTime::now()).date();
    let mut week_start = if today.weekday() == Weekday::Monday {
        today
    } else {
        today.prev_occurrence(Weekday::Monday)
    };

    let mut allowed_errors = 5;
    let mut days = Vec::new();
    while allowed_errors > 0 {
        for d in 0..5 {
            match fetch::<Day>(&format!(
                "day/{}",
                week_start.add(d.std_days()).format(&format).unwrap()
            ))
            .await
            {
                Some(day) => days.push(day),
                None => allowed_errors -= 1,
            }
        }
        week_start = week_start.next_occurrence(Weekday::Monday);
    }

    let form = Form::new().part(
        "days",
        Part::bytes(
            serde_json::to_vec(
                &days
                    .into_iter()
                    .map(|d| d.into_ovr_json())
                    .collect::<Value>(),
            )
            .unwrap(),
        )
        .file_name("days.json")
        .mime_str("application/json")
        .unwrap(),
    );

    let client = reqwest::Client::new();
    println!(
        "{}",
        client
            .post(env::args().nth(2).unwrap())
            .multipart(form)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    );
}

async fn fetch<T: DeserializeOwned>(uri: &str) -> Option<T> {
    let resp = reqwest::get(format!("{}/api/{uri}", env::args().nth(1).unwrap()))
        .await
        .ok()?;
    if resp.status() != StatusCode::OK {
        return None;
    }

    resp.json().await.ok()
}
