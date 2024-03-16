use hyper::Error;
use std::{env, io::{self, stdin,Write}};
mod crdt_chat;
pub mod generated;

async fn looping(){
    let ip = "0.0.0.0";
    let mut port: String = String::new();
    print!("Put opponent's port: ");
    io::stdout().flush().unwrap();

    //stdin().read_to_string(&mut buf).await;
    stdin().read_line(&mut port).unwrap();

    let port: u16 = port.trim().parse().unwrap();

    loop {
        let mut buf: String = String::new();
        stdin().read_line(&mut buf).unwrap();

        let send = tokio::spawn(
            async move{
                crdt_chat::send_msg(ip,port,  buf).await.unwrap();
            }
        );

        send.await.unwrap();
    }   
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let port: u16 = env::args().nth(1).unwrap().trim().parse().unwrap();
    //let port: u16 = 3000;

    let receiver = tokio::spawn(
        crdt_chat::run_receiver(port)
    );

    let looper = tokio::spawn(
        looping()    
    );

    let _out = receiver.await.unwrap();
    let _out2 = looper.await.unwrap();
    Ok(())
}

// use std::io::{stdout, Result};
// use crossterm::{
//     event::{self, KeyCode, KeyEventKind},
//     terminal::{
//         disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
//         LeaveAlternateScreen,
//     },
//     ExecutableCommand,
// };

// use ratatui::{
//     prelude::{CrosstermBackend, Stylize, Terminal},
//     widgets::Paragraph,
// };

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

// #[cfg(test)] 
// mod tests{

//     #[test]    
    
//     fn it_works(){
       
//     }
// }