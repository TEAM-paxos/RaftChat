use axum::{routing::get, Router};
use futures_util::stream::SplitSink;
use raft;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tower_http::services::ServeDir;

mod axum_handler;
mod data_model;
mod events;

#[tokio::main]
async fn main() {
    // raft server
    let (commit_rx, raft_tx) = raft::Raft::new(1, vec![2, 3]);

    // axum server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let app = Router::new()
        .route("/", get(axum_handler::handler))
        .nest_service("/static", ServeDir::new("../client/static"));

    let listener = TcpListener::bind(&addr).await.unwrap();

    tokio::spawn(async move {
        println!("AXUM listening on {}", addr);
        axum::serve(listener, app).await.unwrap();
    });

    //
    let client_commit_idx: Arc<tokio::sync::Mutex<HashMap<String, u64>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    // writer task
    let (writer_tx, writer_rx): (
        Sender<(String, data_model::msg::ClientMsg)>,
        Receiver<(String, data_model::msg::ClientMsg)>,
    ) = mpsc::channel(15);

    let writer = events::task::Writer::new(client_commit_idx.clone());
    writer.start(writer_rx, raft_tx).await;

    // publisher task
    let (pub_tx, pub_rx): (
        Sender<(String, SplitSink<WebSocketStream<TcpStream>, Message>)>,
        Receiver<(String, SplitSink<WebSocketStream<TcpStream>, Message>)>,
    ) = mpsc::channel(15);
    let publisher = events::task::Publisher::new(Vec::new(), client_commit_idx.clone());
    publisher.start(commit_rx, pub_rx).await;

    // websocket server
    let server = TcpListener::bind("127.0.0.1:9001").await;
    let listener = server.expect("failed to bind");

    while let Ok((stream, addr)) = listener.accept().await {
        let w_tx = writer_tx.clone();
        let p_tx = pub_tx.clone();
        tokio::spawn(async move {
            events::handler::client_handler(stream, addr, w_tx, p_tx).await;
        });
    }
}
