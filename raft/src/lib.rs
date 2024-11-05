use std::cmp::{max, min};
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, MutexGuard};

use raftchat::raft_chat_client::RaftChatClient;
use raftchat::raft_chat_server::{RaftChat, RaftChatServer};
use raftchat::Entry;
use raftchat::{AppendEntriesArgs, AppendEntriesRes};
use raftchat::{RequestVoteArgs, RequestVoteRes};
use raftchat::{UserRequestArgs, UserRequestRes};

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
    Follower(Option<u64>),
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

async fn update_term<'a, 'b>(
    guard: &'a mut MutexGuard<'b, RaftState>,
    new_term: u64,
    leader: Option<u64>,
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
