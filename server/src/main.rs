use std::io::stdin;
use std::net::TcpListener;
use std::thread::spawn;
use tungstenite::accept;

/// A WebSocket echo server
fn main () {
    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    for stream in server.incoming() {
        spawn (move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            websocket.send(tungstenite::Message::Text("Hello from server!".into())).unwrap();
            loop {
                let msg = websocket.read().unwrap();

                print!("> {}\n", msg);

                let mut buffer = String::new();
                stdin().read_line(&mut buffer).unwrap();
                
                websocket.send(tungstenite::Message::Text(buffer)).unwrap();
            }
        });
    }
}