fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/raftchat_test_proto3_optional.proto")?;
    Ok(())
}
