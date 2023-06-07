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
    #[error("week not found")]
    WeekNotFound,
    #[error("day not found")]
    DayNotFound,
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
            Error::WeekNotFound => StatusCode::NOT_FOUND,
            Error::DayNotFound => StatusCode::NOT_FOUND,
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
            Error::InvalidWeek => "Format de semaine incorrect.".to_owned(),
            Error::InvalidDay => "Format de date incorrect.".to_owned(),
            Error::WeekNotFound => "Aucun menu trouvé pour cette semaine.".to_owned(),
            Error::DayNotFound => "Aucun menu trouvé pour ce jour.".to_owned(),
            _ => self.to_string(),
        }
    }

    fn as_html(&self) -> String {
        format!("<pre>{}</pre>", self.as_plain_text(false))
    }
}
