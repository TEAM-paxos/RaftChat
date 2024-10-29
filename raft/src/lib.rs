use std::cmp::{max, min};
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, MutexGuard};

use raftchat::raft_chat_client::RaftChatClient;
use raftchat::raft_chat_server::{RaftChat, RaftChatServer};
use raftchat::Entry;
use raftchat::{AppendEntriesArgs, AppendEntriesRes};
use raftchat::{RequestAppendArgs, RequestAppendRes};
use raftchat::{RequestVoteArgs, RequestVoteRes};

use tonic::transport::Channel;
use tonic::{Request, Response, Status};

use wal::WAL;

mod wal;

pub mod raftchat {
    tonic::include_proto!("raftchat");
}

struct RaftConfig {
    self_id: u64,
    peers: Vec<u64>,
}

enum Role {
    Leader,
    Follower,
    Candidate,
}

pub struct RaftState {
    current_term: u64,
    log: WAL,
    voted_for: Option<u64>,
    committed_length: u64,
    role: Role,
    leader: u64,
}

pub struct MyRaftChat {
    config: RaftConfig,
    state: Mutex<RaftState>,
    connections: RaftChatClient<Channel>,
}

async fn update_term<'a, 'b>(guard : &'a mut MutexGuard<'b, RaftState>, new_term : u64) {
    if guard.current_term < new_term {
        guard.current_term = new_term;
        guard.role = Role::Follower;
        guard.voted_for = None;
    }
}

#[tonic::async_trait]
impl RaftChat for MyRaftChat {
    async fn append_entries(
        &self,
        request: Request<AppendEntriesArgs>,
    ) -> Result<Response<AppendEntriesRes>, Status> {
        let args: AppendEntriesArgs = request.into_inner();
        let mut guard = self.state.lock().await;
        if args.term < guard.current_term {
            return Ok(Response::new(AppendEntriesRes {
                term: guard.current_term,
                success: false,
            }));
        }
        update_term(&mut guard, args.term);
        match guard
            .log
            .append_entries(args.prev_length, args.prev_term, &args.entries)
            .await
        {
            None => Ok(Response::new(AppendEntriesRes {
                term: guard.current_term,
                success: false,
            })),
            Some(l) => {
                guard.committed_length = max(guard.committed_length, min(args.committed_length, l));
                Ok(Response::new(AppendEntriesRes {
                    term: guard.current_term,
                    success: true,
                }))
            }
        }
    }

    async fn request_vote(
        &self,
        request: Request<RequestVoteArgs>,
    ) -> Result<Response<RequestVoteRes>, Status> {
        let args: RequestVoteArgs = request.into_inner();
        let mut guard = self.state.lock().await;
        if args.term < guard.current_term {
            return Ok(Response::new(RequestVoteRes {
                term: guard.current_term,
                vote_granted: false,
            }))
        }
        update_term(&mut guard, args.term);
        unimplemented!()
    }

    async fn request_append(
        &self,
        request: Request<RequestAppendArgs>,
    ) -> Result<Response<RequestAppendRes>, Status> {
        unimplemented!()
    }
}

// pub struct Raft {}
//
// struct RaftNode {
//     id: u64,
//     config: RaftConfig,
//     commit_tx: mpsc::Sender<Commit>,
//     propose_rx: mpsc::Receiver<Vec<u8>>,
// }
//
// pub struct Commit {
//     index: u64,
//     data: Vec<u8>,
// }
//
// impl Commit {
//     pub fn get_data(&self) -> Vec<u8> {
//         self.data.clone()
//     }
// }
//
// impl Raft {
//     // return a new commit channel
//     pub fn new(id: u64, peers: Vec<u64>) -> (mpsc::Receiver<Commit>, mpsc::Sender<Vec<u8>>) {
//         let (commit_tx, commit_rx) = mpsc::channel(15);
//         let (propose_tx, propose_rx) = mpsc::channel(15);
//
//         let mut raft_node = RaftNode {
//             id: id,
//             config: RaftConfig {
//                 id: id,
//                 peers: peers,
//             },
//             commit_tx: commit_tx,
//             propose_rx: propose_rx,
//         };
//
//         tokio::spawn(async move {
//             raft_node.start().await;
//         });
//
//         return (commit_rx, propose_tx);
//     }
// }
//
// impl RaftNode {
//     pub async fn start(&mut self) {
//         println!("Starting Raft server with id: {}", self.id);
//
//         let mut idx = 0;
//
//         // [NOTE] This is a dummy implementation
//         // echo back the data
//         while let Some(data) = self.propose_rx.recv().await {
//             println!("[RAFT] Received data: {:?}", data);
//
//             sleep(Duration::from_secs(1));
//
//             let value = Commit {
//                 index: idx,
//                 data: data,
//             };
//
//             self.commit_tx.send(value).await.unwrap();
//             idx += 1;
//         }
//
//         // [TODO] Implement WAL and Transport
//         // setup the WAL and Transport
//     }
// }
