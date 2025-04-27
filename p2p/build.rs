use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_build::compile_protos("proto/kademlia.proto")?;
    tonic_build::compile_protos("proto/join.proto")?;

    Ok(())
}
