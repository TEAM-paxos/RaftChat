
mod crdt_list{}
mod time{ }
mod network;

use crate::generated::Message::Message;

pub async fn run_receiver(port : u16){
    let _res = network::run_receiver(port).await.unwrap();
}

pub async fn send_msg( dest_ip:&str, dest_port: u16, contents: String)-> Result<(), &'static str>{
    if contents.len() > 1000 {
       return Err("his");
    }
    
    let msg = Message{
        from: "hi",
        to  : "you",
        msg: &contents
    };
    
    network::run_sender(&dest_ip, dest_port, "echo",msg).await.unwrap();
    Ok(())
}


#[cfg(test)] 
mod tests{
    use crate::crdt_chat;
    #[test]    
    
    fn it_works(){
     
    }
}