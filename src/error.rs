use axum::http::StatusCode;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use thiserror::Error as ThisError;

use crate::response::TextRepresentable;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("invalid body")]
    InvalidBody,
    #[error("invalid pdf")]
    InvalidPdf,
    #[error("no meal found for today")]
    NoMealToday,
    #[error("no next meal found")]
    NoNextMeal,
    #[error("invalid week")]
    InvalidWeek,
    #[error("invalid day")]
    InvalidDay,
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Error::InvalidBody => StatusCode::BAD_REQUEST,
            Error::InvalidPdf => StatusCode::BAD_REQUEST,
            Error::NoMealToday => StatusCode::NOT_FOUND,
            Error::NoNextMeal => StatusCode::NOT_FOUND,
            Error::InvalidWeek => StatusCode::BAD_REQUEST,
            Error::InvalidDay => StatusCode::BAD_REQUEST,
        }
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Error", 3)?;
        state.serialize_field("error", &self.to_string())?;
        state.end()
    }
}

impl TextRepresentable for Error {
    fn as_plain_text(&self, _human: bool) -> String {
        match self {
            Error::NoMealToday => "Aucun repas de prévu pour aujourd'hui.".to_owned(),
            Error::NoNextMeal => "Aucun repas de prévu pour bientôt.".to_owned(),
            _ => self.to_string(),
        }
    }

    fn as_html(&self) -> String {
        format!("<pre>{}</pre>", self.as_plain_text(false))
    }
}
