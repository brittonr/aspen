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

- [x] 3.1 Create `aspen-snix-bridge` crate — standalone binary that serves snix gRPC (BlobService, DirectoryService, PathInfoService) on a Unix socket, backed by Aspen's IrohBlobService + RaftDirectoryService + RaftPathInfoService
- [x] 3.2 Uses snix's GRPCBlobServiceWrapper/GRPCDirectoryServiceWrapper/GRPCPathInfoServiceWrapper + SimpleRenderer for NAR calculation
- [x] 3.3 Binary runs and serves (verified: starts, binds socket, accepts connections)
- [ ] 3.4 Add --ticket flag for live cluster connection (currently uses in-memory backends)
- [ ] 3.5 Test: `snix-store virtiofs` connects to bridge via `grpc+unix:///path/to/sock`

## 4. Integration Test: End-to-End NAR Round-Trip

- [x] 4.1 Write NAR round-trip test: ingest_nar_and_hash → BlobService + DirectoryService → PathInfoService put/get
- [x] 4.2 Test helloworld NAR (single file), complicated NAR (directory tree), symlink NAR
- [x] 4.3 Test idempotent ingestion (same NAR twice produces same results)
- [x] 4.4 Test PathInfo list after multiple puts

## 5. Connect snix-store VirtioFS to Aspen Backends

- [ ] 5.1 Update snix-boot runVM script to accept ASPEN_TICKET env var for connecting snix-store to cluster
- [ ] 5.2 Test: boot snix microVM, verify /nix/store lists paths previously stored via CI pipeline

## 6. Verify and Clean Up

- [ ] 6.1 Run `cargo clippy --workspace -- --deny warnings` clean
- [ ] 6.2 Run full test suite — all passing
- [ ] 6.3 Update napkin with corrections discovered during implementation
