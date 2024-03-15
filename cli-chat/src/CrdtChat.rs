
mod CRDTList{}
mod Time{ }
mod network;
pub mod serialize;

#[tokio::main]
async fn run_receiver(){
    let res = network::run_receiver().await;
}

#[cfg(test)] 
mod tests{
    use crate::CrdtChat;
    #[test]    
    
    fn it_works(){
        CrdtChat::run_receiver()
    }
}