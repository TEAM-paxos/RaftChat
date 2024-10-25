use crate::data_model::msg::{ClientMsg, Msg, ServerMsg};
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use raft::Commit;
use tokio::sync::TryLockError;
use tokio_tungstenite::tungstenite::client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio::time::{self, Duration};
use tokio_tungstenite::tungstenite::handshake::server;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

type Stream = SplitSink<WebSocketStream<TcpStream>, Message>;

// Writer task
// - Receives messages from the client_handler and forwards them to the Raft
// - If the buffer size is greater than 10, it sends the messages to the Raft
// - It also sends the messages to the Raft every 5 ms
pub struct Writer {
    // < client's address, client's committed index >
    // shared with publisher
    client_commit_idx: Arc<tokio::sync::Mutex<HashMap<String, u64>>>,
}

// Publisher task
// - It preseves the stream that sended from the client_handler
// - Receives committed messages from the Raft and sends them to the clients
pub struct Publisher {
    state_machine:   Arc<tokio::sync::Mutex<Vec<Msg>>>,

    // < client's address, client's committed index >
    // shared with writer
    client_commit_idx: Arc<tokio::sync::Mutex<HashMap<String, u64>>>,

    // < client's address, client stream >
    clients: Arc<tokio::sync::Mutex<HashMap<String, Stream>>>,

    // lock
    pub_lock : Arc<tokio::sync::Mutex<u8>>
}

impl Publisher {
    pub fn new(
        // when recover the server, backup the state machine from raft.
        state_machine: Vec<Msg>,
        client_commit_idx: Arc<tokio::sync::Mutex<HashMap<String, u64>>>,
    ) -> Self {
        Publisher {
            state_machine: Arc::new(tokio::sync::Mutex::new(state_machine)),
            client_commit_idx,
            clients: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            pub_lock: Arc::new(tokio::sync::Mutex::new(0)),
        }
    }

