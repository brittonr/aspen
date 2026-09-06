# Normal node content service — startup blocked

The normal CLI contains an optional protected content listener and a bounded client. Normal startup and content serving currently reject requests. A real Octet startup-evidence admission route is missing.

The first VM attempt found a pre-existing production call to `synthetic_clean_octet_gate_receipt_for_tests()`. It failed at an ambient `/Cargo.toml` read. That helper declares clean findings and creates test artifact references. Neither a minimal workspace manifest nor the older Onix adapter workaround is acceptable evidence.

Production `node run` now rejects before state creation. Protected content serving rejects before the state root or listener opens. Unit-test startup fixtures remain under `cfg(test)`. No flag or environment variable bypasses these guards.

## Implemented boundary

`NodeContentConfig` and `NodeContentPlan` own closed input admission, explicit read grants, address checks, and a finite tick-wait budget. The existing content adapter owns canonical manifests, verified chunks, bounded handoffs, and Iroh transport.

`src/node/content.rs` derives chunk and identity capabilities from `NodeStateRoot`. It uses the normal node identity rather than a separate fixture key. The daemon starts the listener after startup and service-lock admission, runs it inside its tick loop, and closes it before service completion. These hooks remain behind the guard until real startup evidence exists.

`molten-node-host` owns atomic regular-leaf writes. It does not expose raw directory capabilities. The client publishes complete verified bytes through a no-replace hard link. Existing outputs remain unchanged.

The config surface is `docs/node-content-policy.ncl`. The listener supports one exact manifest, 1–16 unique reader keys, at most two concurrent authorized connections, and a 1 MiB archive bound. The configured tick waits total at most 300 seconds. This is not a hard bound on arbitrary blocking filesystem calls. Each content read has a 10-second timeout.

The new content option does not combine with the separate `--live-iroh` control-listener mode. It never falls back to unprotected publication.

## Verification and limits

Focused core, policy-preservation, atomic-file, dependency-boundary, CLI-denial, and Clippy checks passed. The initial VM run completed identity creation and storage preparation but did not start the listener. It produced no transfer or native replay completion receipt.

The exact pinned nightly/Octet workspace gate remains unverified. Existing Rust 1.97.1 compiled the new CLI; no toolchain was rebuilt. Neither Cargo nor flake lockfile changed. The already-locked `cap-tempfile` dependency moved from test-only to runtime use for atomic publication.

No Stage0, Darkhttpd rebuild, Mantle rebuild, host blob service, physical deployment, or default/release promotion occurred. The next change must bind real source, tool, and gate evidence to startup. It must not merely accept caller-asserted clean counts or a test receipt.
