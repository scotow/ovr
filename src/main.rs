use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{FromRef, Query, State},
    http::{header, HeaderValue},
    middleware::map_response,
    response::{IntoResponse, Response},
    routing::get,
    Router, Server,
};
use http_negotiator::{ContentTypeNegotiation, Negotiation, Negotiator};
use tokio::sync::RwLock;

use crate::{
    catalogue::Catalogue,
    error::Error,
    response::{ApiResponse, QueryFormat, ResponseType},
};

mod catalogue;
mod day;
mod error;
mod response;
mod week;

#[derive(FromRef, Clone)]
struct AppState {
    catalogue: Arc<RwLock<Catalogue>>,
    pub negotiator: Arc<Negotiator<ContentTypeNegotiation, ResponseType>>,
}

#[tokio::main]
async fn main() {
    let mut catalogue = Catalogue::new();
    for pdf in [
        include_bytes!("../S12.pdf").as_slice(),
        include_bytes!("../S19-2023.pdf").as_slice(),
        include_bytes!("../S20-2023.pdf").as_slice(),
        include_bytes!("../S23-2023.pdf").as_slice(),
    ] {
        let days = week::parse_pdf(pdf).unwrap();
        dbg!(catalogue.insert(days));
    }
    dbg!(&catalogue);

    Server::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080))
        .http1_title_case_headers(true)
        .serve(
            Router::new()
                .route("/today", get(today_handler))
                .route("/next", get(next_handler))
                .with_state(AppState {
                    catalogue: Arc::new(RwLock::new(catalogue)),
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

async fn today_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    Query(format): Query<QueryFormat>,
    Negotiation(_, response_type): Negotiation<ContentTypeNegotiation, ResponseType>,
) -> impl IntoResponse {
    ApiResponse(
        response_type,
        format,
        catalogue.read().await.today().ok_or(Error::NoMealToday),
    )
}

async fn next_handler(
    State(catalogue): State<Arc<RwLock<Catalogue>>>,
    Query(format): Query<QueryFormat>,
    Negotiation(_, response_type): Negotiation<ContentTypeNegotiation, ResponseType>,
) -> impl IntoResponse {
    ApiResponse(
        response_type,
        format,
        catalogue.read().await.next().ok_or(Error::NoNextMeal),
    )
}
