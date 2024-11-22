pub mod mock_raft;
pub mod persistent_state;
pub mod raftchat_tonic;
pub mod state_machine;
pub mod wal;

use parking_lot::{Condvar, Mutex, MutexGuard};
use rand::Rng;
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

use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Request, Response, Status};

use persistent_state::PersistentState;
use state_machine::{SMWrapper, UserMessageIdMap};
use wal::WAL;

use tokio_util::task::AbortOnDropHandle;

use log::{debug, info};
use std::pin::Pin;

#[derive(Debug)]
pub struct RaftConfig {
    pub serve_addr: SocketAddr,
    pub self_id: &'static str,
    pub peers: Vec<&'static str>,      // except self
    pub election_duration: (u64, u64), // lower~upper bound (ms)
    pub heartbeat_duration: Duration,
    pub persistent_state_path: &'static Path,
    pub wal_path: &'static Path,
}

impl RaftConfig {
    fn get_peer(&self, s: &str) -> Option<&'static str> {
        self.peers.iter().find(|&&peer| peer == s).copied()
    }

    fn cluster_size(&self) -> usize {
        self.peers.len() + 1
    }

    fn quorum_size(&self) -> usize {
        (self.cluster_size() / 2) + 1
    }
}

pub struct LeaderState {
    #[allow(dead_code)]
    heartbeat_handle: AbortOnDropHandle<()>,
    prev_length: HashMap<&'static str, u64>,
    match_length: HashMap<&'static str, u64>,
    // NB : alarm false to senders when dropping LeaderState
    commit_alarm: Vec<(u64, oneshot::Sender<bool>)>,
}

pub struct FollowerState {
    current_leader: Option<&'static str>,
    #[allow(dead_code)]
    timeout_handle: AbortOnDropHandle<()>,
}

pub struct CandidateState {
    #[allow(dead_code)]
    election_handle: AbortOnDropHandle<()>,
    #[allow(dead_code)]
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
    config: RaftConfig,
    state: Mutex<RaftState>,
    committed_length_cvar: Condvar,
    propose_cvar: Condvar,
}

impl MyRaftChat {
    async fn timeout_future(self: Arc<Self>) {
        let rand_duration = rand::thread_rng()
            .gen_range(self.config.election_duration.0..self.config.election_duration.1);

        time::sleep(std::time::Duration::from_millis(rand_duration)).await;
        let mut guard = self.state.lock();
        // TODO : Add checking for term and role

        guard.persistent_state.start_election(self.config.self_id);
        // NB : this will cancel itself
        self.reset_to_candidate(&mut guard);
    }

    async fn heartbeat_future(self: Arc<Self>) {
        loop {
            time::sleep(self.config.heartbeat_duration).await;
            let guard = self.state.lock();
            if let Role::Leader(s) = &guard.role {
                for (peer, client) in guard.connections.clone() {
                    let prev_length = s.prev_length[peer];
                    let args = AppendEntriesArgs {
                        term: guard.persistent_state.current_term(),
                        leader_id: String::from(self.config.self_id),
                        prev_length: prev_length,
                        prev_term: guard.sm.wal().last_term_for(prev_length),
                        entries: vec![],
                        committed_length: guard.committed_length,
                    };
                    debug!("{} send heartbeat to {}", self.config.self_id, peer);
                    task::spawn(self.clone().append_entries_future(peer, client, args));
                }
            } else {
                break;
            }
        }
    }

    async fn election_future(self: Arc<Self>) {
        // NB : Repeatedly requesting vote for every peers is more robust,
        // But we will request vote only once, just for ease of implementation.
        let (req, connections) = {
            let guard = self.state.lock();
            let req = RequestVoteArgs {
                candidate_id: String::from(self.config.self_id),
                prev_length: guard.sm.wal().len(),
                prev_term: guard.sm.wal().last_term(),
                term: guard.persistent_state.current_term(),
            };
            let connections = guard.connections.clone();
            (req, connections)
        };

        let (vote_tx, mut vote_rx) = mpsc::channel::<bool>(10);
        let mut handles = vec![];
        for (_peer, mut client) in connections {
            let req_cloned = req.clone();
            let vote_tx_cloned = vote_tx.clone();
            handles.push(AbortOnDropHandle::new(task::spawn(async move {
                match client.request_vote(Request::new(req_cloned)).await {
                    Ok(res) => vote_tx_cloned.send(res.into_inner().vote_granted).await,
                    Err(_) => vote_tx_cloned.send(false).await,
                }
            })));
        }

        let mut vote_count: usize = 1; // Candidates vote for itself
        let elected = loop {
            // NB : This loop will get stuck if number of vote is not sufficient.
            // We have timeout for election anyway.
            if let Some(b) = vote_rx.recv().await {
                if b {
                    vote_count = vote_count + 1;
                }
            } else {
                break false;
            }
            if vote_count >= self.config.quorum_size() {
                break true;
            }
        };

        if elected {
            info!("{} is elected as leader", self.config.self_id);

            let mut guard = self.state.lock();
            self.reset_to_leader(&mut guard);
            drop(guard);
            self.propose_cvar.notify_all();
        }
    }

