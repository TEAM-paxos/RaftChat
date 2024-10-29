use axum::Extension;
use axum::{routing::get, Router};
use futures_util::stream::SplitSink;
use log::info;
use raft;
use std::collections::HashMap;
use std::env;
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

#[derive(Clone, serde::Serialize, Debug)]
struct Config {
    peers: Vec<String>,
    port: u16,
    socket_port: u16,
}

#[tokio::main]
async fn main() {
    // log setting
    log4rs::init_file("../config/log4rs.yaml", Default::default()).ok();

    // config setting
    dotenv::from_path("../config/config.env").unwrap();

    let peers: Vec<String> = env::var("PEER")
        .unwrap()
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let port: u16 = env::var("WEB_PORT")
        .ok()
        .and_then(|val| val.parse::<u16>().ok())
        .unwrap_or(3001);

    let socket_port: u16 = env::var("SOCKET_PORT")
        .ok()
        .and_then(|val| val.parse::<u16>().ok())
        .unwrap_or(9001);

    let config = Arc::new(Config {
        peers: peers.clone(),
        port,
        socket_port,
    });

    info!("{:?}", config);

    // raft server
    let (commit_rx, raft_tx) = raft::Raft::new(1, peers);

    // axum server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let app = Router::new()
        .route("/", get(axum_handler::handler))
        .nest_service("/static", ServeDir::new("../client/static"))
        .route("/get_info", get(axum_handler::get_info))
        .layer(Extension(config));

    let listener = TcpListener::bind(&addr).await.unwrap();

    tokio::spawn(async move {
        info!("AXUM listening on {}", addr);
        axum::serve(listener, app).await.unwrap();
    });

    // writer and publisher set up
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
    let addr = SocketAddr::from(([0, 0, 0, 0], socket_port));
    let server = TcpListener::bind(addr).await;
    let listener = server.expect("failed to bind");

    while let Ok((stream, addr)) = listener.accept().await {
        let w_tx = writer_tx.clone();
        let p_tx = pub_tx.clone();
        tokio::spawn(async move {
            events::handler::client_handler(stream, addr, w_tx, p_tx).await;
        });
    }
}
