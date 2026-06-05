fn main() {
    connectrpc_build::Config::new()
        .files(&["proto/maps/v1/maps.proto"])
        .includes(&["proto"])
        .include_file("_connectrpc.rs")
        .compile()
        .expect("failed to compile maps.proto");
}
