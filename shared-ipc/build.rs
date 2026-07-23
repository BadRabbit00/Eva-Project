fn main() {
    // Compile gRPC protos
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/eva.proto"], &["proto"])
        .unwrap_or_else(|e| panic!("Failed to compile protos: {}", e));
}
