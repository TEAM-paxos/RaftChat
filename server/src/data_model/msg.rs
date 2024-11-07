use chrono::{DateTime, Utc};
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientMsg {
    committed_index: u64,
    messages: Vec<Msg>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Msg {
    id: String,
    user_id: String,
    content: String,
    time: DateTime<Utc>,
    time_stamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogData {
    content: String,
    time: DateTime<Utc>,
    user_id: String,
}

impl LogData {
    pub fn new(user_id: String, content: String, time: DateTime<Utc>) -> Self {
        LogData {
            content: content,
            time: time,
            user_id: user_id,
        }
    }
    pub fn get_content(&self) -> String {
        self.content.clone()
    }
    pub fn get_time(&self) -> DateTime<Utc> {
        self.time
    }
    pub fn get_user_id(&self) -> String {
        self.user_id.clone()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerMsg {
    committed_index: u64,
    message: Msg,
}

impl ClientMsg {
    pub fn get_messages(&self) -> &Vec<Msg> {
        &self.messages
    }

    pub fn get_committed_index(&self) -> u64 {
        self.committed_index
    }
}

impl Msg {
    pub fn new(
        id: String,
        user_id: String,
        content: String,
        time: DateTime<Utc>,
        time_stamp: u64,
    ) -> Self {
        Msg {
            id: id,
            user_id: user_id,
            content: content,
            time: time,
            time_stamp: time_stamp,
        }
    }
    pub fn get_uid(&self) -> String {
        self.user_id.clone()
    }
    pub fn get_content(&self) -> String {
        self.content.clone()
    }
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
    pub fn get_time(&self) -> DateTime<Utc> {
        self.time.clone()
    }
    pub fn get_time_stamp(&self) -> u64 {
        self.time_stamp
    }
}

impl ServerMsg {
    pub fn new(committed_index: u64, msg: Msg) -> Self {
        ServerMsg {
            committed_index: committed_index,
            message: msg,
        }
    }
}