    async fn append_entries_future(
        self: Arc<Self>,
        peer: &'static str,
        mut client: RaftChatClient<Channel>,
        args: AppendEntriesArgs,
    ) {
        let term = args.term;
        let prev_length = args.prev_length;
        let entries_len = args.entries.len() as u64;
        if entries_len > 0 {
            info!("send non empty append entries to {}", peer);
        }
        if let Ok(res) = client.append_entries(Request::new(args)).await {
            let res = res.into_inner();
            if res.term == term {
                let mut guard = self.state.lock();
                if let (
                    true,
                    RaftState {
                        sm,
                        role: Role::Leader(s),
                        committed_length,
                        ..
                    },
                ) = (guard.persistent_state.current_term() == term, &mut *guard)
                {
                    if res.success {
                        if entries_len > 0 {
                            info!("received append entries ack from {} (succed)", peer);
                        }
                        // Update match_length
                        // TODO : Check this
                        let l = s.match_length.get_mut(peer).unwrap();
                        let new_l = max(*l, prev_length + entries_len);
                        *l = new_l;
                        s.prev_length.insert(peer, new_l);

                        // Update committed_length
                        let mut v: Vec<std::cmp::Reverse<u64>> = s
                            .match_length
                            .values()
                            .cloned()
                            .map(|x| std::cmp::Reverse(x))
                            .collect();
                        v.sort();
                        let l: u64 = v[self.config.quorum_size() - 2].0;
                        if *committed_length < l {
                            *committed_length = l;
                            sm.take_snapshot(l);
                            self.committed_length_cvar.notify_one();
                            let v: Vec<(u64, oneshot::Sender<bool>)> =
                                s.commit_alarm.drain(..).collect();
                            for (i, ch) in v {
                                if i < l {
                                    info!("send commit alarm for {} < {}", i, l);
                                    let _ = ch.send(true);
                                } else {
                                    s.commit_alarm.push((i, ch));
                                }
                            }
                        }
                    } else {
                        if entries_len > 0 {
                            info!("received append entries ack from {} (failed)", peer);
                        }
                        assert!(prev_length > 0); // NB : If prev_length == 0 and res.term == term,
                                                  // then it is absurd to reject append entries rpc
                        s.prev_length.insert(peer, prev_length - 1);
                    }
                }
            }
        }
    }

    fn reset_to_leader(self: &Arc<Self>, guard: &mut MutexGuard<RaftState>) {
        guard.role = Role::Leader(LeaderState {
            heartbeat_handle: AbortOnDropHandle::new(task::spawn(self.clone().heartbeat_future())),
            prev_length: self
                .config
                .peers
                .iter()
                .map(|&p| (p, guard.sm.wal().len()))
                .collect(),
            match_length: self.config.peers.iter().map(|&p| (p, 0)).collect(),
            commit_alarm: Vec::new(),
        });
    }

    fn reset_to_candidate(self: &Arc<Self>, guard: &mut MutexGuard<RaftState>) {
        info!("reset to candidate");
        guard.role = Role::Candidate(CandidateState {
            election_handle: AbortOnDropHandle::new(task::spawn(self.clone().election_future())),
            timeout_handle: AbortOnDropHandle::new(task::spawn(self.clone().timeout_future())),
        });
    }

    fn reset_to_follower(
        self: &Arc<Self>,
        guard: &mut MutexGuard<RaftState>,
        current_leader: Option<&'static str>,
    ) {
        guard.role = Role::Follower(FollowerState {
            current_leader: current_leader,
            timeout_handle: AbortOnDropHandle::new(task::spawn(self.clone().timeout_future())),
        });
    }

    // receive request from web server
    pub fn user_request_thread(self: Arc<Self>, mut req_rx: mpsc::Receiver<UserRequestArgs>) {
        while let Some(args) = req_rx.blocking_recv() {
            let self_cloned = self.clone();
            task::spawn(async move { self_cloned.user_request(Request::new(args)).await });
        }
    }

    // publish committed log to web server
    pub fn publisher_thread(self: Arc<Self>, log_tx: mpsc::Sender<Entry>) {
        let mut sent_length: usize = 0;
        let mut guard = self.state.lock();
        loop {
            let committed_length = guard.committed_length as usize;
            if sent_length < committed_length {
                let entries = guard.sm.wal().as_slice()[sent_length..committed_length].to_vec();
                drop(guard);
                for entry in entries {
                    log_tx.blocking_send(entry).unwrap();
                }
                sent_length = committed_length;
                guard = self.state.lock();
            } else {
                self.committed_length_cvar.wait(&mut guard);
            }
        }
    }

