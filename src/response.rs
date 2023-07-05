use std::sync::Arc;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Query},
    http::{request::Parts, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use either::Either;
use http_negotiator::{AsNegotiationStr, ContentTypeNegotiation, Negotiation, Negotiator};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::Error;

pub struct ApiResponse<T> {
    pub response_type: ResponseType,
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
                ResponseType::Json(human) => {
                    #[derive(Serialize)]
                    struct JsonResponse<T> {
                        success: bool,
                        #[serde(flatten)]
                        data: T,
                    }
                    Json(JsonResponse {
                        success: self.data.is_ok(),
                        data: if human {
                            Ok(json!({
                                "message": match self.data {
                                    Ok(data) => data.as_plain_text(human),
                                    Err(err) => err.as_plain_text(human),
                                }
                            }))
                        } else {
                            match self.data {
                                Ok(data) => serde_json::to_value(data),
                                Err(err) => serde_json::to_value(err),
                            }
                        }
                        .expect("serialization failed"),
                    })
                    .into_response()
                }
                ResponseType::Text(human) => match self.data {
                    Ok(data) => data.as_plain_text(human),
                    Err(err) => err.as_plain_text(human),
                }
                .into_response(),
                ResponseType::Html(scale, center) => Html(
                    include_str!("wrapper.html")
                        .replacen("$SCALE", &scale.to_string(), 1)
                        .replacen("$CENTER", if center { "center" } else { "flex-start" }, 1)
                        .replacen(
                            "$BODY",
                            &match self.data {
                                Ok(data) => data.as_html(),
                                Err(err) => err.as_html(),
                            },
                            1,
                        ),
                )
                .into_response(),
            },
        )
            .into_response()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ResponseTypeRaw {
    Json,
    Text,
    Html,
}

impl AsNegotiationStr for ResponseTypeRaw {
    fn as_str(&self) -> &str {
        match self {
            ResponseTypeRaw::Json => "application/json",
            ResponseTypeRaw::Text => "text/plain",
            ResponseTypeRaw::Html => "text/html",
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ResponseType {
    Json(bool),
    Text(bool),
    Html(f32, bool),
}

#[async_trait]
impl<S> FromRequestParts<S> for ResponseType
where
    S: Send + Sync,
    Arc<Negotiator<ContentTypeNegotiation, ResponseTypeRaw>>: FromRef<S>,
{
    type Rejection = ApiResponse<()>;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Negotiation(_, raw) =
            Negotiation::<ContentTypeNegotiation, ResponseTypeRaw>::from_request_parts(
                parts, state,
            )
            .await
            .map_err(|_| ApiResponse {
                response_type: ResponseType::Json(false),
                data: Err(Error::ContentNegotiation),
            })?;

        #[derive(Deserialize)]
        struct QueryFormat {
            #[serde(default)]
            human: bool,
            #[serde(default = "QueryFormat::default_scale")]
            scale: f32,
            #[serde(default)]
            center: bool,
        }

        impl QueryFormat {
            const fn default_scale() -> f32 {
                1.0
            }
        }

        let Query(format) = Query::<QueryFormat>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiResponse {
                response_type: ResponseType::Json(false),
                data: Err(Error::InvalidFormatParameter),
            })?;

        Ok(match raw {
            ResponseTypeRaw::Json => ResponseType::Json(format.human),
            ResponseTypeRaw::Text => ResponseType::Text(format.human),
            ResponseTypeRaw::Html => ResponseType::Html(format.scale, format.center),
        })
    }
}

pub trait TextRepresentable {
    fn as_plain_text(&self, _human: bool) -> String {
        String::new()
    }

    fn as_html(&self) -> String {
        String::new()
    }
}

impl TextRepresentable for () {}

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
