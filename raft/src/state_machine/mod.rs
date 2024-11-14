use crate::raftchat_tonic::{Command, Entry};
use crate::wal::WAL;
use std::collections::HashMap;

trait StateMachine {
    fn new() -> Self;
    fn act(&mut self, cmd: &Command);
}

pub struct SMWrapper<S> {
    wal: WAL,
    state: S,
    snapshot: (u64, S),
}

pub struct UserMessageIdMap {
    table: HashMap<String, u64>,
}

impl StateMachine for UserMessageIdMap {
    fn new() -> Self {
        unimplemented!()
    }

    fn act(&mut self, cmd: &Command) {
        unimplemented!()
    }
}

impl<S> SMWrapper<S>
where
    S: StateMachine,
{
    pub fn new(wal: WAL) -> Self {
        SMWrapper {
            wal: wal,
            state: StateMachine::new(),
            snapshot: (0, StateMachine::new()),
        }
    }

    pub fn take_snapshot(&mut self, len: u64) {
        unimplemented!()
    }

    pub fn append_entries(
        &mut self,
        prev_length: u64,
        prev_term: u64,
        entries: &[Entry],
    ) -> Option<u64> {
        unimplemented!()
    }
}
