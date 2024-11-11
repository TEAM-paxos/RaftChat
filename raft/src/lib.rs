pub mod mock_raft;
pub mod persistent_state;
pub mod raftchat_tonic;

use std::cmp::{max, min};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, MutexGuard};
use tokio::task;
use tokio::time;

use raftchat_tonic::raft_chat_client::RaftChatClient;
use raftchat_tonic::raft_chat_server::{RaftChat, RaftChatServer};
use raftchat_tonic::Entry;
use raftchat_tonic::{AppendEntriesArgs, AppendEntriesRes};
use raftchat_tonic::{RequestVoteArgs, RequestVoteRes};
use raftchat_tonic::{UserRequestArgs, UserRequestRes};

use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status};

use persistent_state::PersistentState;

use tokio_util::task::AbortOnDropHandle;

pub struct RaftConfig {
    pub serve_addr: SocketAddr,
    pub self_id: String,
    pub peers: Vec<String>, // except self
    pub timeout_duration: Duration,
    pub persistent_state_path: &'static Path,
}

pub struct LeaderState {
    heartbeat_handle: AbortOnDropHandle<()>,
    next_index: HashMap<String, u64>,
    match_length: HashMap<String, u64>,
}

pub struct FollowerState {
    current_leader: Option<String>,
    election_handle: AbortOnDropHandle<()>,
}

pub struct CandidateState {
    election_handle: AbortOnDropHandle<()>,
}

pub enum Role {
    Leader(LeaderState),
    Follower(FollowerState),
    Candidate(CandidateState),
}

pub struct RaftState {
    persistent_state: PersistentState,
    committed_length: u64,
    role: Role,
}

pub struct MyRaftChat {
    config: RaftConfig,
    state: Mutex<RaftState>,                 // mutex 0
    log_sender: Mutex<(u64, Sender<Entry>)>, // mutex 1
    connections: HashMap<String, Option<RaftChatClient<Channel>>>,
}

impl MyRaftChat {
    async fn election_after_timeout(self: Arc<Self>, t: time::Duration) {
        time::sleep(t).await;
        let mut guard = self.state.lock().await;
        // TODO : increment term
        guard.role = Role::Candidate(unimplemented!());
        // TODO : reset election_handle
        // TODO : start election
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
        guard.persistent_state.update_term(args.term);
        // TODO : reset election_handle
        guard.role = Role::Follower(FollowerState {
            current_leader: Some(args.leader_id),
            election_handle: unimplemented!(),
        });
        match guard
            .persistent_state
            .append_entries(args.prev_length, args.prev_term, &args.entries)
        {
            None => Ok(Response::new(AppendEntriesRes {
                term: current_term,
                success: false,
            })),
            Some(compatible_length) => {
                guard.committed_length = max(
                    guard.committed_length,
                    min(args.committed_length, compatible_length),
                );
                // TODO : send committed logs
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
        if guard.persistent_state.update_term(args.term) {
            guard.role = Role::Follower(unimplemented!());
        }
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

pub fn run_raft(config: RaftConfig, log_tx: Sender<Entry>, req_rx: Receiver<UserRequestArgs>) {
    let serve_addr = config.serve_addr;
    let timeout_duration = config.timeout_duration;
    let persistent_state_path = config.persistent_state_path;
    let raft_chat = Arc::new(MyRaftChat {
        config: config,
        state: Mutex::new(RaftState {
            persistent_state: PersistentState::new(persistent_state_path),
            committed_length: 0,
            role: Role::Follower(FollowerState {
                current_leader: None,
                election_handle: AbortOnDropHandle::new(task::spawn(async {})),
            }),
        }),
        log_sender: Mutex::new((0, log_tx)),
        connections: HashMap::new(),
    });

    raft_chat.state.blocking_lock().role = Role::Follower(FollowerState {
        current_leader: None,
        election_handle: AbortOnDropHandle::new(task::spawn(
            raft_chat.clone().election_after_timeout(timeout_duration),
        )),
    });

    let rpc_future = Server::builder()
        .add_service(RaftChatServer::from_arc(raft_chat))
        .serve(serve_addr);
    task::spawn(rpc_future);
}
