pub mod mock_raft;
pub mod persistent_state;
pub mod raftchat_tonic;
pub mod state_machine;
pub mod wal;

use parking_lot::{Condvar, Mutex};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task;
use tokio::time;

use raftchat_tonic::raft_chat_client::RaftChatClient;
use raftchat_tonic::raft_chat_server::{RaftChat, RaftChatServer};
use raftchat_tonic::{AppendEntriesArgs, AppendEntriesRes};
use raftchat_tonic::{Command, Entry};
use raftchat_tonic::{RequestVoteArgs, RequestVoteRes};
use raftchat_tonic::{UserRequestArgs, UserRequestRes};

use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status};

use persistent_state::PersistentState;
use state_machine::{SMWrapper, UserMessageIdMap};
use wal::WAL;

use tokio_util::task::AbortOnDropHandle;

type ArcMutexGuard<T> = parking_lot::ArcMutexGuard<parking_lot::RawMutex, T>;

pub struct RaftConfig {
    pub serve_addr: SocketAddr,
    pub self_id: &'static str,
    pub peers: Vec<&'static str>, // except self
    pub election_duration: Duration,
    pub heartbeat_duration: Duration,
    pub persistent_state_path: &'static Path,
    pub wal_path: &'static Path,
}

impl RaftConfig {
    fn get_peer(&self, s: &str) -> Option<&'static str> {
        self.peers.iter().find(|&&peer| peer == s).copied()
    }
}

pub struct LeaderState {
    heartbeat_handle: AbortOnDropHandle<()>,
    next_length: HashMap<&'static str, u64>,
    match_length: HashMap<&'static str, u64>,
    // NB : alarm false to senders when dropping LeaderState
    commit_alarm: Vec<(u64, oneshot::Sender<bool>)>,
}

pub struct FollowerState {
    current_leader: Option<&'static str>,
    timeout_handle: AbortOnDropHandle<()>,
}

pub struct CandidateState {
    election_handle: AbortOnDropHandle<()>,
    timeout_handle: AbortOnDropHandle<()>,
}

pub enum Role {
    Leader(LeaderState),
    Follower(FollowerState),
    Candidate(CandidateState),
}

pub struct RaftState {
    persistent_state: PersistentState, // current Term & voted For
    sm: SMWrapper<UserMessageIdMap>,   // log[]
    committed_length: u64,             // committed index in paper
    role: Role,
    connections: HashMap<&'static str, RaftChatClient<Channel>>,
}

pub struct MyRaftChat {
    config: Arc<RaftConfig>,
    state: Arc<Mutex<RaftState>>,
    append_cvar: Arc<Condvar>,
}

impl RaftState {
    async fn timeout_future(state: Arc<Mutex<Self>>, config: Arc<RaftConfig>) {
        time::sleep(config.election_duration).await;
        let mut guard = state.lock_arc();
        // TODO : Add checking for term and role
        guard.persistent_state.increment_term();
        // NB : this will cancel itself
        Self::reset_to_candidate(&mut guard, config);
    }

    async fn heartbeat_future(state: Arc<Mutex<Self>>, config: Arc<RaftConfig>) {
        time::sleep(config.heartbeat_duration).await;
        let mut guard = state.clone().lock_arc();
        // TODO
    }

    async fn election_future(state: Arc<Mutex<Self>>, config: Arc<RaftConfig>) {
        // NB : Repeatedly requesting vote for every peers is also possible,
        // But we will request vote for just once, just for ease of implementation.
        let (req, connections) = {
            let guard = state.lock();
            let req = RequestVoteArgs {
                candidate_id: String::from(config.self_id),
                prev_length: guard.sm.wal().len(),
                prev_term: guard.sm.wal().last_term(),
                term: guard.persistent_state.get_current_term(),
            };
            let connections = guard.connections.clone();
            (req, connections)
        };

        let (vote_tx, mut vote_rx) = mpsc::channel::<bool>(10);
        let mut handles = vec![];
        for mut client in connections {
            let req_cloned = req.clone();
            let vote_tx_cloned = vote_tx.clone();
            handles.push(AbortOnDropHandle::new(task::spawn(async move {
                match client.1.request_vote(Request::new(req_cloned)).await {
                    Ok(res) => vote_tx_cloned.send(res.into_inner().vote_granted).await,
                    Err(_) => vote_tx_cloned.send(false).await,
                }
            })));
        }

        let mut vote_count: usize = 0;
        let elected = loop {
            // NB : This loop will get stuck if number of vote is not sufficient.
            // We have timeout for election anyway.
            if let Some(true) = vote_rx.recv().await {
                vote_count = vote_count + 1;
            } else {
                break false;
            }
            if vote_count * 2 > config.peers.len() + 1 {
                break true;
            }
        };

        if elected {
            let mut guard = state.lock_arc();
            Self::reset_to_leader(&mut guard, config);
        }
    }

