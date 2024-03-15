use std::borrow::Borrow;
use std::ops::MulAssign;

use byteorder::{ByteOrder, LittleEndian};
use hyper::body::Bytes;
use hyper::Error;
use rkyv::ser::serializers;
use rkyv::ser::{serializers::AllocSerializer, Serializer};
use rkyv::util::AlignedVec;
use rkyv::{de, Archive, Deserialize, Serialize, Infallible};

use super::network;

#[derive(Archive, Deserialize, Serialize, Debug)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug))]
pub struct Msg{
    pub From: String,
    pub To : String,
    pub Content : String,
}


const  BROKEN_MSG : &str = "broken message";

pub fn serialize_msg(msg:& Msg) -> AlignedVec {
    let mut serializer: AllocSerializer<0> = AllocSerializer::default();
    serializer.serialize_value(msg).unwrap();
    let mut bytes: AlignedVec = serializer.into_serializer().into_inner();
    bytes
}

pub fn desrialize_msg(buf : Bytes) -> Msg {
    println!("{}", buf.len());
    let archived = rkyv::check_archived_root::<Msg>(&buf).unwrap();
    let deserialized = archived.deserialize(&mut Infallible);
    match deserialized {
        Ok(data) => data,
        Err(error) => Msg{
            From : BROKEN_MSG.to_string(),
            To  : BROKEN_MSG.to_string(),
            Content : BROKEN_MSG.to_string(),
        }
    }
}


#[cfg(test)]
mod tests_serialize{
    use hyper::body::Buf;

    use super::*;
    #[test] 
        
    fn serialize_deserialize_check(){
        let temp = Msg{ 
            From: "from".to_string(), 
            To: "to".to_string(),
            Content: "hi".to_string() };
        
        let binding: AlignedVec = serialize_msg(&temp);
        let mut bytes = binding.as_slice();
        let res = desrialize_msg(bytes.copy_to_bytes(bytes.len()));

       assert_eq!(temp.From, res.From)
    }
}