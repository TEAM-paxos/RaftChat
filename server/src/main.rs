use std::env;
use std::net::*;

const MY_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

fn when_leader() {
    println!("leader");

    let listener = TcpListener::bind(MY_ADDR).unwrap();
    let (_stream, addr) = listener.accept().unwrap();
    println!("new follower: {addr:?}");
}

fn when_follower() {
    println!("follower");

    let mut _stream = TcpStream::connect(MY_ADDR).unwrap();
    println!("connected!")
}

fn main() {
    let role: String = env::args().nth(1).unwrap();
    match role.as_str() {
        "leader" => when_leader(),
        "follower" => when_follower(),
        _ => panic!(),
    }
}
