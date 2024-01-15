use byteorder::{ByteOrder, LittleEndian};
use rkyv::ser::{serializers::AllocSerializer, Serializer};
use rkyv::util::AlignedVec;
use rkyv::{Archive, Deserialize, Serialize};
use std::env;
use std::io::Read;
use std::io::Write;
use std::net::*;

#[derive(Archive, Deserialize, Serialize, Debug)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug))]
struct Msg {
    text: String,
}

const MY_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50000);

fn serialize_msg(msg: Msg) -> AlignedVec {
    let mut serializer: AllocSerializer<0> = AllocSerializer::default();
    serializer.write(&[0; 8]).unwrap();
    serializer.serialize_value(&msg).unwrap();
    let mut bytes: AlignedVec = serializer.into_serializer().into_inner();
    let len = bytes.len() as u64;
    LittleEndian::write_u64(bytes.as_mut_slice(), len);
    bytes
}

fn when_leader() {
    println!("leader");

    let listener = TcpListener::bind(MY_ADDR).unwrap();
    let (mut stream, addr) = listener.accept().unwrap();
    println!("new follower: {addr:?}");

    let msg = Msg {
        text: "Hello!".to_string(),
    };
    println!("MSG : {msg:?}");
    let bytes = serialize_msg(msg);
    println!("BYTES : {bytes:?}");
    stream.write_all(bytes.as_slice()).unwrap();
}

fn when_follower() {
    println!("follower");

    let mut stream = TcpStream::connect(MY_ADDR).unwrap();
    println!("connected!");

    let mut buf: [u8; 1024] = [0; 1024];
    stream.read_exact(&mut buf[..8]).unwrap();
    let len = LittleEndian::read_u64(&buf) as usize;
    stream.read_exact(&mut buf[8..len]).unwrap();
    let archived = rkyv::check_archived_root::<Msg>(&buf[..len]).unwrap();
    // let value : Msg = archived.deserialize(&mut rkyv::Infallible).unwrap();
    let bytes = &buf[..len];
    println!("BYTES : {bytes:?}");
    println!("{archived:?}");
}

fn main() {
    let role: String = env::args().nth(1).unwrap();
    match role.as_str() {
        "leader" => when_leader(),
        "follower" => when_follower(),
        _ => panic!(),
    }
}
