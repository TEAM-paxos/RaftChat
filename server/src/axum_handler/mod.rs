use axum::response::Html;
use axum::{Extension, Json};
use std::sync::Arc;
use crate::Config;
use axum::{
    body::Body,
    routing::get,
    Router,
};
use serde_json::{Value, json};


pub async fn handler() -> Html<&'static str> {
    // `std::include_str` macro can be used to include an utf-8 file as `&'static str` in compile
    // time. This method is relative to current `main.rs` file.
    Html(include_str!("../../../client/index.html"))
}

pub async fn get_info(Extension(config): Extension<Arc<Config>>) -> Json<Config>{
    Json(config.as_ref().clone())
}