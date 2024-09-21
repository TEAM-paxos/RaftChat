use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, Receiver, Sender};
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use raft;
use futures_util::stream::SplitSink;
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::Message,
};
use tokio::net::TcpStream;

mod events;
mod axum_handler;
mod data_model;


#[tokio::main]
async fn main () {
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

    // writer task 
    let (writer_tx, writer_rx) : 
        (Sender<data_model::msg::ClientMsg>, Receiver<data_model::msg::ClientMsg>) = mpsc::channel(15);

    let writer = events::task::Writer::new();
    writer.start(writer_rx, raft_tx).await;

    // publisher task
    let (pub_tx, pub_rx) : 
        (Sender<SplitSink<WebSocketStream<TcpStream>, Message>>, 
            Receiver<SplitSink<WebSocketStream<TcpStream>, Message>>) = mpsc::channel(15);
    let publisher = events::task::Publisher::new();
    publisher.start(commit_rx, pub_rx).await;


    // websocket server
    let server = TcpListener::bind("127.0.0.1:9001").await;
    let listener = server.expect("failed to bind");

    while let Ok((stream, addr)) = listener.accept().await {
        let w_tx = writer_tx.clone();
        let p_tx = pub_tx.clone();
        tokio::spawn (async move{
            events::handler::client_handler(stream, addr, w_tx, p_tx).await;
        });
    }
}