    pub fn peer_thread(self: Arc<Self>, peer: &'static str) {
        let mut guard: parking_lot::lock_api::MutexGuard<'_, parking_lot::RawMutex, RaftState> =
            self.state.lock();
        'LOOP: loop {
            if let Role::Leader(s) = &guard.role {
                if s.match_length[peer] < guard.sm.wal().len() {
                    let client = guard.connections[peer].clone();
                    let prev_length = s.prev_length[peer];
                    let args = AppendEntriesArgs {
                        term: guard.persistent_state.current_term(),
                        leader_id: String::from(self.config.self_id),
                        prev_length: prev_length,
                        prev_term: guard.sm.wal().last_term_for(prev_length),
                        entries: guard.sm.wal().as_slice()
                            [prev_length as usize..prev_length as usize + 1]
                            .to_vec(),
                        committed_length: guard.committed_length,
                    };
                    drop(guard);
                    tokio::runtime::Handle::current()
                        .block_on(self.clone().append_entries_future(peer, client, args));
                    guard = self.state.lock();
                    continue 'LOOP;
                }
            }
            self.propose_cvar.wait(&mut guard);
        }
    }
}

#[tonic::async_trait]
impl RaftChat for Arc<MyRaftChat> {
    async fn append_entries(
        &self,
        request: Request<AppendEntriesArgs>,
    ) -> Result<Response<AppendEntriesRes>, Status> {
        let args: AppendEntriesArgs = request.into_inner();
        if args.entries.len() > 0 {
            info!("non empty append entries received");
        }
        let mut guard = self.state.lock();
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
        self.reset_to_follower(&mut guard, Some(leader_id));

        match guard
            .sm
            .append_entries(args.prev_length, args.prev_term, &args.entries)
        {
            None => Ok(Response::new(AppendEntriesRes {
                term: current_term,
                success: false,
            })),
            Some(compatible_length) => {
                let l = max(
                    guard.committed_length,
                    min(args.committed_length, compatible_length),
                );
                guard.committed_length = l;
                guard.sm.take_snapshot(l);
                drop(guard);
                self.committed_length_cvar.notify_one();

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
        info!("request_vote");

        let args: RequestVoteArgs = request.into_inner();
        let mut guard = self.state.lock();
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
        self.reset_to_follower(&mut guard, None);
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
        let future: Pin<
            Box<dyn Send + std::future::Future<Output = Result<Response<UserRequestRes>, Status>>>,
        > = {
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
                    match sm.state().get(&args.client_id) {
                        Some(message_id) => {
                            if message_id + 1 != args.message_id {
                                return Ok(Response::new(UserRequestRes { success: false }));
                            }
                        }
                        None => {
                            if args.message_id != 1 {
                                return Ok(Response::new(UserRequestRes { success: false }));
                            }
                        }
                    };

                    // 2. append log in wal
                    let proposed_idx = sm.propose_entry(Entry {
                        term: persistent_state.current_term(),
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

                    // 4. call commit func - notify peer threads
                    self.propose_cvar.notify_all();

                    // 5. waiting commit
                    Box::pin(async {
                        match rx.await {
                            Ok(true) => return Ok(Response::new(UserRequestRes { success: true })),
                            Ok(false) => {
                                return Ok(Response::new(UserRequestRes { success: false }))
                            }
                            Err(_) => return Ok(Response::new(UserRequestRes { success: false })),
                        }
                    })
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

                        Box::pin(async {
                            let res = client.user_request(request).await;
                            res
                        })
                    } else {
                        return Ok(Response::new(UserRequestRes { success: false }));
                    }
                }
                RaftState {
                    role: Role::Candidate(_),
                    ..
                } => {
                    return Ok(Response::new(UserRequestRes { success: false }));
                }
            }
        };

        future.await
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
    let raft_chat = Arc::new(MyRaftChat {
        config: config,
        state: Mutex::new(RaftState {
            persistent_state: PersistentState::new(persistent_state_path),
            sm: SMWrapper::new(WAL::new(wal_path)),
            committed_length: 0,
            role: Role::Follower(FollowerState {
                current_leader: None,
                timeout_handle: AbortOnDropHandle::new(task::spawn(async {})),
            }),
            connections: HashMap::new(),
        }),
        committed_length_cvar: Condvar::new(),
        propose_cvar: Condvar::new(),
    });

    for addr in raft_chat.config.peers.iter() {
        let channel: Channel = Endpoint::from_static(addr).connect_lazy();
        raft_chat
            .state
            .lock()
            .connections
            .insert(*addr, RaftChatClient::new(channel));
    }

    raft_chat.state.lock().role = Role::Follower(FollowerState {
        current_leader: None,
        timeout_handle: AbortOnDropHandle::new(task::spawn(raft_chat.clone().timeout_future())),
    });

    let raft_chat_cloned = raft_chat.clone();
    task::spawn_blocking(move || raft_chat_cloned.user_request_thread(req_rx));

    let raft_chat_cloned = raft_chat.clone();
    task::spawn_blocking(move || raft_chat_cloned.publisher_thread(log_tx));

    for peer in raft_chat.config.peers.iter().copied() {
        let raft_chat_cloned: Arc<MyRaftChat> = raft_chat.clone();
        task::spawn_blocking(move || raft_chat_cloned.peer_thread(peer));
    }

    let rpc_future = Server::builder()
        .add_service(RaftChatServer::new(raft_chat))
        .serve(serve_addr);
    task::spawn(rpc_future);
}
