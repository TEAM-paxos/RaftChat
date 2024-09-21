use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

// this is a right way to import a module from the same crate?
use crate::data_model::time_stamp::TimeStamp;

#[derive(Serialize, Deserialize, Clone)]
pub struct ClientMsg {
    uid: String,
    data: String,
    #[serde(with = "ts_seconds")]
    time: DateTime<Utc>,
    // commited index that client has received
    cur_seq: u64,
    // timestamp of the message when it was sent
    timestamp: TimeStamp,
}

#[derive(Serialize)]
pub struct ServerMsg {
    commited_index: u64,
    data: Vec<String>,
}

impl ClientMsg {
    pub fn get_uid(&self) -> &String {
        &self.uid
    }
    pub fn get_data(&self) -> String {
        self.data.clone()
    }
}
