use crate::raftchat_tonic::{Entry, UserRequestArgs};
use tokio::sync::mpsc;

// Interface
pub trait DB {
    fn make_channel(
        &self,
        id: u64,
        peers: Vec<String>,
    ) -> (mpsc::Receiver<Entry>, mpsc::Sender<UserRequestArgs>);
}
