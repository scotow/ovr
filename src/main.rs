use std::{
    env::args,
    fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::{
    body::{Body, Bytes},
    extract::{FromRef, FromRequest, Multipart, Path, Query, State},
    http::{header, HeaderValue, Request},
    middleware::map_response,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router, Server,
};
use either::Either;
use http_negotiator::{ContentTypeNegotiation, Negotiator};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::{
    catalogue::{Catalogue, CatalogueUpdate},
    day::Day,
    error::Error,
    response::{ApiResponse, ResponseType, ResponseTypeRaw, TextRepresentable},
    utils::parse_date,
};

mod catalogue;
mod day;
mod error;
mod response;
mod utils;
mod week;

#[derive(FromRef, Clone)]
struct AppState {
    catalogue: Arc<RwLock<Catalogue>>,
    negotiator: Arc<Negotiator<ContentTypeNegotiation, ResponseTypeRaw>>,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut catalogue = Catalogue::new();
    let mut updates = CatalogueUpdate::default();
    for doc in args().skip(1) {
        let week = week::parse_pdf(&fs::read(&doc).map_err(|err| err.to_string())?)
            .map_err(|err| err.to_string())?;
        updates += catalogue.insert(week);
    }
    if !updates.is_empty() {
        println!("{}", updates.as_plain_text(false));
    }

    Server::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080))
        .http1_title_case_headers(true)
        .serve(
            Router::new()
                .route("/", get(index_handler).post(upload_handler))
                .route("/upload", post(upload_handler))
                .route("/today", get(today_handler))
                .route("/next", get(next_handler))
                .route("/find", get(find_handler))
                .route("/weeks/:week", get(week_handler))
                .route("/days/:day", get(day_handler))
                .route("/calendar.ics", get(ics_handler))
                .with_state(AppState {
                    catalogue: Arc::new(RwLock::new(catalogue)),
                    negotiator: Arc::new(
                        Negotiator::new([
                            ResponseTypeRaw::Json,
                            ResponseTypeRaw::Text,
                            ResponseTypeRaw::Html,
                        ])
                        .expect("invalid content-type negotiator"),
                    ),
                })
                .layer(map_response(|mut resp: Response| async {
                    resp.headers_mut().insert(
                        header::SERVER,
                        HeaderValue::from_static(concat!("OVR v", env!("CARGO_PKG_VERSION"))),
                    );
                    resp
                }))
                .into_make_service(),
        )
        .await
        .unwrap_err();

    Ok(())
}

async fn index_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    response_type: ResponseType,
) -> impl IntoResponse {
    ApiResponse {
        response_type,
        data: Ok(if matches!(response_type, ResponseType::Html) {
            Either::Left(catalogue.read().await.weeks())
        } else {
            Either::Right(catalogue.read().await.clone())
        }),
    }
}

async fn upload_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    request: Request<Body>,
) -> impl IntoResponse {
    async fn process(
        catalogue: Arc<RwLock<Catalogue>>,
        request: Request<Body>,
    ) -> Result<CatalogueUpdate, Error> {
        let mut catalogue_lock = catalogue.write().await;
        let mut updates = CatalogueUpdate::default();
        if request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .is_some_and(|h| h.starts_with("multipart/form-data"))
        {
            let mut multipart = Multipart::from_request(request, &())
                .await
                .map_err(|_| Error::InvalidBody)?;
            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|_| Error::InvalidBody)?
            {
                let data = field.bytes().await.map_err(|_| Error::InvalidBody)?;
                let days = week::parse_pdf(&data)?;
                updates += catalogue_lock.insert(days);
            }
        } else {
            let data = Bytes::from_request(request, &())
                .await
                .map_err(|_| Error::InvalidBody)?;
            let days = week::parse_pdf(&data)?;
            updates += catalogue_lock.insert(days);
        }
        Ok(updates)
    }

    ApiResponse {
        response_type: ResponseType::Json(false),
        data: process(catalogue, request).await,
    }
}

async fn today_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    response_type: ResponseType,
) -> impl IntoResponse {
    ApiResponse {
        response_type,
        data: catalogue.read().await.today().ok_or(Error::NoMealToday),
    }
}

async fn next_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    response_type: ResponseType,
) -> impl IntoResponse {
    ApiResponse {
        response_type,
        data: catalogue.read().await.next().ok_or(Error::NoNextMeal),
    }
}

#[derive(Deserialize)]
struct FindQuery {
    dish: String,
}

async fn find_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    response_type: ResponseType,
    Query(query): Query<FindQuery>,
) -> impl IntoResponse {
    ApiResponse {
        response_type,
        data: catalogue
            .read()
            .await
            .find_dish_next(query.dish.split(',').map(|d| d.to_owned()).collect())
            .ok_or(Error::NoNextMeal),
    }
}

async fn week_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    response_type: ResponseType,
    Path(week): Path<String>,
) -> impl IntoResponse {
    async fn process(catalogue: Arc<RwLock<Catalogue>>, week: String) -> Result<Catalogue, Error> {
        let (year, week) = week.split_once('-').ok_or(Error::InvalidWeek)?;
        catalogue.read().await.week(
            year.parse().map_err(|_| Error::InvalidWeek)?,
            week.parse().map_err(|_| Error::InvalidWeek)?,
        )
    }
    ApiResponse {
        response_type,
        data: process(catalogue, week).await,
    }
}

async fn day_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    response_type: ResponseType,
    Path(date): Path<String>,
) -> impl IntoResponse {
    async fn process(catalogue: Arc<RwLock<Catalogue>>, date: String) -> Result<Day, Error> {
        let date = parse_date(&date).ok_or(Error::InvalidDay)?;
        catalogue.read().await.day(date)
    }
    ApiResponse {
        response_type,
        data: process(catalogue, date).await,
    }
}

async fn ics_handler(State(catalogue): State<Arc<RwLock<Catalogue>>>) -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/calendar"),
        )],
        catalogue.read().await.ics(),
    )
}
