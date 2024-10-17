use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ClientMsg {
    uid: String,
    data: String,
    #[serde(with = "ts_seconds")]
    time: DateTime<Utc>,
    // commited index that client has received
    cur_seq: u64,
    // timestamp of the message when it was sent
    timestamp: u64,
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
    pub fn get_time(&self) -> DateTime<Utc> {
        self.time.clone()
    }
    pub fn get_cur_seq(&self) -> u64 {
        self.cur_seq
    }
    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }
}