    pub async fn start(
        &self,
        mut commit_rx: Receiver<Commit>,
        mut pub_rx: Receiver<(String, Stream)>,
    ) {
        println!("Publisher started");

        // receive stream from handler and store it in the hashmap
        let clients = self.clients.clone();
        tokio::spawn(async move {
            while let Some((addr, stream)) = pub_rx.recv().await {
                println!("Received a new client");
                clients.lock().await.insert(addr, stream);
            }
        });

        //[TODO]
        // This is a dummy implementation
        // - It should be refactored to send the messages to the clients
        // - Need some error handling and disconnection handling
        let clients = self.clients.clone();
        let client_commit_idx = self.client_commit_idx.clone();
        let state_machine = self.state_machine.clone();
        let pub_lock = self.pub_lock.clone();
        tokio::spawn(async move {
            while let Some(commit) = commit_rx.recv().await {
                let lock = pub_lock.lock().await;
                println!("got lock");
                let raft_commit_idx = commit.get_index();

                // [Warn] type error
                if raft_commit_idx != state_machine.lock().await.len() as u64 {
                    panic!(" split brained between raft and state machine");
                }
                let c_msg: Msg = bincode::deserialize(&commit.get_data()).unwrap();
                state_machine.lock().await.push(c_msg);
                let mut delete_candidates = Vec::new();

                {
                    let mut clients_: tokio::sync::MutexGuard<
                        '_,
                        HashMap<String, SplitSink<WebSocketStream<TcpStream>, Message>>,
                    > = clients.lock().await;

                    // publish
                    for (addr, client_stream) in clients_.iter_mut() {
                        let client_idx;
                        {
                            let client_commit_idx = client_commit_idx.lock().await;
                            client_idx = client_commit_idx.get(addr).unwrap_or(&0).clone();
                        }

                        // build server msg
                        let mut server_msgs = Vec::new();

                        for i in client_idx..=raft_commit_idx {
                            let temp = ServerMsg::new(i, state_machine.lock().await[i as usize].clone());
                            server_msgs.push(temp);
                        }

                        println!(
                            "tick Sending to {:?} cli idx: ({:?}): msg len: {:?} raft idx: {:?}",
                            addr,
                            client_idx,
                            server_msgs.len(),
                            raft_commit_idx
                        );

                        let res = client_stream
                            .send(Message::Text(serde_json::to_string(&server_msgs).unwrap()))
                            .await;

                        match res {
                            Ok(_) => {
                                
                            }
                            Err(_) => {
                                println!("Failed to send to {:?}", addr);
                                delete_candidates.push(addr.clone());
                            }
                        }
                    }
                }

                {
                    let mut clients_: tokio::sync::MutexGuard<
                        '_,
                        HashMap<String, SplitSink<WebSocketStream<TcpStream>, Message>>,
                    > = clients.lock().await;

                    for addr in delete_candidates.iter() {
                        println!("connection closed {:?}", addr);
                        client_commit_idx.lock().await.remove(addr);
                        clients_.remove(addr);
                    }

                    for i in state_machine.lock().await.iter() {
                        print!("{:?} ", i.get_content());
                    } println!(" << {:?} <<< ", state_machine.lock().await.len());
                }

                drop(lock);
            }
        });

        let clients = self.clients.clone();
        let client_commit_idx = self.client_commit_idx.clone();
        let state_machine = self.state_machine.clone();
        let pub_lock = self.pub_lock.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;

                let lock = pub_lock.try_lock();
                match lock {
                    Ok(_) => {
                        let raft_commit_idx;
                        if state_machine.lock().await.len() == 0 {
                            drop(lock);
                            continue;
                        }
                        else {
                            raft_commit_idx = state_machine.lock().await.len() as u64 - 1;
                        }

                        let mut delete_candidates = Vec::new();

                        {
                            let mut clients_: tokio::sync::MutexGuard<
                                '_,
                                HashMap<String, SplitSink<WebSocketStream<TcpStream>, Message>>,
                            > = clients.lock().await;

                           

                            // publish
                            for (addr, client_stream) in clients_.iter_mut() {
                                let client_idx;
                                {
                                    let client_commit_idx = client_commit_idx.lock().await;
                                    client_idx = client_commit_idx.get(addr).unwrap_or(&0).clone();
                                }

                                // build server msg
                                let mut server_msgs = Vec::new();

                                for i in client_idx..=raft_commit_idx {
                                    let temp = ServerMsg::new(i, state_machine.lock().await[i as usize].clone());
                                    server_msgs.push(temp);
                                }

                                println!(
                                    "tick Sending to {:?} cli idx: ({:?}): msg len: {:?} raft idx: {:?}",
                                    addr,
                                    client_idx,
                                    server_msgs.len(),
                                    raft_commit_idx
                                );

                                let res = client_stream
                                    .send(Message::Text(serde_json::to_string(&server_msgs).unwrap()))
                                    .await;

                                match res {
                                    Ok(_) => {
                                        
                                    }
                                    Err(_) => {
                                        println!("Failed to send to {:?}", addr);
                                        delete_candidates.push(addr.clone());
                                    }
                                }
                            }
                        }

                        {
                            let mut clients_: tokio::sync::MutexGuard<
                                '_,
                                HashMap<String, SplitSink<WebSocketStream<TcpStream>, Message>>,
                            > = clients.lock().await;

                            for addr in delete_candidates.iter() {
                                println!("connection closed {:?}", addr);
                                client_commit_idx.lock().await.remove(addr);
                                clients_.remove(addr);
                            }

                            println!("now clients {:?} {:?}", 
                                client_commit_idx.lock().await.len(),
                                clients_.len()
                            )
                        }
                        drop(lock);
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        });

    }
}

impl Writer {
    pub fn new(client_commit_idx: Arc<tokio::sync::Mutex<HashMap<String, u64>>>) -> Self {
        Writer {
            //buffer: Arc::new(Mutex::new(Vec::new())),
            client_commit_idx,
        }
    }

    pub async fn start(
        &self,
        mut writer_rx: Receiver<(String, ClientMsg)>,
        raft_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    ) {
        println!("Writer started");
        let client_commit_idx = self.client_commit_idx.clone();

        // [NOTE]
        // Below code can be refactored to use tokio::select!
        tokio::spawn(async move {
            while let Some((addr, client_msg)) = writer_rx.recv().await {
                println!("Received a message from client: {:?}", addr);
                let messages: &Vec<Msg> = client_msg.get_messages();
                let index = client_msg.get_committed_index();

                // update client's index
                client_commit_idx.lock().await.insert(addr, index);

                for msg in messages.iter() {
                    println!(
                        "Sending to Raft: {:?} : {:?}",
                        msg.get_uid(),
                        msg.get_content()
                    );
                    raft_tx
                        .send(bincode::serialize(msg).unwrap())
                        .await
                        .unwrap();
                }
            }
        });
    }
}
