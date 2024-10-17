use axum::{routing::get, Router};
use database::DataBase;
use futures_util::stream::SplitSink;
use raft;
use std::net::SocketAddr;
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
    run_axum().await;
    let (writer_tx, pub_tx) = run_tasks(raft::Raft {}).await;

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

// axum serves html and static files to client.
async fn run_axum() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let app = Router::new()
        .route("/", get(axum_handler::handler))
        .nest_service("/static", ServeDir::new("../client/static"));

    let listener = TcpListener::bind(&addr).await.unwrap();

    tokio::spawn(async move {
        println!("AXUM listening on {}", addr);
        axum::serve(listener, app).await.unwrap();
    });
}

// - make channel from Database like raft or sync db
// - start server read write tasks
async fn run_tasks<T: DataBase>(
    db: T,
) -> (
    Sender<data_model::msg::ClientMsg>,
    Sender<SplitSink<WebSocketStream<TcpStream>, Message>>,
) {
    let (commit_rx, database_tx) = db.make_channel(1, vec![1, 2, 3]);

    // writer task
    let (writer_tx, writer_rx): (
        Sender<data_model::msg::ClientMsg>,
        Receiver<data_model::msg::ClientMsg>,
    ) = mpsc::channel(15);

    let writer = events::task::Writer::new();
    writer.start(writer_rx, database_tx).await;

    // publisher task
    let (pub_tx, pub_rx): (
        Sender<SplitSink<WebSocketStream<TcpStream>, Message>>,
        Receiver<SplitSink<WebSocketStream<TcpStream>, Message>>,
    ) = mpsc::channel(15);
    let publisher = events::task::Publisher::new();
    publisher.start(commit_rx, pub_rx).await;

    return (writer_tx, pub_tx);
}
