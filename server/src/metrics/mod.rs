use std::sync::atomic::AtomicI64;

use axum::body::Body;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;

use std::sync::Arc;
use tokio::sync::Mutex;

// Metric
#[derive(Debug)]
pub struct Metrics {
    connections: Gauge<i64, AtomicI64>,
}

impl Metrics {
    pub fn inc_connections(&self) {
        self.connections.inc();
    }

    pub fn dec_connections(&self) {
        self.connections.dec();
    }
}

pub struct AppState {
    pub registry: Registry,
}

pub fn new() -> (Metrics, AppState) {
    let metrics = Metrics {
        connections: Gauge::default(),
    };

    let mut state = AppState {
        registry: Registry::default(),
    };

    state.registry.register(
        "connections",
        "Num of client connections",
        metrics.connections.clone(),
    );

    (metrics, state)
}

pub async fn metrics_handler(State(state): State<Arc<Mutex<AppState>>>) -> impl IntoResponse {
    let state = state.lock().await;
    let mut buffer = String::new();
    encode(&mut buffer, &state.registry).unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .header(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(Body::from(buffer))
        .unwrap()
}