    fn reset_to_leader(guard: &mut ArcMutexGuard<Self>, config: Arc<RaftConfig>) {
        let state = ArcMutexGuard::mutex(guard).clone();
        guard.role = Role::Leader(LeaderState {
            heartbeat_handle: AbortOnDropHandle::new(task::spawn(Self::heartbeat_future(
                state.clone(),
                config.clone(),
            ))),
            next_length: config
                .peers
                .iter()
                .map(|&p| (p, guard.sm.wal().len()))
                .collect(),
            match_length: config.peers.iter().map(|&p| (p, 0)).collect(),
            commit_alarm: Vec::new(),
        });
    }

    fn reset_to_candidate(guard: &mut ArcMutexGuard<Self>, config: Arc<RaftConfig>) {
        let state: Arc<Mutex<Self>> = ArcMutexGuard::mutex(guard).clone();
        guard.role = Role::Candidate(CandidateState {
            election_handle: AbortOnDropHandle::new(task::spawn(Self::election_future(
                state.clone(),
                config.clone(),
            ))),
            timeout_handle: AbortOnDropHandle::new(task::spawn(Self::timeout_future(
                state, config,
            ))),
        });
    }

    fn reset_to_follower(
        guard: &mut ArcMutexGuard<Self>,
        config: Arc<RaftConfig>,
        current_leader: Option<&'static str>,
    ) {
        let state: Arc<Mutex<Self>> = ArcMutexGuard::mutex(guard).clone();
        guard.role = Role::Follower(FollowerState {
            current_leader: current_leader,
            timeout_handle: AbortOnDropHandle::new(task::spawn(Self::timeout_future(
                state, config,
            ))),
        });
    }
}

#[tonic::async_trait]
impl RaftChat for MyRaftChat {
    async fn append_entries(
        &self,
        request: Request<AppendEntriesArgs>,
    ) -> Result<Response<AppendEntriesRes>, Status> {
        let args: AppendEntriesArgs = request.into_inner();
        let mut guard = self.state.clone().lock_arc();
        let (current_term, ok) = guard.persistent_state.update_term(args.term);
        if !ok {
            return Ok(Response::new(AppendEntriesRes {
                term: current_term,
                success: false,
            }));
        }
        let Some(leader_id) = self.config.get_peer(&args.leader_id) else {
            unimplemented!();
        };
        RaftState::reset_to_follower(&mut guard, self.config.clone(), Some(leader_id));
        match guard
            .sm
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
                drop(guard);
                self.append_cvar.notify_one();
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
        let mut guard = self.state.clone().lock_arc();
        let Some(candidate_id) = self.config.get_peer(&args.candidate_id) else {
            unimplemented!();
        };
        let (current_term, ok) = guard.persistent_state.try_vote(args.term, candidate_id);
        if !ok {
            return Ok(Response::new(RequestVoteRes {
                term: current_term,
                vote_granted: false,
            }));
        }
        RaftState::reset_to_follower(&mut guard, self.config.clone(), None);
        Ok(Response::new(RequestVoteRes {
            term: current_term,
            vote_granted: true,
        }))
    }

