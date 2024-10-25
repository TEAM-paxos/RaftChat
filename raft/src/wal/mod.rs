// Write-Ahead-Log
// WAL is a write-ahead log that is used to persist the state of the Raft log to disk.

use crate::raftchat::{Command, Entry};

pub struct WAL {
    data: Vec<Entry>,
}

impl WAL {
    async fn get_ref(&self) -> &[Entry] {
        return &self.data;
    }

    // Dummy implementation.
    // New entry must be written on stable storage before exiting the function
    // return true if succeed
    pub async fn append_entries(
        &mut self,
        prev_length: u64,
        prev_term: u64,
        entries: &[Entry],
    ) -> bool {
        if self.data.len() < prev_length as usize {
            false
        } else if (prev_length == 0) || (self.data[prev_length as usize - 1].term == prev_term) {
            let mut index: usize = prev_length as usize;
            let mut entries: &[Entry] = entries;
            // loop invariant : index <= self.data.len()
            while let [entry, entries_suffix @ ..] = entries {
                if index < self.data.len() {
                    if self.data[index].term == entry.term {
                        index += 1;
                        entries = entries_suffix;
                        continue;
                    } else {
                        self.data.truncate(index);
                        self.data.extend_from_slice(entries);
                        break;
                    }
                } else {
                    self.data.extend_from_slice(entries);
                    break;
                }
            }
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::raftchat::{Command, Entry};
    use crate::WAL;

    #[tokio::test]
    #[rustfmt::skip]
    async fn case_append() {
        let mut log = WAL {
            data: vec![
                Entry { term: 1, command: None, },
                Entry { term: 2, command: None, },
                Entry { term: 3, command: None, },
            ],
        };

        assert_eq!(
            log.append_entries(
                3,
                3,
                &[
                    Entry { term: 4, command: None, },
                    Entry { term: 5, command: None, },
                ]
            )
            .await,
            true
        );
        assert_eq!(
            log.data,
            vec![
                Entry { term: 1, command: None, },
                Entry { term: 2, command: None, },
                Entry { term: 3, command: None, },
                Entry { term: 4, command: None, },
                Entry { term: 5, command: None, },
            ]
        );
    }

    #[tokio::test]
    #[rustfmt::skip]
    async fn case_rewrite() {
        let mut log = WAL {
            data: vec![
                Entry { term: 1, command: None, },
                Entry { term: 2, command: None, },
                Entry { term: 3, command: None, },
            ],
        };

        assert_eq!(
            log.append_entries(
                2,
                2,
                &[
                    Entry { term: 4, command: None, },
                    Entry { term: 5, command: None, },
                ]
            )
            .await,
            true
        );
        assert_eq!(
            log.data,
            vec![
                Entry { term: 1, command: None, },
                Entry { term: 2, command: None, },
                Entry { term: 4, command: None, },
                Entry { term: 5, command: None, },
            ]
        );
    }

    #[tokio::test]
    #[rustfmt::skip]
    async fn case_subsumed() {
        let mut log = WAL {
            data: vec![
                Entry { term: 1, command: None, },
                Entry { term: 2, command: None, },
                Entry { term: 3, command: None, },
            ],
        };

        assert_eq!(
            log.append_entries(
                1,
                1,
                &[
                    Entry { term: 2, command: None, },
                ]
            )
            .await,
            true
        );
        assert_eq!(
            log.data,
            vec![
                Entry { term: 1, command: None, },
                Entry { term: 2, command: None, },
                Entry { term: 3, command: None, },
            ]
        );
    }
}
