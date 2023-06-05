use axum::http::StatusCode;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use thiserror::Error as ThisError;

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
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Error::InvalidBody => StatusCode::BAD_REQUEST,
            Error::InvalidPdf => StatusCode::BAD_REQUEST,
            Error::NoMealToday => StatusCode::NOT_FOUND,
            Error::NoNextMeal => StatusCode::NOT_FOUND,
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
