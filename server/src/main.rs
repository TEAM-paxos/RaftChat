use tokio::net::TcpListener;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

pub mod events;
pub mod axum_handler;


#[tokio::main]
async fn main () {
    
    // axum server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let app = Router::new()
                            .route("/", get(axum_handler::handler))
                            .nest_service("/static", ServeDir::new("../../client/static"));

    let listener = TcpListener::bind(&addr).await.unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
   
    println!("AXUM listening on {}", addr);

    // websocket server
    let server = TcpListener::bind("127.0.0.1:9001").await;
    let listener = server.expect("failed to bind");

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn (async move{
            events::handler::handle_client(stream, addr).await;
        });
    }
}

