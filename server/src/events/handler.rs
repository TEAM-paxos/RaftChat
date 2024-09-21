use crate::data_model::msg::ClientMsg;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use std::process;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::net::{unix::pipe::Sender, TcpStream};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

type Stream = SplitSink<WebSocketStream<TcpStream>, Message>;

pub async fn client_handler(
    stream: TcpStream,
    addr: std::net::SocketAddr,
    writer_tx: tokio::sync::mpsc::Sender<ClientMsg>,
    publisher_tx: tokio::sync::mpsc::Sender<Stream>,
) {
    println!("Incoming WebSocket connection from: {}", addr);

    let ws_steam = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");
    let (mut write_stream, read_stream) = ws_steam.split();

    write_stream
        .send(Message::Text("Connected to server!".into()))
        .await
        .unwrap();

    let h1 = tokio::spawn(async move {
        read_task(read_stream, writer_tx).await;
    });

    println!("Sending write_stream to publisher");
    publisher_tx.send(write_stream).await.unwrap();
    // let h2 = tokio::spawn(async move {
    //     write_task(write_stream).await;
    // });

    h1.await.unwrap();
    // h2.await.unwrap();
}

async fn read_task(
    mut read_stream: SplitStream<WebSocketStream<TcpStream>>,
    writer_tx: tokio::sync::mpsc::Sender<ClientMsg>,
) {
    println!("read_task started");

    while let Some(msg) = read_stream.next().await {
        let msg = msg.unwrap();
        match msg {
            Message::Text(text) => {
                let client_msg: ClientMsg = serde_json::from_str(&text).unwrap_or_else(|err| {
                    println!("{}", text);
                    eprintln!("->Error: {}", err);
                    process::exit(1);
                });

                writer_tx.send(client_msg).await.unwrap();
            }
            Message::Binary(data) => {
                let client_msg: ClientMsg = serde_json::from_slice(&data).unwrap();
                writer_tx.send(client_msg).await.unwrap();
            }
            Message::Close(_) => {
                println!("Client disconnected");
                break;
            }
            _ => {
                println!("Unsupported message type");
            }
        }
    }
}

async fn write_task(mut write_stream: SplitSink<WebSocketStream<TcpStream>, Message>) {
    loop {
        let stdin = io::stdin(); // standard input
        let mut reader = BufReader::new(stdin); // buffer the input
        let mut line = String::new();

        line.clear();
        let bytes_size = reader.read_line(&mut line).await.unwrap();
        if bytes_size == 0 {
            break; // EOF
        }

        write_stream.send(Message::Text(line)).await.unwrap();
    }
}
