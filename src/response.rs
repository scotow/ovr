use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
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
                    Ok(data) => data.as_text(self.human),
                    Err(err) => err.to_string(),
                }
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
}

impl AsNegotiationStr for ResponseType {
    fn as_str(&self) -> &str {
        match self {
            ResponseType::Json => "application/json",
            ResponseType::Text => "text/plain",
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct QueryFormat {
    #[serde(default)]
    pub human: bool,
}

pub trait TextRepresentable {
    fn as_text(&self, _human: bool) -> String {
        String::new()
    }
}
