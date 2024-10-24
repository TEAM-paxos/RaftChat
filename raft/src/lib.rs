use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc;

mod transport;
mod wal;

pub struct Raft {}

struct RaftConfig {
    id: u64,
    peers: Vec<u64>,
}

struct RaftNode {
    id: u64,
    config: RaftConfig,
    commit_tx: mpsc::Sender<Commit>,
    propose_rx: mpsc::Receiver<Vec<u8>>,
}

pub struct Commit {
    index: u64,
    data: Vec<u8>,
}

impl Commit {
    pub fn get_index(&self) -> u64 {
        self.index
    }
    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

impl Raft {
    // return a new commit channel
    pub fn new(id: u64, peers: Vec<u64>) -> (mpsc::Receiver<Commit>, mpsc::Sender<Vec<u8>>) {
        let (commit_tx, commit_rx) = mpsc::channel(15);
        let (propose_tx, propose_rx) = mpsc::channel(15);

        let mut raft_node = RaftNode {
            id: id,
            config: RaftConfig {
                id: id,
                peers: peers,
            },
            commit_tx: commit_tx,
            propose_rx: propose_rx,
        };

        tokio::spawn(async move {
            raft_node.start().await;
        });

        return (commit_rx, propose_tx);
    }
}

impl RaftNode {
    pub async fn start(&mut self) {
        println!("Starting Raft server with id: {}", self.id);

        let mut idx = 0;

        // [NOTE] This is a dummy implementation
        // echo back the data
        while let Some(data) = self.propose_rx.recv().await {
            println!("[RAFT] Received data");

            sleep(Duration::from_secs(1));

            let value = Commit {
                index: idx,
                data: data,
            };

            self.commit_tx.send(value).await.unwrap();
            idx += 1;
        }

        // [TODO] Implement WAL and Transport
        // setup the WAL and Transport
    }
}
