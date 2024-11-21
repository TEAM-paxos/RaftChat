// Write-Ahead-Log

use crate::raftchat_tonic::Entry;
use atomic_write_file::AtomicWriteFile;
use std::path::Path;

pub struct WAL {
    cache: Vec<Entry>,
}

#[derive(Debug, PartialEq)]
pub enum Action<'a> {
    Id,
    Update(u64, &'a [Entry]),
}

impl WAL {
    pub fn new(path: &Path) -> WAL {
        // TODO : init with path
        WAL { cache: vec![] }
    }

    pub fn len(&self) -> u64 {
        self.cache.len() as u64
    }

    pub fn last_term(&self) -> u64 {
        match self.cache.last() {
            Some(entry) => entry.term,
            None => 0,
        }
    }

    pub fn last_term_for(&self, len: u64) -> u64 {
        match self.cache[..len as usize].last() {
            Some(entry) => entry.term,
            None => 0,
        }
    }

    pub fn as_slice(&self) -> &[Entry] {
        &self.cache
    }

    // Dummy implementation.
    // New entry must be written on stable storage before exiting the function
    // return None    if not matched
    // return Some(l) if matched, where l is the length of guaranteed common prefix of
    //                      the log of the leader and the log of this node.
    pub fn append_entries<'a>(
        &mut self,
        prev_length: u64,
        prev_term: u64,
        entries: &'a [Entry],
    ) -> Option<Action<'a>> {
        if self.cache.len() < prev_length as usize {
            None
        } else if (prev_length == 0) || (self.cache[prev_length as usize - 1].term == prev_term) {
            // calculate action to perform
            let action: Action = {
                let mut l: usize = prev_length as usize;
                let mut entries: &[Entry] = entries;
                loop {
                    match entries {
                        [entries_head, entries_tail @ ..] => {
                            if (l < self.cache.len()) && (self.cache[l].term == entries_head.term) {
                                l += 1;
                                entries = entries_tail;
                                continue;
                            } else {
                                break Action::Update(l as u64, entries);
                            }
                        }
                        [] => break Action::Id,
                    }
                }
            };

            // apply action to the log
            if let Action::Update(l, entries) = action {
                self.cache.truncate(l as usize);
                self.cache.extend_from_slice(entries);
            };

            // compatible length
            Some(action)
        } else {
            None
        }
    }

    pub fn propose_entry(&mut self, entry: Entry) -> u64 {
        self.cache.push(entry);
        return self.cache.len() as u64 - 1;
    }
}

#[cfg(test)]
mod tests {

    use crate::raftchat_tonic::{Command, Entry};
    use crate::wal::{Action, WAL};

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
        let mut state = WAL {
            cache: vec![
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
            Some(Action::Update(3, &[mk_entry(4), mk_entry(5)]))
        );
        assert_eq!(
            state.cache,
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
        let mut state = WAL {
            cache: vec![
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
            Some(Action::Update(2, &[mk_entry(4), mk_entry(5)]))
        );
        assert_eq!(
            state.cache,
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
        let mut state = WAL {
            cache: vec![
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
            Some(Action::Id)
        );
        assert_eq!(
            state.cache,
            vec![
                mk_entry(1),
                mk_entry(2),
                mk_entry(3),
            ]
        );
    }
}
