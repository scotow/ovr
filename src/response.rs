use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use either::Either;
use http_negotiator::AsNegotiationStr;
use serde::{Deserialize, Serialize};

use crate::error::Error;

pub struct ApiResponse<T> {
    pub response_type: ResponseType,
    pub human: bool,
    pub data: Result<T, Error>,
}

impl<T: Serialize + TextRepresentable> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        (
            match &self.data {
                Ok(_) => StatusCode::OK,
                Err(err) => err.status_code(),
            },
            match self.response_type {
                ResponseType::Json => {
                    #[derive(Serialize)]
                    struct JsonResponse<T> {
                        success: bool,
                        #[serde(flatten)]
                        data: T,
                    }
                    Json(JsonResponse {
                        success: self.data.is_ok(),
                        data: match self.data {
                            Ok(data) => serde_json::to_value(data),
                            Err(err) => serde_json::to_value(err),
                        }
                        .expect("serialization failed"),
                    })
                    .into_response()
                }
                ResponseType::Text => match self.data {
                    Ok(data) => data.as_plain_text(self.human),
                    Err(err) => err.as_plain_text(self.human),
                }
                .into_response(),
                ResponseType::Html => Html(match self.data {
                    Ok(data) => data.as_html(),
                    Err(err) => err.as_html(),
                })
                .into_response(),
            },
        )
            .into_response()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ResponseType {
    Json,
    Text,
    Html,
}

impl AsNegotiationStr for ResponseType {
    fn as_str(&self) -> &str {
        match self {
            ResponseType::Json => "application/json",
            ResponseType::Text => "text/plain",
            ResponseType::Html => "text/html",
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct QueryFormat {
    #[serde(default)]
    pub human: bool,
}

pub trait TextRepresentable {
    fn as_plain_text(&self, _human: bool) -> String {
        String::new()
    }

    fn as_html(&self) -> String {
        String::new()
    }
}

impl<L: TextRepresentable, R: TextRepresentable> TextRepresentable for Either<L, R> {
    fn as_plain_text(&self, human: bool) -> String {
        match self {
            Either::Left(lhs) => lhs.as_plain_text(human),
            Either::Right(rhs) => rhs.as_plain_text(human),
        }
    }

    fn as_html(&self) -> String {
        match self {
            Either::Left(lhs) => lhs.as_html(),
            Either::Right(rhs) => rhs.as_html(),
        }
    }
}
