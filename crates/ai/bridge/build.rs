//! Compiles the contract, `proto/murabito.proto`, into Rust at build time, with a
//! protobuf compiler that is itself a crate, so no `protoc` need be installed. The
//! generated module lands in `OUT_DIR` and `lib.rs` includes it.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/murabito.proto");
    let descriptors = protox::compile(["proto/murabito.proto"], ["proto"])?;
    prost_build::Config::new().compile_fds(descriptors)?;
    Ok(())
}
