// persistent state

use crate::raftchat_tonic::Entry;
use atomic_write_file::AtomicWriteFile;
use std::path::Path;

pub struct PersistentState {
    // These data must be stored on persistent storage
    current_term: u64,
    voted_for: Option<&'static str>,
    log: Vec<Entry>,
}

impl PersistentState {
    pub fn new(path: &Path) -> PersistentState {
        // TODO : initialize with data from path
        PersistentState {
            current_term: 0,
            voted_for: None,
            log: vec![],
        }
    }

    pub fn get_current_term(&self) -> u64 {
        self.current_term
    }

    pub fn get_voted_for(&self) -> Option<&'static str> {
        self.voted_for
    }

    // Dummy implementation
    pub fn increment_term(&mut self) {
        self.current_term = self.current_term + 1;
    }

    // Dummy implementation.
    // return (current_term, ok)
    //   current_term : term number after update
    //   ok : true if the given term was not outdated
    // return true  if updated
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
    // return (current_term, ok)
    //   current_term : term number after update
    //   ok : true if candidate received a vote
    pub fn try_vote(&mut self, new_term: u64, candidate: &'static str) -> (u64, bool) {
        if new_term < self.current_term {
            (self.current_term, false)
        } else if self.current_term < new_term {
            self.current_term = new_term;
            self.voted_for = Some(candidate);
            (self.current_term, true)
        } else {
            match &self.voted_for {
                None => {
                    self.voted_for = Some(candidate);
                    (self.current_term, true)
                }
                Some(recipient) => (self.current_term, *recipient == candidate),
            }
        }
    }

    // Dummy implementation.
    // New entry must be written on stable storage before exiting the function
    // return None    if not matched
    // return Some(l) if matched, where l is the length of guaranteed common prefix of
    //                      the log of the leader and the log of this node.
    pub fn append_entries(
        &mut self,
        prev_length: u64,
        prev_term: u64,
        entries: &[Entry],
    ) -> Option<u64> {
        if self.log.len() < prev_length as usize {
            None
        } else if (prev_length == 0) || (self.log[prev_length as usize - 1].term == prev_term) {
            // calculate action to perform
            let action: Option<(usize, &[Entry])> = {
                let mut l: usize = prev_length as usize;
                let mut entries: &[Entry] = entries;
                loop {
                    match entries {
                        [entries_head, entries_tail @ ..] => {
                            if (l < self.log.len()) && (self.log[l].term == entries_head.term) {
                                l += 1;
                                entries = entries_tail;
                                continue;
                            } else {
                                break Some((l, entries));
                            }
                        }
                        [] => break None,
                    }
                }
            };

            // apply action to the log
            if let Some((l, entries)) = action {
                self.log.truncate(l);
                self.log.extend_from_slice(entries);
            };

            // compatible length
            Some(prev_length + entries.len() as u64)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::persistent_state::PersistentState;
    use crate::raftchat_tonic::{Command, Entry};

    fn mk_entry(term: u64) -> Entry {
        Entry {
            term: term,
            command: Some(Command {
                client_id: "client1".to_string(),
                message_id: 0,
                data: vec![],
            }),
        }
    }

    #[test]
    #[rustfmt::skip]
    fn case_append() {
        let mut state = PersistentState {
            current_term: 0,
            voted_for: None,
            log: vec![
                mk_entry(1),
                mk_entry(2),
                mk_entry(3),
            ],
        };

        assert_eq!(
            state.append_entries(
                2,
                2,
                &[
                    mk_entry(3),
                    mk_entry(4),
                    mk_entry(5),
                ]
            ),
            Some(5)
        );
        assert_eq!(
            state.log,
            vec![
                mk_entry(1),
                mk_entry(2),
                mk_entry(3),
                mk_entry(4),
                mk_entry(5),
            ]
        );
    }

    #[test]
    #[rustfmt::skip]
    fn case_rewrite() {
        let mut state = PersistentState {
            current_term: 0,
            voted_for: None,
            log: vec![
                mk_entry(1),
                mk_entry(2),
                mk_entry(3),
            ],
        };

        assert_eq!(
            state.append_entries(
                2,
                2,
                &[
                    mk_entry(4),
                    mk_entry(5),
                ]
            ),
            Some(4)
        );
        assert_eq!(
            state.log,
            vec![
                mk_entry(1),
                mk_entry(2),
                mk_entry(4),
                mk_entry(5),
            ]
        );
    }

    #[test]
    #[rustfmt::skip]
    fn case_subsumed() {
        let mut state = PersistentState {
            current_term: 0,
            voted_for: None,
            log: vec![
                mk_entry(1),
                mk_entry(2),
                mk_entry(3),
            ],
        };

        assert_eq!(
            state.append_entries(
                1,
                1,
                &[
                    mk_entry(2),
                ]
            ),
            Some(2)
        );
        assert_eq!(
            state.log,
            vec![
                mk_entry(1),
                mk_entry(2),
                mk_entry(3),
            ]
        );
    }
}
