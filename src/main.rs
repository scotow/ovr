use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::{
    body::{Body, Bytes},
    extract::{FromRef, FromRequest, Multipart, Query, State},
    http::{header, HeaderValue, Request},
    middleware::map_response,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router, Server,
};
use http_negotiator::{ContentTypeNegotiation, Negotiation, Negotiator};
use tokio::sync::RwLock;

use crate::{
    catalogue::{Catalogue, CatalogueUpdate},
    error::Error,
    response::{ApiResponse, QueryFormat, ResponseType},
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
    pub negotiator: Arc<Negotiator<ContentTypeNegotiation, ResponseType>>,
}

#[tokio::main]
async fn main() {
    Server::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080))
        .http1_title_case_headers(true)
        .serve(
            Router::new()
                .route("/", get(catalogue_handler).post(upload_handler))
                .route("/upload", post(upload_handler))
                .route("/today", get(today_handler))
                .route("/next", get(next_handler))
                .with_state(AppState {
                    catalogue: Arc::new(RwLock::new(Catalogue::new())),
                    negotiator: Arc::new(
                        Negotiator::new([ResponseType::Json, ResponseType::Text])
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
}

async fn catalogue_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    Negotiation(_, response_type): Negotiation<ContentTypeNegotiation, ResponseType>,
) -> impl IntoResponse {
    ApiResponse {
        response_type,
        human: false,
        data: Ok(catalogue.read().await.clone()),
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
                let days = week::parse_pdf(&data).map_err(|_| Error::InvalidPdf)?;
                updates += catalogue_lock.insert(days);
            }
        } else {
            let data = Bytes::from_request(request, &())
                .await
                .map_err(|_| Error::InvalidBody)?;
            let days = week::parse_pdf(&data).map_err(|_| Error::InvalidPdf)?;
            updates += catalogue_lock.insert(days);
        }
        Ok(updates)
    }

    ApiResponse {
        response_type: ResponseType::Json,
        human: false,
        data: process(catalogue, request).await,
    }
}

async fn today_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    Query(format): Query<QueryFormat>,
    Negotiation(_, response_type): Negotiation<ContentTypeNegotiation, ResponseType>,
) -> impl IntoResponse {
    ApiResponse {
        response_type,
        human: format.human,
        data: catalogue.read().await.today().ok_or(Error::NoMealToday),
    }
}

async fn next_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    Query(format): Query<QueryFormat>,
    Negotiation(_, response_type): Negotiation<ContentTypeNegotiation, ResponseType>,
) -> impl IntoResponse {
    ApiResponse {
        response_type,
        human: format.human,
        data: catalogue.read().await.next().ok_or(Error::NoNextMeal),
    }
}
