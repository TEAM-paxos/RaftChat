// persistent state

use crate::RaftConfig;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize)]
struct PersistentStateElement {
    current_term: u64,
    voted_for: Option<String>,
}

pub struct PersistentState {
    current_term: u64,
    voted_for: Option<&'static str>,
    path: &'static Path,
    backup_path: &'static Path,
}

impl PersistentState {
    // If a state file exists, it loads the data; otherwise, it initializes a default state.
    pub fn new(config: &RaftConfig, path: &'static Path, backup_path: &'static Path) -> Self {
        let element = if path.exists() {
            let contents = fs::read_to_string(path).expect("Failed to read persistent state file");
            serde_json::from_str(&contents).expect("Failed to deserialize persistent state")
        } else {
            PersistentStateElement {
                current_term: 0,
                voted_for: None,
            }
        };

        let voted_for = element
            .voted_for
            .as_deref()
            .and_then(|id| config.get_peer(id));

        Self {
            current_term: element.current_term,
            voted_for,
            path: path,
            backup_path: backup_path,
        }
    }

    pub fn current_term(&self) -> u64 {
        self.current_term
    }

    pub fn voted_for(&self) -> Option<&'static str> {
        self.voted_for
    }

    // Helper function to save the state atomically by backing it up and linking to the main file.
    fn save_state(&self) {
        let state = PersistentStateElement {
            current_term: self.current_term,
            voted_for: self.voted_for.map(String::from),
        };

        // Create a backup file
        let backup_file =
            File::create(&self.backup_path).expect("Failed to create the backup file.");
        serde_json::to_writer(backup_file, &state)
            .expect("Failed to write the backup state to file.");

        // Create a hard link from the backup file to the main file
        let original_permissions = fs::metadata(&self.path)
            .map(|metadata| metadata.permissions())
            .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o644));

        let status = Command::new("ln")
            .arg(&self.backup_path)
            .arg(&self.path)
            .status()
            .expect("Failed to execute the ln command.");

        if status.success() {
            // Remove the backup file
            let _ = fs::remove_file(&self.backup_path);
            // Restore the original file permissions
            fs::set_permissions(&self.path, original_permissions)
                .expect("Failed to restore the original file permissions.");
        } else {
            panic!();
        }
    }

    // Become candidate
    pub fn start_election(&mut self, self_id: &'static str) {
        self.current_term = self.current_term + 1;
        self.voted_for = Some(self_id);
        self.save_state();
    }

    // Update the current term.
    // return (current_term, ok)
    //   current_term : term number after update
    //   ok : true if new_term was not outdated
    pub fn update_term(&mut self, new_term: u64) -> (u64, bool) {
        if new_term < self.current_term {
            (self.current_term, false)
        } else {
            self.current_term = new_term;
            self.voted_for = None;
            self.save_state();
            (self.current_term, true)
        }
    }

    // Attempts to vote for a candidate.
    // return ok
    //   ok : true if candidate received a vote
    pub fn try_vote(&mut self, candidate: &'static str) -> bool {
        match &self.voted_for {
            None => {
                self.voted_for = Some(candidate);
                self.save_state();
                true
            }
            Some(recipient) => *recipient == candidate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};
    use tempfile::tempdir;

    // Creates a dummy RaftConfig for testing purposes.
    fn create_raft_config() -> RaftConfig {
        RaftConfig {
            serve_addr: SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8080),
            self_id: "1",
            peers: vec!["1", "2", "3", "4", "5"],
            election_duration: (3000, 4000), // raft paper: 150ms ~ 300ms
            heartbeat_duration: tokio::time::Duration::from_millis(250),
            persistent_state_path: Path::new("TODO : path to persistent_state"),
            persistent_state_backup_path: Path::new("TODO : path to persistent_state_backup"),
            wal_path: Path::new("TODO : path to wal"),
        }
    }

    #[test]
    fn test_new_with_existing_file() {
        let dir = tempdir().expect("Failed to create a temporary directory.");
        let path = dir.path().join("state.json");
        let backup_path = dir.path().join("backup_state.json");

        // Write an initial state to the file
        let initial_state = PersistentStateElement {
            current_term: 5,
            voted_for: Some("1".to_string()),
        };
        fs::write(&path, serde_json::to_string(&initial_state).unwrap())
            .expect("Failed to write the initial state.");

        let config = create_raft_config();
        let state = PersistentState::new(&config, &path, &backup_path);

        // Verify that the state was loaded correctly
        assert_eq!(state.current_term(), 5);
        assert_eq!(state.voted_for(), Some("1"));
    }

    #[test]
    fn test_new_with_missing_file() {
        let dir = tempdir().expect("Failed to create a temporary directory.");
        let path = dir.path().join("state.json");
        let backup_path = dir.path().join("backup_state.json");

        let config = create_raft_config();
        let state = PersistentState::new(&config, &path, &backup_path);

        // Verify that a default state is created
        assert_eq!(state.current_term(), 0);
        assert_eq!(state.voted_for(), None);
    }

    #[test]
    fn test_update_term_atomic() {
        let dir = tempdir().expect("Failed to create a temporary directory.");
        let path = dir.path().join("state.json");
        let backup_path = dir.path().join("backup_state.json");

        let config = create_raft_config();
        let mut state = PersistentState::new(&config, &path, &backup_path);

        let (new_term, ok) = state.update_term(10);

        assert!(ok);
        assert_eq!(new_term, 10);
        assert_eq!(state.current_term(), 10);

        let contents = fs::read_to_string(&path).expect("Failed to read the state file.");
        let persisted_state: PersistentStateElement =
            serde_json::from_str(&contents).expect("Failed to decode the state file.");
        assert_eq!(persisted_state.current_term, 10);
        assert_eq!(persisted_state.voted_for, None);
    }

    #[test]
    fn test_try_vote_atomic() {
        let dir = tempdir().expect("Failed to create a temporary directory.");
        let path = dir.path().join("state.json");
        let backup_path = dir.path().join("backup_state.json");

        let config = create_raft_config();
        let mut state = PersistentState::new(&config, &path, &backup_path);

        let ok = state.try_vote("2");

        assert!(ok);
        assert_eq!(state.voted_for(), Some("2"));

        let contents = fs::read_to_string(&path).expect("Failed to read the state file.");
        let persisted_state: PersistentStateElement =
            serde_json::from_str(&contents).expect("Failed to decode the state file.");
        assert_eq!(persisted_state.current_term, state.current_term());
        assert_eq!(persisted_state.voted_for, Some("2".to_string()));
    }
}
