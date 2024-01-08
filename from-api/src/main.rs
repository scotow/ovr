use std::env;
use itertools::chain;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::Value;

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
            ).map(Value::String)
                .collect()
        )
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let week = reqwest::get(env::args().nth(1).unwrap())
        .await
        .unwrap()
        .json::<Vec<Day>>()
        .await
        .unwrap();

    let form = Form::new()
        .part("week",
              Part::bytes(serde_json::to_vec(
                  &week.into_iter()
                      .map(|d| d.into_ovr_json())
                      .collect::<Value>()
              )
                  .unwrap())
                  .file_name("week.json")
                  .mime_str("application/json")
                  .unwrap()
    );

    let client = reqwest::Client::new();
    println!("{}", client.post(env::args().nth(2).unwrap())
        .multipart(form)
        .send()
        .await
        .unwrap().text().await.unwrap());
}
