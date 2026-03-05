## 1. Audit Existing Implementations

- [x] 1.1 Verify `aspen-snix` IrohBlobService compiles and trait methods are complete (has/open_read/open_write/chunks)
- [x] 1.2 Verify `aspen-snix` RaftDirectoryService compiles and trait methods are complete (get/put/get_recursive/put_multiple_start)
- [x] 1.3 Verify `aspen-snix` RaftPathInfoService compiles and trait methods are complete (get/put/list)
- [x] 1.4 Verify `aspen-castore` IrpcBlobService and IrpcDirectoryService compile with irpc transport
- [x] 1.5 Check RPC handler wiring: SnixHandler registered via submit_handler_factory! in aspen-nix-handler at priority 800

## 2. Wire CastoreServer into Iroh Router

- [x] 2.1 Add `castore()` method to RouterBuilder in aspen-cluster for CASTORE_ALPN registration
- [x] 2.2 Wire CastoreServer in setup_router (aspen-node binary) with IrohBlobService + RaftDirectoryService, gated on `snix` feature
- [x] 2.3 Add `snix` feature to root Cargo.toml (deps: aspen-snix, aspen-castore, aspen-cluster/snix; blob implied)
- [x] 2.4 Make RaftDirectoryService/RaftPathInfoService accept `?Sized` K for `dyn KeyValueStore` usage
- [x] 2.5 Remove unnecessary `Clone` bound on IrohBlobService (store is Arc'd internally)
- [ ] 2.6 Verify snix-store can connect via irpc (IrpcBlobService/IrpcDirectoryService) to a running node

## 3. Add gRPC Bridge for snix-store CLI Compatibility

- [ ] 3.1 Add a gRPC listener to the node (or a sidecar binary) that translates snix's gRPC protocol to irpc/Raft KV calls — this lets `snix-store virtiofs --blob-service-addr grpc+unix:...` connect to Aspen
- [ ] 3.2 Alternatively: register Aspen backends directly in snix's composition registry with a custom URL scheme (e.g., `aspen+irpc://`)
- [ ] 3.3 Test: `snix-store virtiofs` connects to the bridge and serves /nix/store paths from Aspen

## 4. Integration Test: End-to-End NAR Round-Trip

- [ ] 4.1 Write a test that stores a NAR via ingest_nar_and_hash through the Aspen backends and retrieves it via PathInfoService
- [ ] 4.2 Write a test that verifies BlobService round-trip (write blob, read back, verify BLAKE3 digest)
- [ ] 4.3 Write a test that verifies DirectoryService round-trip (put directory tree, get_recursive, verify structure)

## 5. Connect snix-store VirtioFS to Aspen Backends

- [ ] 5.1 Update snix-boot runVM script to accept ASPEN_TICKET env var for connecting snix-store to cluster
- [ ] 5.2 Test: boot snix microVM, verify /nix/store lists paths previously stored via CI pipeline

## 6. Verify and Clean Up

- [ ] 6.1 Run `cargo clippy --workspace -- --deny warnings` clean
- [ ] 6.2 Run full test suite — all passing
- [ ] 6.3 Update napkin with corrections discovered during implementation
