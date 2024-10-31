use crate::Config;
use axum::response::Html;
use axum::{Extension, Json};
use std::sync::Arc;

pub async fn handler() -> Html<&'static str> {
    // `std::include_str` macro can be used to include an utf-8 file as `&'static str` in compile
    // time. This method is relative to current `main.rs` file.
    Html(include_str!("../../../client/index.html"))
}

pub async fn get_info(Extension(config): Extension<Config>) -> Json<Config> {
    Json(config.clone())
}
