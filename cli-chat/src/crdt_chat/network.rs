use std::net::SocketAddr;
use bebop::SubRecord;
use crate::generated::Message::Message;

use bebop::Record;
use http_body_util::Full;
use hyper::body::{Body, Bytes};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use hyper::{Method, StatusCode};
use http_body_util::{combinators::BoxBody, BodyExt};


pub async fn run_sender(ip: &str, port:u16, dir:&str, msg : Message<'_>) ->  Result<(),Box<dyn std::error::Error + Send + Sync>> {
// This is where we will setup our HTTP client requests.
    // Parse our URL...
    let destination = format!("http://{}:{}/{}", ip, port, dir);
    //println!("Dest: {}", destination);
    let url = destination.parse::<hyper::Uri>()?;

    // Get the host and the port
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);

    let address = format!("{}:{}", host, port);

    // Open a TCP connection to the remote host
    let stream = TcpStream::connect(address).await?;

    // Use an adapter to access something implementing `tokio::io` traits as if they implement
    // `hyper::rt` IO traits.
    let io = TokioIo::new(stream);

    // Create the Hyper client
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    // Spawn a task to poll the connection, driving the HTTP state
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    //make msg
    let mut buf = Vec::with_capacity(Message::MIN_SERIALIZED_SIZE);
    msg.serialize(&mut buf).unwrap();

    // The authority of our URL will be the hostname of the httpbin remote
    let authority = url.authority().unwrap().clone();

    // Create an HTTP request with an empty body and a HOST header
    let req = Request::builder()
        .uri(url)
        .method("POST")
        .header(hyper::header::HOST, authority.as_str())
        .body(Full::<Bytes>::new(buf.into()))?;

    // Await the response...
    let _res = sender.send_request(req).await?;

    //println!("Response status: {}", _res.status());

    Ok(())
}

pub async fn run_receiver(port : u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //println!("run receiver {}", port);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(reply))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn reply(
    req: Request<hyper::body::Incoming>
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error>{
    match (req.method(), req.uri().path()){
        (&Method::GET, "/echo") => Ok(Response::new(full(
            "Try posting data to /echo",
        ))),
        (&Method::POST, "/echo") => {
            // we'll be back
            let upper = req.body().size_hint().upper().unwrap_or(u64::MAX);
            if upper > 1024 * 64 {
                let mut resp = Response::new(full("Body too big"));
                *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
                return Ok(resp);
            }
            
            let whole_body = req.collect().await?.to_bytes();
            let res = Message::deserialize(&whole_body).unwrap();

            
            print!("{}", res.msg);

            Ok(Response::new(full(
                "got it",
            )))
        },
        _ => {
            let mut not_found = Response::new(empty());
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

// We create some utility functions to make Empty and Full bodies
// fit our broadened Response body type.
fn empty() -> BoxBody<Bytes, hyper::Error> {
    http_body_util::Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

#[cfg(test)] 
mod tests{
    use bebop::{Record, SubRecord};
    use crate::generated::Message::Message;
    
    #[test]    
    fn it_works(){
        let mut buf = Vec::with_capacity(Message::MIN_SERIALIZED_SIZE);
        let temp = Message{
            from : "hi",
            to : "hello",
            msg : "hi hello"
        };

        temp.serialize(&mut buf).unwrap();
        let s2 = Message::deserialize(&buf).unwrap();
        assert_eq!(temp, s2);
    }
}