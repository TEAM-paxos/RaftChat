use tokio::sync::mpsc;

// Interface
pub trait DB {
    fn make_channel(
        &self,
        id: u64,
        peers: Vec<String>,
    ) -> (mpsc::Receiver<Commit>, mpsc::Sender<UserRequest>);
}

// DB will return this struct.
pub struct Commit {
    // committed index
    index: u64,
    data: Vec<u8>,
}

impl Commit {
    pub fn new(_index: u64, _data: Vec<u8>) -> Commit {
        Commit {
            index: _index,
            data: _data,
        }
    }

    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn get_index(&self) -> u64 {
        self.index
    }
}

// DB receive UserRequest struct form
// client and return Commit struct.
#[derive(Debug)]
pub struct UserRequest {
    id: String, // id is assumed to be unique for each client
    timestamp: u64,
    data: Vec<u8>,
}

impl UserRequest {
    pub fn new(_id: String, _timestamp: u64, _data: Vec<u8>) -> UserRequest {
        UserRequest {
            id: _id,
            timestamp: _timestamp,
            data: _data,
        }
    }

    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }
    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn get_id_ref(&self) -> &String {
        &self.id
    }
}
