pub mod mock_raft;
pub mod raftchat_tonic;
pub mod wal;

use std::cmp::{max, min};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, MutexGuard};

use raftchat_tonic::raft_chat_client::RaftChatClient;
use raftchat_tonic::raft_chat_server::{RaftChat, RaftChatServer};
use raftchat_tonic::Entry;
use raftchat_tonic::{AppendEntriesArgs, AppendEntriesRes};
use raftchat_tonic::{RequestVoteArgs, RequestVoteRes};
use raftchat_tonic::{UserRequestArgs, UserRequestRes};

use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status};

use wal::WAL;

pub struct RaftConfig {
    pub serve_addr: SocketAddr,
    pub self_id: String,
    pub peers: Vec<String>, // except self
}

pub enum Role {
    Leader,
    Follower(Option<String>),
    Candidate,
}

pub struct RaftState {
    current_term: u64,
    log: WAL,
    voted_for: Option<String>,
    committed_length: u64,
    role: Role,
}

pub struct MyRaftChat {
    config: RaftConfig,
    state: Mutex<RaftState>,
    connections: HashMap<String, Option<RaftChatClient<Channel>>>,
}

async fn update_term<'a, 'b>(
    guard: &'a mut MutexGuard<'b, RaftState>,
    new_term: u64,
    leader: Option<String>,
) {
    if guard.current_term < new_term {
        guard.current_term = new_term;
        guard.role = Role::Follower(leader);
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
        update_term(&mut guard, args.term, Some(args.leader_id));
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
            }));
        }
        update_term(&mut guard, args.term, None);
        let voted_granted = if guard.voted_for == None {
            guard.voted_for = Some(args.candidate_id);
            true
        } else if guard.voted_for == Some(args.candidate_id) {
            true
        } else {
            false
        };
        Ok(Response::new(RequestVoteRes {
            term: guard.current_term,
            vote_granted: voted_granted,
        }))
    }

    async fn user_request(
        &self,
        request: Request<UserRequestArgs>,
    ) -> Result<Response<UserRequestRes>, Status> {
        unimplemented!()
    }
}

pub fn run_raft(
    config: RaftConfig,
    log_tx: mpsc::Sender<Entry>,
    req_rx: mpsc::Receiver<UserRequestArgs>,
) {
    let raft_chat = MyRaftChat {
        config: config,
        state: unimplemented!(),
        connections: unimplemented!(),
    };

    let rpc_future = Server::builder()
        .add_service(RaftChatServer::new(raft_chat))
        .serve(config.serve_addr);

    tokio::spawn(async move {
        rpc_future.await.unwrap();
    });
}
