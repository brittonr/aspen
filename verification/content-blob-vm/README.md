# Protected content handoff VM fixture

This fixture adds cross-process handoff and explicit read grants to the existing live Iroh content adapter. It does not replace Molten content identity or verification.

## Owners

`molten-core::content_store_adapter::ContentReadGrant` owns the deterministic reader/manifest decision. An operator grants one manifest to at most 16 unique reader keys. A pin, endpoint identity, or locator grants no authority by itself. This local policy is not a verified UCAN credential.

`publish_protected_live_iroh_chunks` gathers the existing identity and canonical chunk facts. Its protocol adapter checks the authenticated reader before calling `BlobsProtocol`. Its private memory store contains only the selected manifest chunks. Authorized connection count is bounded. This is not a handshake-flood or hostile-peer availability guarantee.

`LiveIrohRemote` contains admitted metadata, not a server router or private key. `export_handoff` emits canonical Preserves metadata. `admit_live_handoff` requires independent caller expectations for the manifest, provider, and socket. It checks exact locator membership, order, formats, and bounds before networking. It replaces ticket address hints with the admitted socket.

`execute_live_iroh_remote_get` reuses existing content preflight, partial-state transitions, bounded reception, and chunk verification. Transport failures never become verified content. The client can report `stale-ticket` for a denied read. The separate server counter supplies the access-denial observation.

The older publication API remains an unprotected compatibility surface. It does not become protected merely because this fixture exists. Both live-read entry points now require a nonzero timeout of at most 60 seconds.

## Fixture driver

`examples/content-blob-vm.rs` requires the guest kernel marker `molten_blob_vm=1`. The marker is an operator safeguard, not attestation. Do not run its server on the host.

The driver supports `identity`, `prepare`, `serve`, `fetch`, and `fault`. `prepare` checks an explicit archive digest before storage effects. `serve` requires the canonical pin and read grant. `fetch` creates its output only after chunk assembly and exact archive verification.

`fault` deliberately damages a cloned VM test disk. It is not a product deletion or retention API. Private keys remain in capability-rooted guest storage. Public event records do not contain private key material.

`policy.ncl` owns the fixture limits. `policy.json` is its deterministic export. The archive limit is 1 MiB, chunk size is 65,536 bytes, and maximum read time is 10 seconds.

## Verification boundary

Onix owns the two-VM harness under `verification/molten-blob-vm/`. Only the storage VM receives the archive fixture. Clients receive bounded handoff metadata, not shared payload mounts.

The behavior includes exact transfer, server-side reader denial, wrong archive identity, clean storage restart, and missing/corrupt chunk rejection before publication. Onix separately runs Mantle signature admission and both native Darkhttpd VM cases.

The tested source snapshot is `3b5b79132e9b875c6c72c70ed7f38cfdd4e6f11a`, based on release `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0`. The content adapter, core adapter, and crypto owners matched current upstream before this delta. Current upstream dependencies remain unavailable on this worker. No manifest or lockfile was weakened to compile the release snapshot.

The example tests perform no networking. The core tests use explicit values. The actual network servers run only in VMs.

This evidence does not prove package provenance, compiler correctness, reproducibility, garbage collection, power-loss durability, production readiness, or support-matrix eligibility. Current-workspace acceptance and lifecycle completion remain blocked separately.
