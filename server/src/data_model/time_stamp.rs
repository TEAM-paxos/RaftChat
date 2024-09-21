use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeStamp{
    uid : String,
    seq : u64,
}