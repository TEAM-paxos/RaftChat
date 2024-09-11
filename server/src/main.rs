use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::Message,
};
use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use tokio::io::{self, AsyncBufReadExt, BufReader};

async fn handle_client(stream: TcpStream, addr: std::net::SocketAddr) {

    let ws_steam = tokio_tungstenite::accept_async(stream)
                                                .await
                                                .expect("Error during the websocket handshake occurred");

    let(mut write_stream, mut read_stream) = ws_steam.split();      

    write_stream.send(Message::Text("Connected to server!".into())).await.unwrap();

    let h1 = tokio::spawn(async move {
        read_task(read_stream).await;
    });

    let h2 = tokio::spawn(async move {
        write_task(write_stream).await;
    });
    
    h1.await.unwrap();
    h2.await.unwrap();
}

async fn read_task(mut read_stream: SplitStream<WebSocketStream<TcpStream>> ) {
    println!("Reading messages from client");
    while let Some(msg) = read_stream.next().await {
        let msg = msg.unwrap();
        print!("> {}\n", msg);
    }
}

async fn write_task(mut write_stream: SplitSink<WebSocketStream<TcpStream>, Message>){
    loop {
        let stdin = io::stdin();  // standard input
        let mut reader = BufReader::new(stdin);  // buffer the input
        let mut line = String::new();

        line.clear();
        let bytes_size = reader.read_line(&mut line).await.unwrap();
        if bytes_size == 0 {
            break; // EOF
        }
        
        write_stream.send(Message::Text(line)).await.unwrap();
    }
}

#[tokio::main]
async fn main () {
    let server = TcpListener::bind("127.0.0.1:9001").await;
    let listener = server.expect("failed to bind");

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn (async move{
            handle_client(stream, addr).await;
        });
    }
}