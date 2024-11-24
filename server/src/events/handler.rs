use crate::data_model::msg::ClientMsg;
use futures_util::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use log::{error, info, warn};
use std::process;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

type Stream = SplitSink<WebSocketStream<TcpStream>, Message>;

pub async fn client_handler(
    stream: TcpStream,
    addr: std::net::SocketAddr,
    writer_tx: tokio::sync::mpsc::Sender<(String, ClientMsg)>,
    publisher_tx: tokio::sync::mpsc::Sender<(String, Stream)>,
) {
    info!("Incoming WebSocket connection from: {}", addr);

    let ws_steam = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    // Split websocket stream
    let (write_stream, read_stream) = ws_steam.split();

    let join_handle = tokio::spawn(async move {
        read_task(read_stream, writer_tx, addr.clone()).await;
    });

    // Send write_stream to publisher
    publisher_tx
        .send((addr.to_string(), write_stream))
        .await
        .unwrap();

    join_handle.await.unwrap();
}

async fn read_task(
    mut read_stream: SplitStream<WebSocketStream<TcpStream>>,
    writer_tx: tokio::sync::mpsc::Sender<(String, ClientMsg)>,
    addr: std::net::SocketAddr,
) {
    info!("read_task started");

    while let Some(msg) = read_stream.next().await {
        let msg = msg.unwrap_or(Message::Close(None));
        match msg {
            Message::Text(text) => {
                let client_msg: ClientMsg = serde_json::from_str(&text).unwrap_or_else(|err| {
                    error!("{}", text);
                    error!("{}", err);
                    process::exit(1);
                });

                writer_tx
                    .send((addr.to_string(), client_msg))
                    .await
                    .unwrap();
            }
            Message::Binary(data) => {
                let client_msg: ClientMsg = serde_json::from_slice(&data).unwrap();
                writer_tx
                    .send((addr.to_string(), client_msg))
                    .await
                    .unwrap();
            }
            Message::Close(_) => {
                info!("Client disconnected");
                break;
            }
            _ => {
                warn!("Unsupported message type");
            }
        }
    }
}
