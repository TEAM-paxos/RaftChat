pub mod mock_raft;
pub mod persistent_state;
pub mod raftchat_tonic;

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

use persistent_state::PersistentState;

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
    persistent_state: PersistentState,
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
    if guard.persistent_state.update_term(new_term) {
        guard.role = Role::Follower(leader);
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
        let current_term = guard.persistent_state.get_current_term();
        if args.term < current_term {
            return Ok(Response::new(AppendEntriesRes {
                term: current_term,
                success: false,
            }));
        }
        update_term(&mut guard, args.term, Some(args.leader_id));
        match guard
            .persistent_state
            .append_entries(args.prev_length, args.prev_term, &args.entries)
            .await
        {
            None => Ok(Response::new(AppendEntriesRes {
                term: current_term,
                success: false,
            })),
            Some(l) => {
                guard.committed_length = max(guard.committed_length, min(args.committed_length, l));
                Ok(Response::new(AppendEntriesRes {
                    term: current_term,
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
        let current_term = guard.persistent_state.get_current_term();
        if args.term < current_term {
            return Ok(Response::new(RequestVoteRes {
                term: current_term,
                vote_granted: false,
            }));
        }
        update_term(&mut guard, args.term, None);
        let vote_granted = guard.persistent_state.try_vote(&args.candidate_id);
        Ok(Response::new(RequestVoteRes {
            term: current_term,
            vote_granted: vote_granted,
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
