use crate::raftchat_tonic::{Command, Entry, UserRequestArgs};
use crate::RaftConfig;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

struct RaftNode {
    config: RaftConfig,
    log_tx: mpsc::Sender<Entry>,
    req_rx: mpsc::Receiver<UserRequestArgs>,
    test_flag: bool,
    client_timestamp_map: std::collections::HashMap<String, u64>,
}

pub fn run_mock_raft(
    config: RaftConfig,
    log_tx: mpsc::Sender<Entry>,
    req_rx: mpsc::Receiver<UserRequestArgs>,
) {
    let mut raft_node = RaftNode {
        config: config,
        log_tx: log_tx,
        req_rx: req_rx,
        test_flag: true,
        client_timestamp_map: std::collections::HashMap::new(),
    };

    tokio::spawn(async move {
        raft_node.start().await;
    });

    // tokio::task::spawn_blocking(move || {
    //     tokio::runtime::Handle::current().block_on(async {
    //         raft_node.start().await;
    //     });
    // });
}

impl RaftNode {
    pub async fn start(&mut self) {
        //println!("Starting Raft server with id: {}", self.id);

        let mut idx = 0;
        let mut drop: i32 = 3;

        // [NOTE] This is a dummy implementation
        // echo back the data
        while let Some(data) = self.req_rx.recv().await {
            // no op test
            if idx == 5 {
                let value = Entry {
                    term: 0,
                    command: None,
                };

                self.log_tx.send(value).await.unwrap();
                idx += 1;
            }

            //println!("[RAFT] Received data");

            sleep(Duration::from_secs(1)).await;

            // now msg is committed and wirtten on disk.

            let time = self.client_timestamp_map.get(&data.client_id).unwrap_or(&1);

            if self.test_flag && idx == drop {
                drop += 100;
                continue;
            }

            // filter duplciate requests and out of order requests
            if *time != data.message_id {
                continue;
            }

            self.client_timestamp_map
                .insert(data.client_id.clone(), *time + 1);

            let value = Entry {
                term: 0,
                command: Some(Command {
                    client_id: data.client_id,
                    message_id: data.message_id,
                    data: data.data,
                }),
            };

            self.log_tx.send(value).await.unwrap();
            idx += 1;
        }

        // [TODO] Implement WAL and Transport
        // setup the WAL and Transport
    }
}
