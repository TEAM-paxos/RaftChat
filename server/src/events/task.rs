use crate::data_model::msg::ClientMsg;
use database::{Commit, RequestLog};
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio::time::{self, Duration};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

type Stream = SplitSink<WebSocketStream<TcpStream>, Message>;

// Writer task
// - Receives messages from the client_handler and forwards them to the Raft
// - If the buffer size is greater than 10, it sends the messages to the Raft
// - It also sends the messages to the Raft every 5 ms
pub struct Writer {
    buffer: Arc<Mutex<Vec<ClientMsg>>>,
}

// Publisher task
// - It preseves the stream that sended from the client_handler
// - Receives committed messages from the Raft and sends them to the clients
pub struct Publisher {
    buffer: Arc<Mutex<Vec<ClientMsg>>>,
    clients: Arc<tokio::sync::Mutex<Vec<Stream>>>,
}

impl Publisher {
    pub fn new() -> Self {
        Publisher {
            buffer: Arc::new(Mutex::new(Vec::new())),
            clients: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub async fn start(&self, mut commit_rx: Receiver<Commit>, mut pub_rx: Receiver<Stream>) {
        println!("Publisher started");

        // receive stream from handler and store it in the hashmap
        let clients = self.clients.clone();
        tokio::spawn(async move {
            let mut idx = 0;
            while let Some(stream) = pub_rx.recv().await {
                println!("Received a new client");
                clients.lock().await.push(stream);

                idx += 1;
            }
        });

        //[TODO]
        // This is a dummy implementation
        // - It should be refactored to send the messages to the clients
        // - Need some error handling and disconnection handling
        let clients = self.clients.clone();
        tokio::spawn(async move {
            while let Some(commit) = commit_rx.recv().await {
                let mut clients = clients.lock().await;
                for client in clients.iter_mut() {
                    let c_msg: ClientMsg = bincode::deserialize(&commit.get_data()).unwrap();

                    println!("Sending to client: {:?}", c_msg.get_data());
                    client.send(Message::Text(c_msg.get_data())).await.unwrap();
                }
            }
        });
    }
}

impl Writer {
    pub fn new() -> Self {
        Writer {
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn start(
        &self,
        mut writer_rx: Receiver<ClientMsg>,
        raft_tx: tokio::sync::mpsc::Sender<RequestLog>,
    ) {
        println!("Writer started");

        let buf: Arc<Mutex<Vec<ClientMsg>>> = self.buffer.clone();
        let buf2 = self.buffer.clone();
        let raft_tx1 = raft_tx.clone();
        let raft_tx2 = raft_tx.clone();

        // [NOTE]
        // Below code can be refactored to use tokio::select!
        tokio::spawn(async move {
            while let Some(msg) = writer_rx.recv().await {
                let mut size = 0;
                let mut local_buffer = Vec::<ClientMsg>::new();

                {
                    let mut buf: std::sync::MutexGuard<'_, Vec<ClientMsg>> = buf.lock().unwrap();
                    buf.push(msg);

                    size = buf.len();
                    if size > 10 {
                        local_buffer = buf.drain(0..size).collect();
                    }
                }

                // fowarding to Raft
                if size > 10 {
                    let raft_tx = raft_tx1.clone();
                    tokio::spawn(async move {
                        for msg in local_buffer.iter() {
                            println!("Sending to Raft: {:?}", msg.get_data());
                            raft_tx
                                .send(RequestLog::new(
                                    msg.get_uid().to_string(),
                                    msg.get_timestamp(),
                                    bincode::serialize(msg).unwrap(),
                                ))
                                .await
                                .unwrap();
                        }
                    });
                }
            }
        });

        tokio::spawn(async move {
            // fowarding to Raft each 5 ms
            loop {
                time::sleep(Duration::from_millis(5)).await;
                let mut local_buffer = Vec::<ClientMsg>::new();
                let mut size = 0;
                let mut lock = buf2.try_lock();

                if let Ok(ref mut buf) = lock {
                    size = buf.len();
                    if size > 0 {
                        local_buffer = buf.drain(0..size).collect();
                    }
                }

                if size > 0 {
                    let raft_tx = raft_tx2.clone();
                    tokio::spawn(async move {
                        for msg in local_buffer.iter() {
                            println!("[Timer] Sending to Raft: {:?}", msg.get_data());
                            raft_tx
                                .send(RequestLog::new(
                                    msg.get_uid().to_string(),
                                    msg.get_timestamp(),
                                    bincode::serialize(msg).unwrap(),
                                ))
                                .await
                                .unwrap();
                        }
                    });
                }
            }
        });
    }
}