    // handling user request
    // - follower => forward to leader and return result.
    // - candidate => retuurn false
    // - leader => propose new entry, wait until committed and return result.
    async fn user_request(
        &self,
        request: Request<UserRequestArgs>,
    ) -> Result<Response<UserRequestRes>, Status> {
        let mut client;
        {
            let mut guard = self.state.lock();

            match &mut *guard {
                RaftState {
                    role: Role::Leader(leader_state),
                    sm,
                    persistent_state,
                    ..
                } => {
                    let args = request.into_inner();

                    // 1. blocking
                    let client_committed_idx = sm.state().get(&args.client_id);

                    if client_committed_idx == None
                        || client_committed_idx.unwrap() + 1 != args.message_id
                    {
                        return Ok(Response::new(UserRequestRes { success: false }));
                    }

                    // 2. append log in wal
                    let proposed_idx = sm.propose_entry(Entry {
                        term: persistent_state.get_current_term(),
                        command: Some(Command {
                            client_id: args.client_id,
                            message_id: args.message_id,
                            data: args.data,
                        }),
                    });

                    // 3. append channel raft state
                    let (tx, rx) = oneshot::channel();
                    leader_state.commit_alarm.push((proposed_idx, tx));

                    drop(guard);
                    // 4. call commit func

                    // 5. waiting commit
                    match rx.blocking_recv() {
                        Ok(true) => return Ok(Response::new(UserRequestRes { success: true })),
                        Ok(false) => return Ok(Response::new(UserRequestRes { success: false })),
                        Err(_) => return Ok(Response::new(UserRequestRes { success: false })),
                    };
                }
                RaftState {
                    role: Role::Follower(follower_state),
                    ..
                } => {
                    if let Some(leader_id) = follower_state.current_leader {
                        client = guard
                            .connections
                            .get(leader_id)
                            .expect("Get connection failed : follower don't know who's the leader")
                            .clone();

                        drop(guard);
                    }
                    else {return Ok(Response::new(UserRequestRes { success: false }));}
                }
                RaftState {
                    role: Role::Candidate(_),
                    ..
                } => {
                    return Ok(Response::new(UserRequestRes { success: false }));
                }
            }
        }
        
        let res = client.user_request(request).await?;
        return Ok(res);
    }
}

pub fn publisher(
    state: Arc<Mutex<RaftState>>,
    log_tx: mpsc::Sender<Entry>,
    append_cvar: Arc<Condvar>,
) {
    let sent_length: usize = 0;
    loop {
        let mut guard = state.lock();
        let committed_length = guard.committed_length as usize;
        if sent_length < committed_length {
            let entries = guard.sm.wal().as_slice()[sent_length..committed_length].to_vec();
            drop(guard);
            for entry in entries {
                log_tx.blocking_send(entry).unwrap();
            }
        } else {
            append_cvar.wait(&mut guard);
        }
    }
}

pub fn run_raft(
    config: RaftConfig,
    log_tx: mpsc::Sender<Entry>,
    req_rx: mpsc::Receiver<UserRequestArgs>,
) {
    let serve_addr = config.serve_addr;
    let persistent_state_path = config.persistent_state_path;
    let wal_path = config.wal_path;
    let append_cvar = Arc::new(Condvar::new());
    let raft_chat = Arc::new(MyRaftChat {
        config: Arc::new(config),
        state: Arc::new(Mutex::new(RaftState {
            persistent_state: PersistentState::new(persistent_state_path),
            sm: SMWrapper::new(WAL::new(wal_path)),
            committed_length: 0,
            role: Role::Follower(FollowerState {
                current_leader: None,
                timeout_handle: AbortOnDropHandle::new(task::spawn(async {})),
            }),
            connections: HashMap::new(),
        })),
        append_cvar: append_cvar.clone(),
    });

    let state = raft_chat.state.clone();
    task::spawn_blocking(move || publisher(state, log_tx, append_cvar));

    raft_chat.state.lock().role = Role::Follower(FollowerState {
        current_leader: None,
        timeout_handle: AbortOnDropHandle::new(task::spawn(RaftState::timeout_future(
            raft_chat.state.clone(),
            raft_chat.config.clone(),
        ))),
    });

    let rpc_future = Server::builder()
        .add_service(RaftChatServer::from_arc(raft_chat))
        .serve(serve_addr);
    task::spawn(rpc_future);
}
