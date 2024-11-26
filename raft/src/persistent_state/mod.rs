// persistent state

use crate::RaftConfig;
use atomic_write_file::AtomicWriteFile;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]

pub struct PersistentStateElement {
    current_term: u64,
    voted_for: Option<String>,
}

pub struct PersistentState {
    current_term: u64,
    voted_for: Option<&'static str>,
    path: PathBuf,
    backup_path: PathBuf,
}

impl PersistentState {
    pub fn new(config: &RaftConfig, path: &Path, backup_path: &Path) -> PersistentState {
        let element: PersistentStateElement = if path.exists() {
            let mut file = fs::File::open(path).expect("Failed to open persistent state file");
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Failed to read persistent state file");
            serde_json::from_str(&contents).expect("Failed to deserialize persistent state")
        } else {
            PersistentStateElement {
                current_term: 0,
                voted_for: None,
            }
        };

        PersistentState {
            current_term: element.current_term,
            voted_for: element.voted_for.as_deref(),
            path: path.to_path_buf(),
            backup_path: backup_path.to_path_buf(),
        }
    }

    pub fn current_term(&self) -> u64 {
        self.current_term
    }

    pub fn voted_for(&self) -> Option<&'static str> {
        self.voted_for
    }

    fn save(&self, path: &Path) {
        let serialized = serde_json::to_string(&PersistentStateElement {
            current_term: self.current_term,
            voted_for: self.voted_for.map(|s| s.to_string()),
        });

        // TODO : save serialized data to path
    }

    // Dummy implementation
    pub fn start_election(&mut self, self_id: &'static str) {
        self.current_term = self.current_term + 1;
        self.voted_for = Some(self_id);
    }

    // Dummy implementation.
    // return (current_term, ok)
    //   current_term : term number after update
    //   ok : true if the given term was not outdated
    pub fn update_term(&mut self, new_term: u64) -> (u64, bool) {
        if new_term < self.current_term {
            (self.current_term, false)
        } else {
            // Warning : this two updates must be committed simultaneously
            self.current_term = new_term;
            self.voted_for = None;
            (self.current_term, true)
        }
    }

    // Dummy implementation
    // return ok
    //   ok : true if candidate received a vote
    pub fn try_vote(&mut self, candidate: &'static str) -> bool {
        match &self.voted_for {
            None => {
                self.voted_for = Some(candidate);
                true
            }
            Some(recipient) => *recipient == candidate,
        }
    }
}
