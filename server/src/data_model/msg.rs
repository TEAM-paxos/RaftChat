use std::clone;

use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

// {
//     "committed_index": 0,
//     "messages": [
//       {
//         "id": "unique_id",
//         "user_id": "default userId",
//         "content": "asdfasdf",
//         "time": 1728305829,
//         "time_stamp": 1
//       }
//     ]
// }

#[derive(Serialize, Deserialize, Clone)]
pub struct ClientMsg {
    committed_index: u64,
    messages: Vec<Msg>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Msg{
    id: String, 
    user_id: String,
    content: String,
    time: DateTime<Utc>,
    time_stamp: u64
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerMsg {
    commited_index: u64,
    data: Vec<String>,
}

impl ClientMsg{
    pub fn get_messages(&self) -> &Vec<Msg> {
        &self.messages
    }

    pub fn get_committed_index(&self) -> u64 {
        self.committed_index
    }
}

impl Msg {
    pub fn get_uid(&self) -> &String {
        &self.user_id
    }
    pub fn get_content(&self) -> String {
        self.content.clone()
    }
}