
mod CrdtChat;

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use ratatui::{
    prelude::{CrosstermBackend, Stylize, Terminal},
    widgets::Paragraph,
};
use hyper::body::Buf;
use http_body_util::{BodyExt, Empty, Full};
use hyper::Request;
use hyper::body::Bytes;
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error + Send + Sync>> {
    // This is where we will setup our HTTP client requests.
    // Parse our URL...
    let url = "http://127.0.0.1:3000/echo".parse::<hyper::Uri>()?;

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
    let temp = CrdtChat::serialize::Msg{ 
        From: "from".to_string(), 
        To: "to".to_string(),
        Content: "hi".to_string() };
    
    let binding = CrdtChat::serialize::serialize_msg(&temp);
    let mut bytes = binding.as_slice();

    // The authority of our URL will be the hostname of the httpbin remote
    let authority = url.authority().unwrap().clone();
    println!("{:?}", bytes);
    // Create an HTTP request with an empty body and a HOST header
    let req = Request::builder()
        .uri(url)
        .method("POST")
        .header(hyper::header::HOST, authority.as_str())
        .body(Full::new(bytes.copy_to_bytes(bytes.len())))?;

    // Await the response...
    let mut res = sender.send_request(req).await?;

    println!("Response status: {}", res.status());
    Ok(())
}

// fn main() -> Result<()> {
//     stdout().execute(EnterAlternateScreen)?;
//     enable_raw_mode()?;
//     let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
//     terminal.clear()?;

//     // TODO main loop
//     loop {
//         terminal.draw(|frame| {
//             let area = frame.size();
//             frame.render_widget(
//                 Paragraph::new("Hello Ratatui! (press 'q' to quit)")
//                     .white()
//                     .on_blue(),
//                 area,
//             );
//         })?;
    
//         if event::poll(std::time::Duration::from_millis(16))? {
//             if let event::Event::Key(key) = event::read()? {
//                 if key.kind == KeyEventKind::Press
//                     && key.code == KeyCode::Char('q')
//                 {
//                     break;
//                 }
//             }
//         }
//     }

//     stdout().execute(LeaveAlternateScreen)?;
//     disable_raw_mode()?;
//     Ok(())
// }