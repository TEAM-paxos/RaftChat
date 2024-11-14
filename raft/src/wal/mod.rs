// Write-Ahead-Log

use crate::raftchat_tonic::Entry;
use atomic_write_file::AtomicWriteFile;
use std::path::Path;

pub struct WAL {
    cache: Vec<Entry>,
}

impl WAL {
    pub fn new(path: &Path) -> WAL {
        WAL { cache: vec![] }
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
        if self.cache.len() < prev_length as usize {
            None
        } else if (prev_length == 0) || (self.cache[prev_length as usize - 1].term == prev_term) {
            // calculate action to perform
            let action: Option<(usize, &[Entry])> = {
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
                                break Some((l, entries));
                            }
                        }
                        [] => break None,
                    }
                }
            };

            // apply action to the log
            if let Some((l, entries)) = action {
                self.cache.truncate(l);
                self.cache.extend_from_slice(entries);
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

    use crate::raftchat_tonic::{Command, Entry};
    use crate::wal::WAL;

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
        let mut state = WAL {
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
        let mut state = WAL {
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
