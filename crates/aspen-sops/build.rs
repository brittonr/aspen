//! Build script: compile SOPS KeyService proto when the `keyservice` feature is enabled.

fn main() {
    #[cfg(feature = "keyservice")]
    {
        tonic_build::configure()
            .build_client(false) // We only need the server
            .compile_protos(&["proto/keyservice.proto"], &["proto/"])
            .expect("failed to compile SOPS keyservice proto");
    }
}
