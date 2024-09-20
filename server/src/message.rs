use std::time;

#[derive(Debug, Clone, Default)]
struct Msg{
    id : String,
    data : String,
    time : std::time,
    cur_seq : u64,   
}