Evidence-ID: alloc-safe-hooks-ticket.v1-final
Task-ID: V1
Artifact-Type: verification
Covers: architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.bare-hook-ticket-dependency-stays-alloc-safe, architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.expiry-math-stays-testable-without-wall-clock, architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-hook-tickets-roundtrip-successfully, architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent, architecture.modularity.alloc-safe-hook-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe, architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.runtime-conversion-happens-at-the-shell-boundary, architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.std-convenience-wrappers-require-explicit-opt-in, architecture.modularity.hook-ticket-runtime-helpers-require-explicit-shell-opt-in.hook-ticket-seam-proof-is-reviewable, architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-hook-ticket-errors, architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-serialized-hook-tickets-are-rejected-explicitly, architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.runtime-consumers-surface-legacy-decode-failures-explicitly, architecture.modularity.hook-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.hook-ticket-error-surface-proof-is-reviewable, ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fails-loudly-on-serializer-invariant-break, ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fail-loud-proof-is-reviewable

# Final validation

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks-ticket`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.29s
     Running unittests src/lib.rs (target/debug/deps/aspen_hooks_ticket-8d53a5e5f26b3586)

running 13 tests
test tests::test_add_bootstrap_peer_limit ... ok
test tests::test_expiry_helpers ... ok
test tests::test_invalid_ticket_string ... ok
test tests::test_ticket_builder ... ok
test tests::test_deserialize_expired_ticket ... ok
test tests::test_ticket_new ... ok
test tests::test_multiple_bootstrap_peers ... ok
test tests::test_validation_empty_cluster_id ... ok
test tests::test_ticket_roundtrip ... ok
test tests::test_validation_empty_event_type ... ok
test tests::test_ticket_with_auth_roundtrip ... ok
test tests::test_validation_invalid_payload_json ... ok
test tests::test_validation_no_peers ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/legacy.rs (target/debug/deps/legacy-3c77f777d7bad8c1)

running 1 test
test test_legacy_serialized_ticket_is_rejected ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/std.rs (target/debug/deps/std-ba51eda204e5fdce)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ui.rs (target/debug/deps/ui-7df0d735459b2141)

running 1 test
warning: patch `snix-glue v0.1.0 (/home/brittonr/git/aspen/vendor/snix-glue)` was not used in the crate graph
warning: patch `cargo-hyperlight v0.1.5 (/home/brittonr/git/aspen/vendor/cargo-hyperlight)` was not used in the crate graph
warning: patch `uhlc v0.8.2 (/home/brittonr/git/aspen/vendor/uhlc)` was not used in the crate graph
help: Check that the patched package version and available features are compatible
      with the dependency requirements. If the patch has a different version from
      what is locked in the Cargo.lock file, run `cargo update` to use the new
      version. This may also occur with an optional dependency that is not enabled.
    Checking aspen-hooks-ticket-tests v0.0.0 (/home/brittonr/git/aspen/crates/aspen-hooks-ticket/target/tests/trybuild/aspen-hooks-ticket)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s


test [0m[1mtests/ui/std_wrappers_require_feature.rs[0m ... [0m[32mok
[0m

test test_std_wrappers_require_feature ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s

   Doc-tests aspen_hooks_ticket

running 1 test
test crates/aspen-hooks-ticket/src/lib.rs - (line 17) ... ignored

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

all doctests ran in 0.21s; merged doctests compilation took 0.21s
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks-ticket --test ui`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.27s
     Running tests/ui.rs (target/debug/deps/ui-7df0d735459b2141)

running 1 test
warning: patch `cargo-hyperlight v0.1.5 (/home/brittonr/git/aspen/vendor/cargo-hyperlight)` was not used in the crate graph
warning: patch `uhlc v0.8.2 (/home/brittonr/git/aspen/vendor/uhlc)` was not used in the crate graph
warning: patch `snix-glue v0.1.0 (/home/brittonr/git/aspen/vendor/snix-glue)` was not used in the crate graph
help: Check that the patched package version and available features are compatible
      with the dependency requirements. If the patch has a different version from
      what is locked in the Cargo.lock file, run `cargo update` to use the new
      version. This may also occur with an optional dependency that is not enabled.
    Checking aspen-hooks-ticket-tests v0.0.0 (/home/brittonr/git/aspen/crates/aspen-hooks-ticket/target/tests/trybuild/aspen-hooks-ticket)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s


test [0m[1mtests/ui/std_wrappers_require_feature.rs[0m ... [0m[32mok
[0m

test test_std_wrappers_require_feature ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s

```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks-ticket --features std --test std`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.27s
     Running tests/std.rs (target/debug/deps/std-47413162c370c2f3)

running 1 test
test test_std_wrappers_work ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo check -p aspen-hooks-ticket`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo check -p aspen-hooks-ticket --target wasm32-unknown-unknown`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.23s
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo check -p aspen-hooks-ticket --no-default-features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo check -p aspen-hooks-ticket --no-default-features --target wasm32-unknown-unknown`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.22s
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo check -p aspen-hooks-ticket --features std`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo tree -p aspen-hooks-ticket -e normal`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-hooks-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks-ticket)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   ├── serde v1.0.228
│   │   ├── serde_core v1.0.228
│   │   └── serde_derive v1.0.228 (proc-macro)
│   │       ├── proc-macro2 v1.0.106
│   │       │   └── unicode-ident v1.0.24
│   │       ├── quote v1.0.45
│   │       │   └── proc-macro2 v1.0.106 (*)
│   │       └── syn v2.0.117
│   │           ├── proc-macro2 v1.0.106 (*)
│   │           ├── quote v1.0.45 (*)
│   │           └── unicode-ident v1.0.24
│   └── thiserror v2.0.18
│       └── thiserror-impl v2.0.18 (proc-macro)
│           ├── proc-macro2 v1.0.106 (*)
│           ├── quote v1.0.45 (*)
│           └── syn v2.0.117 (*)
├── iroh-tickets v0.4.0
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1
│   │   └── derive_more-impl v2.1.1 (proc-macro)
│   │       ├── convert_case v0.10.0
│   │       │   └── unicode-segmentation v1.12.0
│   │       ├── proc-macro2 v1.0.106 (*)
│   │       ├── quote v1.0.45 (*)
│   │       ├── syn v2.0.117 (*)
│   │       └── unicode-xid v0.2.6
│   ├── iroh-base v0.97.0
│   │   ├── curve25519-dalek v5.0.0-pre.1
│   │   │   ├── cfg-if v1.0.4
│   │   │   ├── cpufeatures v0.2.17
│   │   │   ├── curve25519-dalek-derive v0.1.1 (proc-macro)
│   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   └── syn v2.0.117 (*)
│   │   │   ├── digest v0.11.0-rc.10
│   │   │   │   ├── block-buffer v0.11.0
│   │   │   │   │   └── hybrid-array v0.4.8
│   │   │   │   │       └── typenum v1.19.0
│   │   │   │   ├── const-oid v0.10.2
│   │   │   │   └── crypto-common v0.2.1
│   │   │   │       └── hybrid-array v0.4.8 (*)
│   │   │   ├── rand_core v0.9.5
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── subtle v2.6.1
│   │   │   └── zeroize v1.8.2
│   │   │       └── zeroize_derive v1.4.3 (proc-macro)
│   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │           ├── quote v1.0.45 (*)
│   │   │           └── syn v2.0.117 (*)
│   │   ├── data-encoding v2.10.0
│   │   ├── derive_more v2.1.1 (*)
│   │   ├── digest v0.11.0-rc.10 (*)
│   │   ├── ed25519-dalek v3.0.0-pre.1
│   │   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│   │   │   ├── ed25519 v3.0.0-rc.4
│   │   │   │   ├── serde v1.0.228 (*)
│   │   │   │   └── signature v3.0.0-rc.10
│   │   │   ├── rand_core v0.9.5
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── sha2 v0.11.0-rc.2
│   │   │   │   ├── cfg-if v1.0.4
│   │   │   │   ├── cpufeatures v0.2.17
│   │   │   │   └── digest v0.11.0-rc.10 (*)
│   │   │   ├── subtle v2.6.1
│   │   │   └── zeroize v1.8.2 (*)
│   │   ├── n0-error v0.1.3
│   │   │   ├── n0-error-macros v0.1.3 (proc-macro)
│   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   └── syn v2.0.117 (*)
│   │   │   └── spez v0.1.2 (proc-macro)
│   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │       ├── quote v1.0.45 (*)
│   │   │       └── syn v2.0.117 (*)
│   │   ├── rand_core v0.9.5
│   │   ├── serde v1.0.228 (*)
│   │   ├── sha2 v0.11.0-rc.2 (*)
│   │   ├── url v2.5.8
│   │   │   ├── form_urlencoded v1.2.2
│   │   │   │   └── percent-encoding v2.3.2
│   │   │   ├── idna v1.1.0
│   │   │   │   ├── idna_adapter v1.2.1
│   │   │   │   │   ├── icu_normalizer v2.1.1
│   │   │   │   │   │   ├── icu_collections v2.1.1
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro)
│   │   │   │   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   │   │   │   ├── potential_utf v0.1.4
│   │   │   │   │   │   │   │   └── zerovec v0.11.5
│   │   │   │   │   │   │   │       ├── yoke v0.8.1
│   │   │   │   │   │   │   │       │   ├── stable_deref_trait v1.2.1
│   │   │   │   │   │   │   │       │   ├── yoke-derive v0.8.1 (proc-macro)
│   │   │   │   │   │   │   │       │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │       │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │       │   │   ├── syn v2.0.117 (*)
│   │   │   │   │   │   │   │       │   │   └── synstructure v0.13.2
│   │   │   │   │   │   │   │       │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │       │   │       ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │       │   │       └── syn v2.0.117 (*)
│   │   │   │   │   │   │   │       │   └── zerofrom v0.1.6
│   │   │   │   │   │   │   │       │       └── zerofrom-derive v0.1.6 (proc-macro)
│   │   │   │   │   │   │   │       │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │       │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │       │           ├── syn v2.0.117 (*)
│   │   │   │   │   │   │   │       │           └── synstructure v0.13.2 (*)
│   │   │   │   │   │   │   │       ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   │       └── zerovec-derive v0.11.2 (proc-macro)
│   │   │   │   │   │   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │           └── syn v2.0.117 (*)
│   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   ├── icu_normalizer_data v2.1.1
│   │   │   │   │   │   ├── icu_provider v2.1.1
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   ├── icu_locale_core v2.1.1
│   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   ├── litemap v0.8.1
│   │   │   │   │   │   │   │   ├── tinystr v0.8.2
│   │   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   ├── zerotrie v0.2.3
│   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   │   └── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   ├── smallvec v1.15.1
│   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   └── icu_properties v2.1.2
│   │   │   │   │       ├── icu_collections v2.1.1 (*)
│   │   │   │   │       ├── icu_locale_core v2.1.1 (*)
│   │   │   │   │       ├── icu_properties_data v2.1.2
│   │   │   │   │       ├── icu_provider v2.1.1 (*)
│   │   │   │   │       ├── zerotrie v0.2.3 (*)
│   │   │   │   │       └── zerovec v0.11.5 (*)
│   │   │   │   ├── smallvec v1.15.1
│   │   │   │   └── utf8_iter v1.0.4
│   │   │   ├── percent-encoding v2.3.2
│   │   │   ├── serde v1.0.228 (*)
│   │   │   └── serde_derive v1.0.228 (proc-macro) (*)
│   │   ├── zeroize v1.8.2 (*)
│   │   └── zeroize_derive v1.4.3 (proc-macro) (*)
│   ├── n0-error v0.1.3 (*)
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18 (*)
│   │   ├── heapless v0.7.17
│   │   │   ├── hash32 v0.2.1
│   │   │   │   └── byteorder v1.5.0
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── spin v0.9.8
│   │   │   │   └── lock_api v0.4.14
│   │   │   │       └── scopeguard v1.2.0
│   │   │   └── stable_deref_trait v1.2.1
│   │   └── serde v1.0.228 (*)
│   └── serde v1.0.228 (*)
├── postcard v1.1.3 (*)
├── serde v1.0.228 (*)
├── serde_json v1.0.149
│   ├── itoa v1.0.17
│   ├── memchr v2.8.0
│   ├── serde_core v1.0.228
│   └── zmij v1.0.21
└── thiserror v2.0.18 (*)
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo tree -p aspen-hooks-ticket -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-hooks-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks-ticket)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   ├── thiserror v2.0.18
│   │   └── thiserror-impl feature "default"
│   │       └── thiserror-impl v2.0.18 (proc-macro)
│   │           ├── proc-macro2 feature "default"
│   │           │   ├── proc-macro2 v1.0.106
│   │           │   │   └── unicode-ident feature "default"
│   │           │   │       └── unicode-ident v1.0.24
│   │           │   └── proc-macro2 feature "proc-macro"
│   │           │       └── proc-macro2 v1.0.106 (*)
│   │           ├── quote feature "default"
│   │           │   ├── quote v1.0.45
│   │           │   │   └── proc-macro2 v1.0.106 (*)
│   │           │   └── quote feature "proc-macro"
│   │           │       ├── quote v1.0.45 (*)
│   │           │       └── proc-macro2 feature "proc-macro" (*)
│   │           └── syn feature "default"
│   │               ├── syn v2.0.117
│   │               │   ├── proc-macro2 v1.0.106 (*)
│   │               │   ├── quote v1.0.45 (*)
│   │               │   └── unicode-ident feature "default" (*)
│   │               ├── syn feature "clone-impls"
│   │               │   └── syn v2.0.117 (*)
│   │               ├── syn feature "derive"
│   │               │   └── syn v2.0.117 (*)
│   │               ├── syn feature "parsing"
│   │               │   └── syn v2.0.117 (*)
│   │               ├── syn feature "printing"
│   │               │   └── syn v2.0.117 (*)
│   │               └── syn feature "proc-macro"
│   │                   ├── syn v2.0.117 (*)
│   │                   ├── proc-macro2 feature "proc-macro" (*)
│   │                   └── quote feature "proc-macro" (*)
│   ├── serde feature "alloc"
│   │   ├── serde v1.0.228
│   │   │   ├── serde_core feature "result"
│   │   │   │   └── serde_core v1.0.228
│   │   │   └── serde_derive feature "default"
│   │   │       └── serde_derive v1.0.228 (proc-macro)
│   │   │           ├── proc-macro2 feature "proc-macro" (*)
│   │   │           ├── quote feature "proc-macro" (*)
│   │   │           ├── syn feature "clone-impls" (*)
│   │   │           ├── syn feature "derive" (*)
│   │   │           ├── syn feature "parsing" (*)
│   │   │           ├── syn feature "printing" (*)
│   │   │           └── syn feature "proc-macro" (*)
│   │   └── serde_core feature "alloc"
│   │       └── serde_core v1.0.228
│   └── serde feature "derive"
│       ├── serde v1.0.228 (*)
│       └── serde feature "serde_derive"
│           └── serde v1.0.228 (*)
├── thiserror v2.0.18 (*)
├── serde feature "derive" (*)
├── postcard feature "alloc"
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18 (*)
│   │   ├── serde feature "derive" (*)
│   │   ├── heapless feature "serde"
│   │   │   └── heapless v0.7.17
│   │   │       ├── serde v1.0.228 (*)
│   │   │       ├── stable_deref_trait v1.2.1
│   │   │       ├── hash32 feature "default"
│   │   │       │   └── hash32 v0.2.1
│   │   │       │       └── byteorder v1.5.0
│   │   │       └── spin feature "default"
│   │   │           ├── spin v0.9.8
│   │   │           │   └── lock_api feature "default"
│   │   │           │       ├── lock_api v0.4.14
│   │   │           │       │   └── scopeguard v1.2.0
│   │   │           │       └── lock_api feature "atomic_usize"
│   │   │           │           └── lock_api v0.4.14 (*)
│   │   │           ├── spin feature "barrier"
│   │   │           │   ├── spin v0.9.8 (*)
│   │   │           │   └── spin feature "mutex"
│   │   │           │       └── spin v0.9.8 (*)
│   │   │           ├── spin feature "lazy"
│   │   │           │   ├── spin v0.9.8 (*)
│   │   │           │   └── spin feature "once"
│   │   │           │       └── spin v0.9.8 (*)
│   │   │           ├── spin feature "lock_api"
│   │   │           │   ├── spin v0.9.8 (*)
│   │   │           │   └── spin feature "lock_api_crate"
│   │   │           │       └── spin v0.9.8 (*)
│   │   │           ├── spin feature "mutex" (*)
│   │   │           ├── spin feature "once" (*)
│   │   │           ├── spin feature "rwlock"
│   │   │           │   └── spin v0.9.8 (*)
│   │   │           └── spin feature "spin_mutex"
│   │   │               ├── spin v0.9.8 (*)
│   │   │               └── spin feature "mutex" (*)
│   │   │       [build-dependencies]
│   │   │       └── rustc_version feature "default"
│   │   │           └── rustc_version v0.4.1
│   │   │               └── semver feature "default"
│   │   │                   ├── semver v1.0.27
│   │   │                   └── semver feature "std"
│   │   │                       └── semver v1.0.27
│   │   └── postcard-derive feature "default"
│   │       └── postcard-derive v0.2.2 (proc-macro)
│   │           ├── proc-macro2 feature "default" (*)
│   │           ├── quote feature "default" (*)
│   │           └── syn feature "default" (*)
│   └── serde feature "alloc" (*)
├── iroh-tickets feature "default"
│   └── iroh-tickets v0.4.0
│       ├── serde feature "default"
│       │   ├── serde v1.0.228 (*)
│       │   └── serde feature "std"
│       │       ├── serde v1.0.228 (*)
│       │       └── serde_core feature "std"
│       │           └── serde_core v1.0.228
│       ├── serde feature "derive" (*)
│       ├── data-encoding feature "default"
│       │   ├── data-encoding v2.10.0
│       │   └── data-encoding feature "std"
│       │       ├── data-encoding v2.10.0
│       │       └── data-encoding feature "alloc"
│       │           └── data-encoding v2.10.0
│       ├── derive_more feature "default"
│       │   ├── derive_more v2.1.1
│       │   │   └── derive_more-impl feature "default"
│       │   │       └── derive_more-impl v2.1.1 (proc-macro)
│       │   │           ├── proc-macro2 feature "default" (*)
│       │   │           ├── quote feature "default" (*)
│       │   │           ├── syn feature "default" (*)
│       │   │           ├── convert_case feature "default"
│       │   │           │   └── convert_case v0.10.0
│       │   │           │       └── unicode-segmentation feature "default"
│       │   │           │           └── unicode-segmentation v1.12.0
│       │   │           └── unicode-xid feature "default"
│       │   │               └── unicode-xid v0.2.6
│       │   │           [build-dependencies]
│       │   │           └── rustc_version feature "default" (*)
│       │   └── derive_more feature "std"
│       │       └── derive_more v2.1.1 (*)
│       ├── derive_more feature "display"
│       │   ├── derive_more v2.1.1 (*)
│       │   └── derive_more-impl feature "display"
│       │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│       │       └── syn feature "extra-traits"
│       │           └── syn v2.0.117 (*)
│       ├── iroh-base feature "default"
│       │   ├── iroh-base v0.97.0
│       │   │   ├── serde feature "default" (*)
│       │   │   ├── serde feature "derive" (*)
│       │   │   ├── serde feature "rc"
│       │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   └── serde_core feature "rc"
│       │   │   │       └── serde_core v1.0.228
│       │   │   ├── data-encoding feature "default" (*)
│       │   │   ├── derive_more feature "debug"
│       │   │   │   ├── derive_more v2.1.1 (*)
│       │   │   │   └── derive_more-impl feature "debug"
│       │   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│       │   │   │       └── syn feature "extra-traits" (*)
│       │   │   ├── derive_more feature "default" (*)
│       │   │   ├── derive_more feature "display" (*)
│       │   │   ├── ed25519-dalek feature "default"
│       │   │   │   ├── ed25519-dalek v3.0.0-pre.1
│       │   │   │   │   ├── ed25519 v3.0.0-rc.4
│       │   │   │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   │   │   ├── signature v3.0.0-rc.10
│       │   │   │   │   │   └── pkcs8 feature "default"
│       │   │   │   │   │       └── pkcs8 v0.11.0-rc.11
│       │   │   │   │   │           ├── der feature "default"
│       │   │   │   │   │           │   └── der v0.8.0
│       │   │   │   │   │           │       ├── zeroize v1.8.2
│       │   │   │   │   │           │       │   └── zeroize_derive feature "default"
│       │   │   │   │   │           │       │       └── zeroize_derive v1.4.3 (proc-macro)
│       │   │   │   │   │           │       │           ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │           │       │           ├── quote feature "default" (*)
│       │   │   │   │   │           │       │           ├── syn feature "default" (*)
│       │   │   │   │   │           │       │           ├── syn feature "extra-traits" (*)
│       │   │   │   │   │           │       │           ├── syn feature "full"
│       │   │   │   │   │           │       │           │   └── syn v2.0.117 (*)
│       │   │   │   │   │           │       │           └── syn feature "visit"
│       │   │   │   │   │           │       │               └── syn v2.0.117 (*)
│       │   │   │   │   │           │       ├── const-oid feature "default"
│       │   │   │   │   │           │       │   └── const-oid v0.10.2
│       │   │   │   │   │           │       ├── pem-rfc7468 feature "alloc"
│       │   │   │   │   │           │       │   ├── pem-rfc7468 v1.0.0
│       │   │   │   │   │           │       │   │   └── base64ct feature "default"
│       │   │   │   │   │           │       │   │       └── base64ct v1.8.3
│       │   │   │   │   │           │       │   └── base64ct feature "alloc"
│       │   │   │   │   │           │       │       └── base64ct v1.8.3
│       │   │   │   │   │           │       └── pem-rfc7468 feature "default"
│       │   │   │   │   │           │           └── pem-rfc7468 v1.0.0 (*)
│       │   │   │   │   │           ├── der feature "oid"
│       │   │   │   │   │           │   └── der v0.8.0 (*)
│       │   │   │   │   │           └── spki feature "default"
│       │   │   │   │   │               └── spki v0.8.0-rc.4
│       │   │   │   │   │                   ├── der feature "default" (*)
│       │   │   │   │   │                   └── der feature "oid" (*)
│       │   │   │   │   ├── rand_core v0.9.5
│       │   │   │   │   │   └── getrandom feature "default"
│       │   │   │   │   │       └── getrandom v0.3.4
│       │   │   │   │   │           ├── libc v0.2.183
│       │   │   │   │   │           └── cfg-if feature "default"
│       │   │   │   │   │               └── cfg-if v1.0.4
│       │   │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   │   ├── sha2 v0.11.0-rc.2
│       │   │   │   │   │   ├── cfg-if feature "default" (*)
│       │   │   │   │   │   ├── cpufeatures feature "default"
│       │   │   │   │   │   │   └── cpufeatures v0.2.17
│       │   │   │   │   │   └── digest feature "default"
│       │   │   │   │   │       ├── digest v0.11.0-rc.10
│       │   │   │   │   │       │   ├── block-buffer feature "default"
│       │   │   │   │   │       │   │   └── block-buffer v0.11.0
│       │   │   │   │   │       │   │       └── hybrid-array feature "default"
│       │   │   │   │   │       │   │           └── hybrid-array v0.4.8
│       │   │   │   │   │       │   │               ├── typenum feature "const-generics"
│       │   │   │   │   │       │   │               │   └── typenum v1.19.0
│       │   │   │   │   │       │   │               └── typenum feature "default"
│       │   │   │   │   │       │   │                   └── typenum v1.19.0
│       │   │   │   │   │       │   ├── const-oid feature "default" (*)
│       │   │   │   │   │       │   └── crypto-common feature "default"
│       │   │   │   │   │       │       └── crypto-common v0.2.1
│       │   │   │   │   │       │           └── hybrid-array feature "default" (*)
│       │   │   │   │   │       └── digest feature "block-api"
│       │   │   │   │   │           ├── digest v0.11.0-rc.10 (*)
│       │   │   │   │   │           └── digest feature "block-buffer"
│       │   │   │   │   │               └── digest v0.11.0-rc.10 (*)
│       │   │   │   │   ├── signature v3.0.0-rc.10
│       │   │   │   │   ├── subtle v2.6.1
│       │   │   │   │   ├── zeroize v1.8.2 (*)
│       │   │   │   │   └── curve25519-dalek feature "digest"
│       │   │   │   │       └── curve25519-dalek v5.0.0-pre.1
│       │   │   │   │           ├── rand_core v0.9.5 (*)
│       │   │   │   │           ├── zeroize v1.8.2 (*)
│       │   │   │   │           ├── serde feature "derive" (*)
│       │   │   │   │           ├── cfg-if feature "default" (*)
│       │   │   │   │           ├── cpufeatures feature "default" (*)
│       │   │   │   │           ├── curve25519-dalek-derive feature "default"
│       │   │   │   │           │   └── curve25519-dalek-derive v0.1.1 (proc-macro)
│       │   │   │   │           │       ├── proc-macro2 feature "default" (*)
│       │   │   │   │           │       ├── quote feature "default" (*)
│       │   │   │   │           │       ├── syn feature "default" (*)
│       │   │   │   │           │       └── syn feature "full" (*)
│       │   │   │   │           ├── digest feature "block-api" (*)
│       │   │   │   │           └── subtle feature "const-generics"
│       │   │   │   │               └── subtle v2.6.1
│       │   │   │   │           [build-dependencies]
│       │   │   │   │           └── rustc_version feature "default" (*)
│       │   │   │   ├── ed25519-dalek feature "fast"
│       │   │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   │   │   └── curve25519-dalek feature "precomputed-tables"
│       │   │   │   │       └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   │   └── ed25519-dalek feature "zeroize"
│       │   │   │       ├── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   │       └── curve25519-dalek feature "zeroize"
│       │   │   │           └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   ├── ed25519-dalek feature "rand_core"
│       │   │   │   └── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   ├── ed25519-dalek feature "serde"
│       │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   │   └── ed25519 feature "serde"
│       │   │   │       └── ed25519 v3.0.0-rc.4 (*)
│       │   │   ├── ed25519-dalek feature "zeroize" (*)
│       │   │   ├── curve25519-dalek feature "default"
│       │   │   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   │   ├── curve25519-dalek feature "alloc"
│       │   │   │   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   │   │   └── zeroize feature "alloc"
│       │   │   │   │       └── zeroize v1.8.2 (*)
│       │   │   │   ├── curve25519-dalek feature "precomputed-tables" (*)
│       │   │   │   └── curve25519-dalek feature "zeroize" (*)
│       │   │   ├── curve25519-dalek feature "rand_core"
│       │   │   │   └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   ├── curve25519-dalek feature "serde"
│       │   │   │   └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   ├── curve25519-dalek feature "zeroize" (*)
│       │   │   ├── digest feature "default" (*)
│       │   │   ├── rand_core feature "default"
│       │   │   │   └── rand_core v0.9.5 (*)
│       │   │   ├── zeroize feature "default"
│       │   │   │   ├── zeroize v1.8.2 (*)
│       │   │   │   └── zeroize feature "alloc" (*)
│       │   │   ├── zeroize feature "derive"
│       │   │   │   ├── zeroize v1.8.2 (*)
│       │   │   │   └── zeroize feature "zeroize_derive"
│       │   │   │       └── zeroize v1.8.2 (*)
│       │   │   ├── zeroize_derive feature "default" (*)
│       │   │   ├── sha2 feature "default"
│       │   │   │   ├── sha2 v0.11.0-rc.2 (*)
│       │   │   │   ├── sha2 feature "alloc"
│       │   │   │   │   ├── sha2 v0.11.0-rc.2 (*)
│       │   │   │   │   └── digest feature "alloc"
│       │   │   │   │       └── digest v0.11.0-rc.10 (*)
│       │   │   │   └── sha2 feature "oid"
│       │   │   │       ├── sha2 v0.11.0-rc.2 (*)
│       │   │   │       └── digest feature "oid"
│       │   │   │           ├── digest v0.11.0-rc.10 (*)
│       │   │   │           └── digest feature "const-oid"
│       │   │   │               └── digest v0.11.0-rc.10 (*)
│       │   │   ├── url feature "default"
│       │   │   │   ├── url v2.5.8
│       │   │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   │   ├── serde_derive v1.0.228 (proc-macro) (*)
│       │   │   │   │   ├── idna feature "alloc"
│       │   │   │   │   │   └── idna v1.1.0
│       │   │   │   │   │       ├── idna_adapter feature "default"
│       │   │   │   │   │       │   └── idna_adapter v1.2.1
│       │   │   │   │   │       │       ├── icu_normalizer v2.1.1
│       │   │   │   │   │       │       │   ├── icu_collections v2.1.1
│       │   │   │   │   │       │       │   │   ├── displaydoc v0.2.5 (proc-macro)
│       │   │   │   │   │       │       │   │   │   ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │       │       │   │   │   ├── quote feature "default" (*)
│       │   │   │   │   │       │       │   │   │   └── syn feature "default" (*)
│       │   │   │   │   │       │       │   │   ├── potential_utf feature "zerovec"
│       │   │   │   │   │       │       │   │   │   └── potential_utf v0.1.4
│       │   │   │   │   │       │       │   │   │       └── zerovec v0.11.5
│       │   │   │   │   │       │       │   │   │           ├── yoke v0.8.1
│       │   │   │   │   │       │       │   │   │           │   ├── stable_deref_trait v1.2.1
│       │   │   │   │   │       │       │   │   │           │   ├── yoke-derive v0.8.1 (proc-macro)
│       │   │   │   │   │       │       │   │   │           │   │   ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │   │   ├── quote feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │   │   ├── syn feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │   │   ├── syn feature "fold"
│       │   │   │   │   │       │       │   │   │           │   │   │   └── syn v2.0.117 (*)
│       │   │   │   │   │       │       │   │   │           │   │   └── synstructure feature "default"
│       │   │   │   │   │       │       │   │   │           │   │       ├── synstructure v0.13.2
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── proc-macro2 v1.0.106 (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── quote v1.0.45 (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "clone-impls" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "derive" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "extra-traits" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "parsing" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "printing" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   └── syn feature "visit" (*)
│       │   │   │   │   │       │       │   │   │           │   │       └── synstructure feature "proc-macro"
│       │   │   │   │   │       │       │   │   │           │   │           ├── synstructure v0.13.2 (*)
│       │   │   │   │   │       │       │   │   │           │   │           ├── proc-macro2 feature "proc-macro" (*)
│       │   │   │   │   │       │       │   │   │           │   │           ├── quote feature "proc-macro" (*)
│       │   │   │   │   │       │       │   │   │           │   │           └── syn feature "proc-macro" (*)
│       │   │   │   │   │       │       │   │   │           │   └── zerofrom v0.1.6
│       │   │   │   │   │       │       │   │   │           │       └── zerofrom-derive v0.1.6 (proc-macro)
│       │   │   │   │   │       │       │   │   │           │           ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │           ├── quote feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │           ├── syn feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │           ├── syn feature "fold" (*)
│       │   │   │   │   │       │       │   │   │           │           └── synstructure feature "default" (*)
│       │   │   │   │   │       │       │   │   │           ├── zerofrom v0.1.6 (*)
│       │   │   │   │   │       │       │   │   │           └── zerovec-derive v0.11.2 (proc-macro)
│       │   │   │   │   │       │       │   │   │               ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │       │       │   │   │               ├── quote feature "default" (*)
│       │   │   │   │   │       │       │   │   │               ├── syn feature "default" (*)
│       │   │   │   │   │       │       │   │   │               └── syn feature "extra-traits" (*)
│       │   │   │   │   │       │       │   │   ├── zerovec feature "derive"
│       │   │   │   │   │       │       │   │   │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       │   │   ├── zerovec feature "yoke"
│       │   │   │   │   │       │       │   │   │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       │   │   ├── yoke feature "derive"
│       │   │   │   │   │       │       │   │   │   ├── yoke v0.8.1 (*)
│       │   │   │   │   │       │       │   │   │   ├── yoke feature "zerofrom"
│       │   │   │   │   │       │       │   │   │   │   └── yoke v0.8.1 (*)
│       │   │   │   │   │       │       │   │   │   └── zerofrom feature "derive"
│       │   │   │   │   │       │       │   │   │       └── zerofrom v0.1.6 (*)
│       │   │   │   │   │       │       │   │   └── zerofrom feature "derive" (*)
│       │   │   │   │   │       │       │   ├── icu_normalizer_data v2.1.1
│       │   │   │   │   │       │       │   ├── icu_provider v2.1.1
│       │   │   │   │   │       │       │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│       │   │   │   │   │       │       │   │   ├── icu_locale_core v2.1.1
│       │   │   │   │   │       │       │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│       │   │   │   │   │       │       │   │   │   ├── litemap v0.8.1
│       │   │   │   │   │       │       │   │   │   ├── tinystr v0.8.2
│       │   │   │   │   │       │       │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│       │   │   │   │   │       │       │   │   │   │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       │   │   │   ├── writeable v0.6.2
│       │   │   │   │   │       │       │   │   │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       │   │   ├── writeable v0.6.2
│       │   │   │   │   │       │       │   │   ├── zerotrie v0.2.3
│       │   │   │   │   │       │       │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│       │   │   │   │   │       │       │   │   │   ├── zerofrom v0.1.6 (*)
│       │   │   │   │   │       │       │   │   │   └── yoke feature "derive" (*)
│       │   │   │   │   │       │       │   │   ├── zerovec feature "derive" (*)
│       │   │   │   │   │       │       │   │   ├── yoke feature "derive" (*)
│       │   │   │   │   │       │       │   │   └── zerofrom feature "derive" (*)
│       │   │   │   │   │       │       │   ├── smallvec v1.15.1
│       │   │   │   │   │       │       │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       └── icu_properties v2.1.2
│       │   │   │   │   │       │           ├── icu_collections v2.1.1 (*)
│       │   │   │   │   │       │           ├── icu_properties_data v2.1.2
│       │   │   │   │   │       │           ├── icu_provider v2.1.1 (*)
│       │   │   │   │   │       │           ├── zerovec feature "derive" (*)
│       │   │   │   │   │       │           ├── zerovec feature "yoke" (*)
│       │   │   │   │   │       │           ├── icu_locale_core feature "zerovec"
│       │   │   │   │   │       │           │   ├── icu_locale_core v2.1.1 (*)
│       │   │   │   │   │       │           │   └── tinystr feature "zerovec"
│       │   │   │   │   │       │           │       └── tinystr v0.8.2 (*)
│       │   │   │   │   │       │           ├── zerotrie feature "yoke"
│       │   │   │   │   │       │           │   └── zerotrie v0.2.3 (*)
│       │   │   │   │   │       │           └── zerotrie feature "zerofrom"
│       │   │   │   │   │       │               └── zerotrie v0.2.3 (*)
│       │   │   │   │   │       ├── smallvec feature "const_generics"
│       │   │   │   │   │       │   └── smallvec v1.15.1
│       │   │   │   │   │       ├── smallvec feature "default"
│       │   │   │   │   │       │   └── smallvec v1.15.1
│       │   │   │   │   │       └── utf8_iter feature "default"
│       │   │   │   │   │           └── utf8_iter v1.0.4
│       │   │   │   │   ├── idna feature "compiled_data"
│       │   │   │   │   │   ├── idna v1.1.0 (*)
│       │   │   │   │   │   └── idna_adapter feature "compiled_data"
│       │   │   │   │   │       ├── idna_adapter v1.2.1 (*)
│       │   │   │   │   │       ├── icu_normalizer feature "compiled_data"
│       │   │   │   │   │       │   ├── icu_normalizer v2.1.1 (*)
│       │   │   │   │   │       │   └── icu_provider feature "baked"
│       │   │   │   │   │       │       └── icu_provider v2.1.1 (*)
│       │   │   │   │   │       └── icu_properties feature "compiled_data"
│       │   │   │   │   │           ├── icu_properties v2.1.2 (*)
│       │   │   │   │   │           └── icu_provider feature "baked" (*)
│       │   │   │   │   ├── form_urlencoded feature "alloc"
│       │   │   │   │   │   ├── form_urlencoded v1.2.2
│       │   │   │   │   │   │   └── percent-encoding v2.3.2
│       │   │   │   │   │   └── percent-encoding feature "alloc"
│       │   │   │   │   │       └── percent-encoding v2.3.2
│       │   │   │   │   └── percent-encoding feature "alloc" (*)
│       │   │   │   └── url feature "std"
│       │   │   │       ├── url v2.5.8 (*)
│       │   │   │       ├── serde feature "std" (*)
│       │   │   │       ├── idna feature "std"
│       │   │   │       │   ├── idna v1.1.0 (*)
│       │   │   │       │   └── idna feature "alloc" (*)
│       │   │   │       ├── form_urlencoded feature "std"
│       │   │   │       │   ├── form_urlencoded v1.2.2 (*)
│       │   │   │       │   ├── form_urlencoded feature "alloc" (*)
│       │   │   │       │   └── percent-encoding feature "std"
│       │   │   │       │       ├── percent-encoding v2.3.2
│       │   │   │       │       └── percent-encoding feature "alloc" (*)
│       │   │   │       └── percent-encoding feature "std" (*)
│       │   │   ├── url feature "serde"
│       │   │   │   └── url v2.5.8 (*)
│       │   │   └── n0-error feature "default"
│       │   │       └── n0-error v0.1.3
│       │   │           ├── n0-error-macros feature "default"
│       │   │           │   └── n0-error-macros v0.1.3 (proc-macro)
│       │   │           │       ├── proc-macro2 feature "default" (*)
│       │   │           │       ├── quote feature "default" (*)
│       │   │           │       ├── syn feature "default" (*)
│       │   │           │       ├── syn feature "extra-traits" (*)
│       │   │           │       └── syn feature "full" (*)
│       │   │           └── spez feature "default"
│       │   │               └── spez v0.1.2 (proc-macro)
│       │   │                   ├── proc-macro2 feature "default" (*)
│       │   │                   ├── quote feature "default" (*)
│       │   │                   ├── syn feature "default" (*)
│       │   │                   └── syn feature "full" (*)
│       │   └── iroh-base feature "relay"
│       │       └── iroh-base v0.97.0 (*)
│       ├── iroh-base feature "key"
│       │   ├── iroh-base v0.97.0 (*)
│       │   └── iroh-base feature "relay" (*)
│       ├── n0-error feature "default" (*)
│       ├── postcard feature "default"
│       │   ├── postcard v1.1.3 (*)
│       │   └── postcard feature "heapless-cas"
│       │       ├── postcard v1.1.3 (*)
│       │       ├── postcard feature "heapless"
│       │       │   └── postcard v1.1.3 (*)
│       │       └── heapless feature "cas"
│       │           ├── heapless v0.7.17 (*)
│       │           └── heapless feature "atomic-polyfill"
│       │               └── heapless v0.7.17 (*)
│       └── postcard feature "use-std"
│           ├── postcard v1.1.3 (*)
│           ├── serde feature "std" (*)
│           └── postcard feature "alloc" (*)
└── serde_json feature "alloc"
    ├── serde_json v1.0.149
    │   ├── memchr v2.8.0
    │   ├── serde_core v1.0.228
    │   ├── itoa feature "default"
    │   │   └── itoa v1.0.17
    │   └── zmij feature "default"
    │       └── zmij v1.0.21
    └── serde_core feature "alloc" (*)
[dev-dependencies]
├── iroh feature "default"
│   ├── iroh v0.97.0
│   │   ├── iroh-metrics v0.38.3
│   │   │   ├── serde feature "default" (*)
│   │   │   ├── serde feature "derive" (*)
│   │   │   ├── serde feature "rc" (*)
│   │   │   ├── itoa feature "default" (*)
│   │   │   ├── tracing feature "default"
│   │   │   │   ├── tracing v0.1.44
│   │   │   │   │   ├── tracing-core v0.1.36
│   │   │   │   │   │   └── once_cell feature "default"
│   │   │   │   │   │       ├── once_cell v1.21.4
│   │   │   │   │   │       │   ├── portable-atomic v1.13.1
│   │   │   │   │   │       │   │   └── serde v1.0.228 (*)
│   │   │   │   │   │       │   └── critical-section feature "default"
│   │   │   │   │   │       │       └── critical-section v1.2.0
│   │   │   │   │   │       └── once_cell feature "std"
│   │   │   │   │   │           ├── once_cell v1.21.4 (*)
│   │   │   │   │   │           └── once_cell feature "alloc"
│   │   │   │   │   │               ├── once_cell v1.21.4 (*)
│   │   │   │   │   │               └── once_cell feature "race"
│   │   │   │   │   │                   └── once_cell v1.21.4 (*)
│   │   │   │   │   ├── pin-project-lite feature "default"
│   │   │   │   │   │   └── pin-project-lite v0.2.17
│   │   │   │   │   ├── log feature "default"
│   │   │   │   │   │   └── log v0.4.29
│   │   │   │   │   └── tracing-attributes feature "default"
│   │   │   │   │       └── tracing-attributes v0.1.31 (proc-macro)
│   │   │   │   │           ├── proc-macro2 feature "default" (*)
│   │   │   │   │           ├── quote feature "default" (*)
│   │   │   │   │           ├── syn feature "clone-impls" (*)
│   │   │   │   │           ├── syn feature "extra-traits" (*)
│   │   │   │   │           ├── syn feature "full" (*)
│   │   │   │   │           ├── syn feature "parsing" (*)
│   │   │   │   │           ├── syn feature "printing" (*)
│   │   │   │   │           ├── syn feature "proc-macro" (*)
│   │   │   │   │           └── syn feature "visit-mut"
│   │   │   │   │               └── syn v2.0.117 (*)
│   │   │   │   ├── tracing feature "attributes"
│   │   │   │   │   ├── tracing v0.1.44 (*)
│   │   │   │   │   └── tracing feature "tracing-attributes"
│   │   │   │   │       └── tracing v0.1.44 (*)
│   │   │   │   └── tracing feature "std"
│   │   │   │       ├── tracing v0.1.44 (*)
│   │   │   │       └── tracing-core feature "std"
│   │   │   │           ├── tracing-core v0.1.36 (*)
│   │   │   │           └── tracing-core feature "once_cell"
│   │   │   │               └── tracing-core v0.1.36 (*)
│   │   │   ├── portable-atomic feature "default"
│   │   │   │   ├── portable-atomic v1.13.1 (*)
│   │   │   │   └── portable-atomic feature "fallback"
│   │   │   │       └── portable-atomic v1.13.1 (*)
│   │   │   ├── portable-atomic feature "serde"
│   │   │   │   └── portable-atomic v1.13.1 (*)
│   │   │   ├── n0-error feature "default" (*)
│   │   │   ├── iroh-metrics-derive feature "default"
│   │   │   │   └── iroh-metrics-derive v0.4.1 (proc-macro)
│   │   │   │       ├── proc-macro2 feature "default" (*)
│   │   │   │       ├── quote feature "default" (*)
│   │   │   │       ├── syn feature "default" (*)
│   │   │   │       └── heck feature "default"
│   │   │   │           └── heck v0.5.0
│   │   │   ├── postcard feature "default" (*)
│   │   │   ├── postcard feature "use-std" (*)
│   │   │   └── ryu feature "default"
│   │   │       └── ryu v1.0.23
│   │   ├── iroh-relay v0.97.0
│   │   │   ├── iroh-metrics v0.38.3 (*)
│   │   │   ├── serde feature "default" (*)
│   │   │   ├── serde feature "derive" (*)
│   │   │   ├── serde feature "rc" (*)
│   │   │   ├── tokio feature "default"
│   │   │   │   └── tokio v1.50.0
│   │   │   │       ├── mio v1.1.1
│   │   │   │       │   └── libc feature "default"
│   │   │   │       │       ├── libc v0.2.183
│   │   │   │       │       └── libc feature "std"
│   │   │   │       │           └── libc v0.2.183
│   │   │   │       ├── bytes feature "default"
│   │   │   │       │   ├── bytes v1.11.1
│   │   │   │       │   └── bytes feature "std"
│   │   │   │       │       └── bytes v1.11.1
│   │   │   │       ├── libc feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── socket2 feature "all"
│   │   │   │       │   └── socket2 v0.6.3
│   │   │   │       │       └── libc feature "default" (*)
│   │   │   │       ├── socket2 feature "default"
│   │   │   │       │   └── socket2 v0.6.3 (*)
│   │   │   │       └── tokio-macros feature "default"
│   │   │   │           └── tokio-macros v2.6.1 (proc-macro)
│   │   │   │               ├── proc-macro2 feature "default" (*)
│   │   │   │               ├── quote feature "default" (*)
│   │   │   │               ├── syn feature "default" (*)
│   │   │   │               └── syn feature "full" (*)
│   │   │   ├── tokio feature "fs"
│   │   │   │   └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "io-std"
│   │   │   │   └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "io-util"
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   └── tokio feature "bytes"
│   │   │   │       └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "macros"
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   └── tokio feature "tokio-macros"
│   │   │   │       └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "net"
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   ├── tokio feature "libc"
│   │   │   │   │   └── tokio v1.50.0 (*)
│   │   │   │   ├── tokio feature "mio"
│   │   │   │   │   └── tokio v1.50.0 (*)
│   │   │   │   ├── tokio feature "socket2"
│   │   │   │   │   └── tokio v1.50.0 (*)
│   │   │   │   ├── mio feature "net"
│   │   │   │   │   └── mio v1.1.1 (*)
│   │   │   │   ├── mio feature "os-ext"
│   │   │   │   │   ├── mio v1.1.1 (*)
│   │   │   │   │   └── mio feature "os-poll"
│   │   │   │   │       └── mio v1.1.1 (*)
│   │   │   │   └── mio feature "os-poll" (*)
│   │   │   ├── tokio feature "rt"
│   │   │   │   └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "sync"
│   │   │   │   └── tokio v1.50.0 (*)
│   │   │   ├── bytes feature "default" (*)
│   │   │   ├── data-encoding feature "default" (*)
│   │   │   ├── derive_more feature "debug" (*)
│   │   │   ├── derive_more feature "default" (*)
│   │   │   ├── derive_more feature "deref"
│   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   └── derive_more-impl feature "deref"
│   │   │   │       └── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │   ├── derive_more feature "display" (*)
│   │   │   ├── derive_more feature "from"
│   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   └── derive_more-impl feature "from"
│   │   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │   │       └── syn feature "extra-traits" (*)
│   │   │   ├── derive_more feature "try_into"
│   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   └── derive_more-impl feature "try_into"
│   │   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │   │       ├── syn feature "extra-traits" (*)
│   │   │   │       ├── syn feature "full" (*)
│   │   │   │       └── syn feature "visit-mut" (*)
│   │   │   ├── hickory-resolver feature "default"
│   │   │   │   ├── hickory-resolver v0.25.2
│   │   │   │   │   ├── thiserror v2.0.18 (*)
│   │   │   │   │   ├── tokio-rustls v0.26.4
│   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   └── rustls feature "std"
│   │   │   │   │   │       ├── rustls v0.23.37
│   │   │   │   │   │       │   ├── subtle v2.6.1
│   │   │   │   │   │       │   ├── zeroize feature "default" (*)
│   │   │   │   │   │       │   ├── log feature "default" (*)
│   │   │   │   │   │       │   ├── once_cell feature "alloc" (*)
│   │   │   │   │   │       │   ├── once_cell feature "race" (*)
│   │   │   │   │   │       │   ├── ring feature "default"
│   │   │   │   │   │       │   │   ├── ring v0.17.14
│   │   │   │   │   │       │   │   │   ├── cfg-if v1.0.4
│   │   │   │   │   │       │   │   │   ├── getrandom feature "default"
│   │   │   │   │   │       │   │   │   │   └── getrandom v0.2.17
│   │   │   │   │   │       │   │   │   │       ├── libc v0.2.183
│   │   │   │   │   │       │   │   │   │       └── cfg-if feature "default" (*)
│   │   │   │   │   │       │   │   │   └── untrusted feature "default"
│   │   │   │   │   │       │   │   │       └── untrusted v0.9.0
│   │   │   │   │   │       │   │   │   [build-dependencies]
│   │   │   │   │   │       │   │   │   └── cc v1.2.57
│   │   │   │   │   │       │   │   │       ├── find-msvc-tools feature "default"
│   │   │   │   │   │       │   │   │       │   └── find-msvc-tools v0.1.9
│   │   │   │   │   │       │   │   │       └── shlex feature "default"
│   │   │   │   │   │       │   │   │           ├── shlex v1.3.0
│   │   │   │   │   │       │   │   │           └── shlex feature "std"
│   │   │   │   │   │       │   │   │               └── shlex v1.3.0
│   │   │   │   │   │       │   │   ├── ring feature "alloc"
│   │   │   │   │   │       │   │   │   └── ring v0.17.14 (*)
│   │   │   │   │   │       │   │   └── ring feature "dev_urandom_fallback"
│   │   │   │   │   │       │   │       └── ring v0.17.14 (*)
│   │   │   │   │   │       │   ├── rustls-pki-types feature "alloc"
│   │   │   │   │   │       │   │   └── rustls-pki-types v1.14.0
│   │   │   │   │   │       │   │       └── zeroize feature "default" (*)
│   │   │   │   │   │       │   ├── rustls-pki-types feature "default"
│   │   │   │   │   │       │   │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   │       │   │   └── rustls-pki-types feature "alloc" (*)
│   │   │   │   │   │       │   └── rustls-webpki feature "alloc"
│   │   │   │   │   │       │       ├── rustls-webpki v0.103.9
│   │   │   │   │   │       │       │   ├── ring v0.17.14 (*)
│   │   │   │   │   │       │       │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   │       │       │   └── untrusted feature "default" (*)
│   │   │   │   │   │       │       ├── ring feature "alloc" (*)
│   │   │   │   │   │       │       └── rustls-pki-types feature "alloc" (*)
│   │   │   │   │   │       ├── once_cell feature "std" (*)
│   │   │   │   │   │       ├── rustls-pki-types feature "std"
│   │   │   │   │   │       │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   │       │   └── rustls-pki-types feature "alloc" (*)
│   │   │   │   │   │       └── rustls-webpki feature "std"
│   │   │   │   │   │           ├── rustls-webpki v0.103.9 (*)
│   │   │   │   │   │           ├── rustls-pki-types feature "std" (*)
│   │   │   │   │   │           └── rustls-webpki feature "alloc" (*)
│   │   │   │   │   ├── tracing v0.1.44 (*)
│   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   ├── cfg-if feature "default" (*)
│   │   │   │   │   ├── futures-util feature "std"
│   │   │   │   │   │   ├── futures-util v0.3.32
│   │   │   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   │   │   ├── futures-macro v0.3.32 (proc-macro)
│   │   │   │   │   │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   │   │   ├── quote feature "default" (*)
│   │   │   │   │   │   │   │   ├── syn feature "default" (*)
│   │   │   │   │   │   │   │   └── syn feature "full" (*)
│   │   │   │   │   │   │   ├── futures-sink v0.3.32
│   │   │   │   │   │   │   ├── futures-task v0.3.32
│   │   │   │   │   │   │   ├── slab v0.4.12
│   │   │   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   │   │   ├── futures-channel feature "std"
│   │   │   │   │   │   │   │   ├── futures-channel v0.3.32
│   │   │   │   │   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   │   │   │   │   └── futures-sink v0.3.32
│   │   │   │   │   │   │   │   ├── futures-channel feature "alloc"
│   │   │   │   │   │   │   │   │   ├── futures-channel v0.3.32 (*)
│   │   │   │   │   │   │   │   │   └── futures-core feature "alloc"
│   │   │   │   │   │   │   │   │       └── futures-core v0.3.32
│   │   │   │   │   │   │   │   └── futures-core feature "std"
│   │   │   │   │   │   │   │       ├── futures-core v0.3.32
│   │   │   │   │   │   │   │       └── futures-core feature "alloc" (*)
│   │   │   │   │   │   │   ├── futures-io feature "std"
│   │   │   │   │   │   │   │   └── futures-io v0.3.32
│   │   │   │   │   │   │   └── memchr feature "default"
│   │   │   │   │   │   │       ├── memchr v2.8.0
│   │   │   │   │   │   │       └── memchr feature "std"
│   │   │   │   │   │   │           ├── memchr v2.8.0
│   │   │   │   │   │   │           └── memchr feature "alloc"
│   │   │   │   │   │   │               └── memchr v2.8.0
│   │   │   │   │   │   ├── futures-util feature "alloc"
│   │   │   │   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   ├── futures-util feature "slab"
│   │   │   │   │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   ├── futures-core feature "alloc" (*)
│   │   │   │   │   │   │   └── futures-task feature "alloc"
│   │   │   │   │   │   │       └── futures-task v0.3.32
│   │   │   │   │   │   ├── futures-util feature "slab" (*)
│   │   │   │   │   │   ├── futures-core feature "std" (*)
│   │   │   │   │   │   ├── futures-task feature "std"
│   │   │   │   │   │   │   ├── futures-task v0.3.32
│   │   │   │   │   │   │   └── futures-task feature "alloc" (*)
│   │   │   │   │   │   └── slab feature "std"
│   │   │   │   │   │       └── slab v0.4.12
│   │   │   │   │   ├── hickory-proto feature "std"
│   │   │   │   │   │   ├── hickory-proto v0.25.2
│   │   │   │   │   │   │   ├── futures-io v0.3.32
│   │   │   │   │   │   │   ├── ipnet v2.12.0
│   │   │   │   │   │   │   ├── thiserror v2.0.18 (*)
│   │   │   │   │   │   │   ├── tracing v0.1.44 (*)
│   │   │   │   │   │   │   ├── url v2.5.8 (*)
│   │   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   │   ├── tokio feature "io-util" (*)
│   │   │   │   │   │   │   ├── tokio feature "macros" (*)
│   │   │   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   │   │   ├── data-encoding feature "alloc" (*)
│   │   │   │   │   │   │   ├── cfg-if feature "default" (*)
│   │   │   │   │   │   │   ├── futures-util feature "alloc" (*)
│   │   │   │   │   │   │   ├── futures-channel feature "alloc" (*)
│   │   │   │   │   │   │   ├── async-trait feature "default"
│   │   │   │   │   │   │   │   └── async-trait v0.1.89 (proc-macro)
│   │   │   │   │   │   │   │       ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   │   │       ├── quote feature "default" (*)
│   │   │   │   │   │   │   │       ├── syn feature "clone-impls" (*)
│   │   │   │   │   │   │   │       ├── syn feature "full" (*)
│   │   │   │   │   │   │   │       ├── syn feature "parsing" (*)
│   │   │   │   │   │   │   │       ├── syn feature "printing" (*)
│   │   │   │   │   │   │   │       ├── syn feature "proc-macro" (*)
│   │   │   │   │   │   │   │       └── syn feature "visit-mut" (*)
│   │   │   │   │   │   │   ├── enum-as-inner feature "default"
│   │   │   │   │   │   │   │   └── enum-as-inner v0.6.1 (proc-macro)
│   │   │   │   │   │   │   │       ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   │   │       ├── quote feature "default" (*)
│   │   │   │   │   │   │   │       ├── syn feature "default" (*)
│   │   │   │   │   │   │   │       └── heck feature "default" (*)
│   │   │   │   │   │   │   ├── h2 feature "default"
│   │   │   │   │   │   │   │   └── h2 v0.4.13
│   │   │   │   │   │   │   │       ├── futures-core v0.3.32
│   │   │   │   │   │   │   │       ├── futures-sink v0.3.32
│   │   │   │   │   │   │   │       ├── tokio feature "default" (*)
│   │   │   │   │   │   │   │       ├── tokio feature "io-util" (*)
│   │   │   │   │   │   │   │       ├── bytes feature "default" (*)
│   │   │   │   │   │   │   │       ├── slab feature "default"
│   │   │   │   │   │   │   │       │   ├── slab v0.4.12
│   │   │   │   │   │   │   │       │   └── slab feature "std" (*)
│   │   │   │   │   │   │   │       ├── atomic-waker feature "default"
│   │   │   │   │   │   │   │       │   └── atomic-waker v1.1.2
│   │   │   │   │   │   │   │       ├── fnv feature "default"
│   │   │   │   │   │   │   │       │   ├── fnv v1.0.7
│   │   │   │   │   │   │   │       │   └── fnv feature "std"
│   │   │   │   │   │   │   │       │       └── fnv v1.0.7
│   │   │   │   │   │   │   │       ├── http feature "default"
│   │   │   │   │   │   │   │       │   ├── http v1.4.0
│   │   │   │   │   │   │   │       │   │   ├── bytes feature "default" (*)
│   │   │   │   │   │   │   │       │   │   └── itoa feature "default" (*)
│   │   │   │   │   │   │   │       │   └── http feature "std"
│   │   │   │   │   │   │   │       │       └── http v1.4.0 (*)
│   │   │   │   │   │   │   │       ├── indexmap feature "default"
│   │   │   │   │   │   │   │       │   ├── indexmap v2.13.0
│   │   │   │   │   │   │   │       │   │   ├── equivalent v1.0.2
│   │   │   │   │   │   │   │       │   │   └── hashbrown v0.16.1
│   │   │   │   │   │   │   │       │   │       ├── equivalent v1.0.2
│   │   │   │   │   │   │   │       │   │       ├── foldhash v0.2.0
│   │   │   │   │   │   │   │       │   │       └── allocator-api2 feature "alloc"
│   │   │   │   │   │   │   │       │   │           └── allocator-api2 v0.2.21
│   │   │   │   │   │   │   │       │   └── indexmap feature "std"
│   │   │   │   │   │   │   │       │       └── indexmap v2.13.0 (*)
│   │   │   │   │   │   │   │       ├── indexmap feature "std" (*)
│   │   │   │   │   │   │   │       ├── tokio-util feature "codec"
│   │   │   │   │   │   │   │       │   └── tokio-util v0.7.18
│   │   │   │   │   │   │   │       │       ├── tokio feature "default" (*)
│   │   │   │   │   │   │   │       │       ├── tokio feature "sync" (*)
│   │   │   │   │   │   │   │       │       ├── bytes feature "default" (*)
│   │   │   │   │   │   │   │       │       ├── pin-project-lite feature "default" (*)
│   │   │   │   │   │   │   │       │       ├── futures-util feature "default"
│   │   │   │   │   │   │   │       │       │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   │       │       │   ├── futures-util feature "async-await"
│   │   │   │   │   │   │   │       │       │   │   └── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   │       │       │   ├── futures-util feature "async-await-macro"
│   │   │   │   │   │   │   │       │       │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   │       │       │   │   ├── futures-util feature "async-await" (*)
│   │   │   │   │   │   │   │       │       │   │   └── futures-util feature "futures-macro"
│   │   │   │   │   │   │   │       │       │   │       └── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   │       │       │   └── futures-util feature "std" (*)
│   │   │   │   │   │   │   │       │       ├── futures-core feature "default"
│   │   │   │   │   │   │   │       │       │   ├── futures-core v0.3.32
│   │   │   │   │   │   │   │       │       │   └── futures-core feature "std" (*)
│   │   │   │   │   │   │   │       │       └── futures-sink feature "default"
│   │   │   │   │   │   │   │       │           ├── futures-sink v0.3.32
│   │   │   │   │   │   │   │       │           └── futures-sink feature "std"
│   │   │   │   │   │   │   │       │               ├── futures-sink v0.3.32
│   │   │   │   │   │   │   │       │               └── futures-sink feature "alloc"
│   │   │   │   │   │   │   │       │                   └── futures-sink v0.3.32
│   │   │   │   │   │   │   │       ├── tokio-util feature "default"
│   │   │   │   │   │   │   │       │   └── tokio-util v0.7.18 (*)
│   │   │   │   │   │   │   │       ├── tokio-util feature "io"
│   │   │   │   │   │   │   │       │   └── tokio-util v0.7.18 (*)
│   │   │   │   │   │   │   │       └── tracing feature "std" (*)
│   │   │   │   │   │   │   ├── h2 feature "stream"
│   │   │   │   │   │   │   │   └── h2 v0.4.13 (*)
│   │   │   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   │   │   ├── once_cell feature "critical-section"
│   │   │   │   │   │   │   │   ├── once_cell v1.21.4 (*)
│   │   │   │   │   │   │   │   └── once_cell feature "portable-atomic"
│   │   │   │   │   │   │   │       └── once_cell v1.21.4 (*)
│   │   │   │   │   │   │   ├── idna feature "alloc" (*)
│   │   │   │   │   │   │   ├── idna feature "compiled_data" (*)
│   │   │   │   │   │   │   ├── rand feature "alloc"
│   │   │   │   │   │   │   │   └── rand v0.9.2
│   │   │   │   │   │   │   │       ├── rand_chacha v0.9.0
│   │   │   │   │   │   │   │       │   ├── rand_core feature "default" (*)
│   │   │   │   │   │   │   │       │   └── ppv-lite86 feature "simd"
│   │   │   │   │   │   │   │       │       └── ppv-lite86 v0.2.21
│   │   │   │   │   │   │   │       │           ├── zerocopy feature "default"
│   │   │   │   │   │   │   │       │           │   └── zerocopy v0.8.42
│   │   │   │   │   │   │   │       │           └── zerocopy feature "simd"
│   │   │   │   │   │   │   │       │               └── zerocopy v0.8.42
│   │   │   │   │   │   │   │       └── rand_core v0.9.5 (*)
│   │   │   │   │   │   │   ├── rand feature "std_rng"
│   │   │   │   │   │   │   │   └── rand v0.9.2 (*)
│   │   │   │   │   │   │   ├── rustls feature "logging"
│   │   │   │   │   │   │   │   ├── rustls v0.23.37 (*)
│   │   │   │   │   │   │   │   └── rustls feature "log"
│   │   │   │   │   │   │   │       └── rustls v0.23.37 (*)
│   │   │   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   │   │   ├── rustls feature "tls12"
│   │   │   │   │   │   │   │   └── rustls v0.23.37 (*)
│   │   │   │   │   │   │   ├── tinyvec feature "alloc"
│   │   │   │   │   │   │   │   ├── tinyvec v1.11.0
│   │   │   │   │   │   │   │   │   └── tinyvec_macros feature "default"
│   │   │   │   │   │   │   │   │       └── tinyvec_macros v0.1.1
│   │   │   │   │   │   │   │   └── tinyvec feature "tinyvec_macros"
│   │   │   │   │   │   │   │       └── tinyvec v1.11.0 (*)
│   │   │   │   │   │   │   ├── tinyvec feature "default"
│   │   │   │   │   │   │   │   └── tinyvec v1.11.0 (*)
│   │   │   │   │   │   │   └── tokio-rustls feature "early-data"
│   │   │   │   │   │   │       └── tokio-rustls v0.26.4 (*)
│   │   │   │   │   │   ├── thiserror feature "std"
│   │   │   │   │   │   │   └── thiserror v2.0.18 (*)
│   │   │   │   │   │   ├── data-encoding feature "std" (*)
│   │   │   │   │   │   ├── futures-util feature "std" (*)
│   │   │   │   │   │   ├── futures-channel feature "std" (*)
│   │   │   │   │   │   ├── futures-io feature "std" (*)
│   │   │   │   │   │   ├── hickory-proto feature "futures-io"
│   │   │   │   │   │   │   └── hickory-proto v0.25.2 (*)
│   │   │   │   │   │   ├── tracing feature "std" (*)
│   │   │   │   │   │   ├── ipnet feature "std"
│   │   │   │   │   │   │   └── ipnet v2.12.0
│   │   │   │   │   │   ├── rand feature "std"
│   │   │   │   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   │   │   │   ├── rand_core feature "std"
│   │   │   │   │   │   │   │   ├── rand_core v0.9.5 (*)
│   │   │   │   │   │   │   │   └── getrandom feature "std"
│   │   │   │   │   │   │   │       └── getrandom v0.3.4 (*)
│   │   │   │   │   │   │   ├── rand feature "alloc" (*)
│   │   │   │   │   │   │   └── rand_chacha feature "std"
│   │   │   │   │   │   │       ├── rand_chacha v0.9.0 (*)
│   │   │   │   │   │   │       ├── rand_core feature "std" (*)
│   │   │   │   │   │   │       └── ppv-lite86 feature "std"
│   │   │   │   │   │   │           └── ppv-lite86 v0.2.21 (*)
│   │   │   │   │   │   ├── rand feature "thread_rng"
│   │   │   │   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   │   │   │   ├── rand feature "os_rng"
│   │   │   │   │   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   │   │   │   │   └── rand_core feature "os_rng"
│   │   │   │   │   │   │   │       └── rand_core v0.9.5 (*)
│   │   │   │   │   │   │   ├── rand feature "std" (*)
│   │   │   │   │   │   │   └── rand feature "std_rng" (*)
│   │   │   │   │   │   └── url feature "std" (*)
│   │   │   │   │   ├── once_cell feature "critical-section" (*)
│   │   │   │   │   ├── smallvec feature "default" (*)
│   │   │   │   │   ├── rand feature "alloc" (*)
│   │   │   │   │   ├── rustls feature "logging" (*)
│   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   ├── rustls feature "tls12" (*)
│   │   │   │   │   ├── moka feature "default"
│   │   │   │   │   │   └── moka v0.12.14
│   │   │   │   │   │       ├── equivalent feature "default"
│   │   │   │   │   │       │   └── equivalent v1.0.2
│   │   │   │   │   │       ├── portable-atomic feature "default" (*)
│   │   │   │   │   │       ├── smallvec feature "default" (*)
│   │   │   │   │   │       ├── crossbeam-channel feature "default"
│   │   │   │   │   │       │   ├── crossbeam-channel v0.5.15
│   │   │   │   │   │       │   │   └── crossbeam-utils v0.8.21
│   │   │   │   │   │       │   └── crossbeam-channel feature "std"
│   │   │   │   │   │       │       ├── crossbeam-channel v0.5.15 (*)
│   │   │   │   │   │       │       └── crossbeam-utils feature "std"
│   │   │   │   │   │       │           └── crossbeam-utils v0.8.21
│   │   │   │   │   │       ├── crossbeam-utils feature "default"
│   │   │   │   │   │       │   ├── crossbeam-utils v0.8.21
│   │   │   │   │   │       │   └── crossbeam-utils feature "std" (*)
│   │   │   │   │   │       ├── crossbeam-epoch feature "default"
│   │   │   │   │   │       │   ├── crossbeam-epoch v0.9.18
│   │   │   │   │   │       │   │   └── crossbeam-utils v0.8.21
│   │   │   │   │   │       │   └── crossbeam-epoch feature "std"
│   │   │   │   │   │       │       ├── crossbeam-epoch v0.9.18 (*)
│   │   │   │   │   │       │       ├── crossbeam-utils feature "std" (*)
│   │   │   │   │   │       │       └── crossbeam-epoch feature "alloc"
│   │   │   │   │   │       │           └── crossbeam-epoch v0.9.18 (*)
│   │   │   │   │   │       ├── parking_lot feature "default"
│   │   │   │   │   │       │   └── parking_lot v0.12.5
│   │   │   │   │   │       │       ├── lock_api feature "default" (*)
│   │   │   │   │   │       │       └── parking_lot_core feature "default"
│   │   │   │   │   │       │           └── parking_lot_core v0.9.12
│   │   │   │   │   │       │               ├── libc feature "default" (*)
│   │   │   │   │   │       │               ├── cfg-if feature "default" (*)
│   │   │   │   │   │       │               └── smallvec feature "default" (*)
│   │   │   │   │   │       ├── tagptr feature "default"
│   │   │   │   │   │       │   └── tagptr v0.2.0
│   │   │   │   │   │       ├── uuid feature "default"
│   │   │   │   │   │       │   ├── uuid v1.22.0
│   │   │   │   │   │       │   │   └── getrandom feature "default"
│   │   │   │   │   │       │   │       └── getrandom v0.4.2
│   │   │   │   │   │       │   │           ├── libc v0.2.183
│   │   │   │   │   │       │   │           └── cfg-if feature "default" (*)
│   │   │   │   │   │       │   └── uuid feature "std"
│   │   │   │   │   │       │       └── uuid v1.22.0 (*)
│   │   │   │   │   │       └── uuid feature "v4"
│   │   │   │   │   │           ├── uuid v1.22.0 (*)
│   │   │   │   │   │           └── uuid feature "rng"
│   │   │   │   │   │               └── uuid v1.22.0 (*)
│   │   │   │   │   ├── moka feature "sync"
│   │   │   │   │   │   └── moka v0.12.14 (*)
│   │   │   │   │   ├── parking_lot feature "default" (*)
│   │   │   │   │   ├── resolv-conf feature "default"
│   │   │   │   │   │   └── resolv-conf v0.7.6
│   │   │   │   │   └── resolv-conf feature "system"
│   │   │   │   │       └── resolv-conf v0.7.6
│   │   │   │   ├── hickory-resolver feature "system-config"
│   │   │   │   │   └── hickory-resolver v0.25.2 (*)
│   │   │   │   └── hickory-resolver feature "tokio"
│   │   │   │       ├── hickory-resolver v0.25.2 (*)
│   │   │   │       ├── tokio feature "rt" (*)
│   │   │   │       ├── hickory-resolver feature "tokio" (*)
│   │   │   │       └── hickory-proto feature "tokio"
│   │   │   │           ├── hickory-proto v0.25.2 (*)
│   │   │   │           ├── tokio feature "net" (*)
│   │   │   │           ├── tokio feature "rt" (*)
│   │   │   │           ├── tokio feature "rt-multi-thread"
│   │   │   │           │   ├── tokio v1.50.0 (*)
│   │   │   │           │   └── tokio feature "rt" (*)
│   │   │   │           ├── tokio feature "time"
│   │   │   │           │   └── tokio v1.50.0 (*)
│   │   │   │           ├── hickory-proto feature "std" (*)
│   │   │   │           └── hickory-proto feature "tokio" (*)
│   │   │   ├── hickory-resolver feature "https-ring"
│   │   │   │   ├── hickory-resolver v0.25.2 (*)
│   │   │   │   ├── hickory-resolver feature "__https"
│   │   │   │   │   ├── hickory-resolver v0.25.2 (*)
│   │   │   │   │   └── hickory-resolver feature "__tls"
│   │   │   │   │       ├── hickory-resolver v0.25.2 (*)
│   │   │   │   │       └── hickory-resolver feature "tokio" (*)
│   │   │   │   └── hickory-proto feature "https-ring"
│   │   │   │       ├── hickory-proto v0.25.2 (*)
│   │   │   │       ├── hickory-proto feature "__https"
│   │   │   │       │   ├── hickory-proto v0.25.2 (*)
│   │   │   │       │   └── hickory-proto feature "std" (*)
│   │   │   │       └── hickory-proto feature "tls-ring"
│   │   │   │           ├── hickory-proto v0.25.2 (*)
│   │   │   │           ├── hickory-proto feature "__tls"
│   │   │   │           │   ├── hickory-proto v0.25.2 (*)
│   │   │   │           │   ├── hickory-proto feature "std" (*)
│   │   │   │           │   └── hickory-proto feature "tokio" (*)
│   │   │   │           ├── hickory-proto feature "tokio-rustls"
│   │   │   │           │   └── hickory-proto v0.25.2 (*)
│   │   │   │           └── tokio-rustls feature "ring"
│   │   │   │               ├── tokio-rustls v0.26.4 (*)
│   │   │   │               └── rustls feature "ring"
│   │   │   │                   ├── rustls v0.23.37 (*)
│   │   │   │                   └── rustls-webpki feature "ring"
│   │   │   │                       └── rustls-webpki v0.103.9 (*)
│   │   │   ├── hickory-resolver feature "tokio" (*)
│   │   │   ├── http feature "default" (*)
│   │   │   ├── tokio-util feature "codec" (*)
│   │   │   ├── tokio-util feature "default" (*)
│   │   │   ├── tokio-util feature "io" (*)
│   │   │   ├── tokio-util feature "io-util"
│   │   │   │   ├── tokio-util v0.7.18 (*)
│   │   │   │   ├── tokio feature "io-util" (*)
│   │   │   │   ├── tokio feature "rt" (*)
│   │   │   │   └── tokio-util feature "io" (*)
│   │   │   ├── tokio-util feature "rt"
│   │   │   │   ├── tokio-util v0.7.18 (*)
│   │   │   │   ├── tokio feature "rt" (*)
│   │   │   │   ├── tokio feature "sync" (*)
│   │   │   │   └── tokio-util feature "futures-util"
│   │   │   │       └── tokio-util v0.7.18 (*)
│   │   │   ├── tracing feature "default" (*)
│   │   │   ├── rand feature "default"
│   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   ├── rand feature "os_rng" (*)
│   │   │   │   ├── rand feature "small_rng"
│   │   │   │   │   └── rand v0.9.2 (*)
│   │   │   │   ├── rand feature "std" (*)
│   │   │   │   ├── rand feature "std_rng" (*)
│   │   │   │   └── rand feature "thread_rng" (*)
│   │   │   ├── rustls feature "ring" (*)
│   │   │   ├── rustls-pki-types feature "default" (*)
│   │   │   ├── tokio-rustls feature "logging"
│   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   └── rustls feature "logging" (*)
│   │   │   ├── tokio-rustls feature "ring" (*)
│   │   │   ├── url feature "default" (*)
│   │   │   ├── url feature "serde" (*)
│   │   │   ├── iroh-base feature "key" (*)
│   │   │   ├── iroh-base feature "relay" (*)
│   │   │   ├── n0-error feature "default" (*)
│   │   │   ├── postcard feature "alloc" (*)
│   │   │   ├── postcard feature "experimental-derive"
│   │   │   │   ├── postcard v1.1.3 (*)
│   │   │   │   └── postcard feature "postcard-derive"
│   │   │   │       └── postcard v1.1.3 (*)
│   │   │   ├── postcard feature "use-std" (*)
│   │   │   ├── blake3 feature "default"
│   │   │   │   ├── blake3 v1.8.3
│   │   │   │   │   ├── arrayvec v0.7.6
│   │   │   │   │   ├── constant_time_eq v0.4.2
│   │   │   │   │   ├── cfg-if feature "default" (*)
│   │   │   │   │   ├── cpufeatures feature "default" (*)
│   │   │   │   │   └── arrayref feature "default"
│   │   │   │   │       └── arrayref v0.3.9
│   │   │   │   │   [build-dependencies]
│   │   │   │   │   └── cc feature "default"
│   │   │   │   │       └── cc v1.2.57 (*)
│   │   │   │   └── blake3 feature "std"
│   │   │   │       ├── blake3 v1.8.3 (*)
│   │   │   │       └── constant_time_eq feature "std"
│   │   │   │           └── constant_time_eq v0.4.2
│   │   │   ├── http-body-util feature "default"
│   │   │   │   └── http-body-util v0.1.3
│   │   │   │       ├── futures-core v0.3.32
│   │   │   │       ├── bytes feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── http feature "default" (*)
│   │   │   │       └── http-body feature "default"
│   │   │   │           └── http-body v1.0.1
│   │   │   │               ├── bytes feature "default" (*)
│   │   │   │               └── http feature "default" (*)
│   │   │   ├── hyper feature "client"
│   │   │   │   └── hyper v1.8.1
│   │   │   │       ├── tokio feature "default" (*)
│   │   │   │       ├── tokio feature "sync" (*)
│   │   │   │       ├── bytes feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── futures-channel feature "default"
│   │   │   │       │   ├── futures-channel v0.3.32 (*)
│   │   │   │       │   └── futures-channel feature "std" (*)
│   │   │   │       ├── futures-core feature "default" (*)
│   │   │   │       ├── h2 feature "default" (*)
│   │   │   │       ├── atomic-waker feature "default" (*)
│   │   │   │       ├── http feature "default" (*)
│   │   │   │       ├── itoa feature "default" (*)
│   │   │   │       ├── smallvec feature "const_generics" (*)
│   │   │   │       ├── smallvec feature "const_new"
│   │   │   │       │   ├── smallvec v1.15.1
│   │   │   │       │   └── smallvec feature "const_generics" (*)
│   │   │   │       ├── smallvec feature "default" (*)
│   │   │   │       ├── http-body feature "default" (*)
│   │   │   │       ├── httparse feature "default"
│   │   │   │       │   ├── httparse v1.10.1
│   │   │   │       │   └── httparse feature "std"
│   │   │   │       │       └── httparse v1.10.1
│   │   │   │       ├── httpdate feature "default"
│   │   │   │       │   └── httpdate v1.0.3
│   │   │   │       ├── pin-utils feature "default"
│   │   │   │       │   └── pin-utils v0.1.0
│   │   │   │       └── want feature "default"
│   │   │   │           └── want v0.3.1
│   │   │   │               └── try-lock feature "default"
│   │   │   │                   └── try-lock v0.2.5
│   │   │   ├── hyper feature "default"
│   │   │   │   └── hyper v1.8.1 (*)
│   │   │   ├── hyper feature "http1"
│   │   │   │   └── hyper v1.8.1 (*)
│   │   │   ├── hyper feature "server"
│   │   │   │   └── hyper v1.8.1 (*)
│   │   │   ├── hyper-util feature "default"
│   │   │   │   └── hyper-util v0.1.20
│   │   │   │       ├── futures-util v0.3.32 (*)
│   │   │   │       ├── tokio v1.50.0 (*)
│   │   │   │       ├── bytes feature "default" (*)
│   │   │   │       ├── libc feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── socket2 feature "all" (*)
│   │   │   │       ├── socket2 feature "default" (*)
│   │   │   │       ├── futures-channel feature "default" (*)
│   │   │   │       ├── http feature "default" (*)
│   │   │   │       ├── tracing feature "std" (*)
│   │   │   │       ├── ipnet feature "default"
│   │   │   │       │   ├── ipnet v2.12.0
│   │   │   │       │   └── ipnet feature "std" (*)
│   │   │   │       ├── percent-encoding feature "default"
│   │   │   │       │   ├── percent-encoding v2.3.2
│   │   │   │       │   └── percent-encoding feature "std" (*)
│   │   │   │       ├── http-body feature "default" (*)
│   │   │   │       ├── hyper feature "default" (*)
│   │   │   │       ├── base64 feature "default"
│   │   │   │       │   ├── base64 v0.22.1
│   │   │   │       │   └── base64 feature "std"
│   │   │   │       │       ├── base64 v0.22.1
│   │   │   │       │       └── base64 feature "alloc"
│   │   │   │       │           └── base64 v0.22.1
│   │   │   │       └── tower-service feature "default"
│   │   │   │           └── tower-service v0.3.3
│   │   │   ├── lru feature "default"
│   │   │   │   ├── lru v0.16.3
│   │   │   │   │   └── hashbrown feature "default"
│   │   │   │   │       ├── hashbrown v0.16.1 (*)
│   │   │   │   │       ├── hashbrown feature "allocator-api2"
│   │   │   │   │       │   └── hashbrown v0.16.1 (*)
│   │   │   │   │       ├── hashbrown feature "default-hasher"
│   │   │   │   │       │   └── hashbrown v0.16.1 (*)
│   │   │   │   │       ├── hashbrown feature "equivalent"
│   │   │   │   │       │   └── hashbrown v0.16.1 (*)
│   │   │   │   │       ├── hashbrown feature "inline-more"
│   │   │   │   │       │   └── hashbrown v0.16.1 (*)
│   │   │   │   │       └── hashbrown feature "raw-entry"
│   │   │   │   │           └── hashbrown v0.16.1 (*)
│   │   │   │   └── lru feature "hashbrown"
│   │   │   │       └── lru v0.16.3 (*)
│   │   │   ├── n0-future feature "default"
│   │   │   │   └── n0-future v0.3.2
│   │   │   │       ├── tokio feature "default" (*)
│   │   │   │       ├── tokio feature "macros" (*)
│   │   │   │       ├── tokio feature "rt" (*)
│   │   │   │       ├── tokio feature "test-util"
│   │   │   │       │   ├── tokio v1.50.0 (*)
│   │   │   │       │   ├── tokio feature "rt" (*)
│   │   │   │       │   ├── tokio feature "sync" (*)
│   │   │   │       │   └── tokio feature "time" (*)
│   │   │   │       ├── tokio feature "time" (*)
│   │   │   │       ├── derive_more feature "debug" (*)
│   │   │   │       ├── derive_more feature "default" (*)
│   │   │   │       ├── derive_more feature "deref" (*)
│   │   │   │       ├── derive_more feature "display" (*)
│   │   │   │       ├── futures-util feature "default" (*)
│   │   │   │       ├── futures-util feature "sink"
│   │   │   │       │   ├── futures-util v0.3.32 (*)
│   │   │   │       │   └── futures-util feature "futures-sink"
│   │   │   │       │       └── futures-util v0.3.32 (*)
│   │   │   │       ├── tokio-util feature "default" (*)
│   │   │   │       ├── tokio-util feature "rt" (*)
│   │   │   │       ├── futures-buffered feature "default"
│   │   │   │       │   └── futures-buffered v0.2.13
│   │   │   │       │       ├── futures-core v0.3.32
│   │   │   │       │       ├── pin-project-lite feature "default" (*)
│   │   │   │       │       ├── cordyceps feature "default"
│   │   │   │       │       │   └── cordyceps v0.3.4
│   │   │   │       │       ├── diatomic-waker feature "default"
│   │   │   │       │       │   ├── diatomic-waker v0.2.3
│   │   │   │       │       │   └── diatomic-waker feature "alloc"
│   │   │   │       │       │       └── diatomic-waker v0.2.3
│   │   │   │       │       └── spin feature "spin_mutex"
│   │   │   │       │           ├── spin v0.10.0
│   │   │   │       │           └── spin feature "mutex"
│   │   │   │       │               └── spin v0.10.0
│   │   │   │       ├── futures-lite feature "default"
│   │   │   │       │   ├── futures-lite v2.6.1
│   │   │   │       │   │   ├── fastrand v2.3.0
│   │   │   │       │   │   ├── futures-core v0.3.32
│   │   │   │       │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │       │   │   ├── futures-io feature "default"
│   │   │   │       │   │   │   ├── futures-io v0.3.32
│   │   │   │       │   │   │   └── futures-io feature "std" (*)
│   │   │   │       │   │   └── parking feature "default"
│   │   │   │       │   │       └── parking v2.2.1
│   │   │   │       │   ├── futures-lite feature "race"
│   │   │   │       │   │   ├── futures-lite v2.6.1 (*)
│   │   │   │       │   │   └── futures-lite feature "fastrand"
│   │   │   │       │   │       └── futures-lite v2.6.1 (*)
│   │   │   │       │   └── futures-lite feature "std"
│   │   │   │       │       ├── futures-lite v2.6.1 (*)
│   │   │   │       │       ├── fastrand feature "std"
│   │   │   │       │       │   ├── fastrand v2.3.0
│   │   │   │       │       │   └── fastrand feature "alloc"
│   │   │   │       │       │       └── fastrand v2.3.0
│   │   │   │       │       ├── futures-lite feature "alloc"
│   │   │   │       │       │   └── futures-lite v2.6.1 (*)
│   │   │   │       │       ├── futures-lite feature "fastrand" (*)
│   │   │   │       │       ├── futures-lite feature "futures-io"
│   │   │   │       │       │   └── futures-lite v2.6.1 (*)
│   │   │   │       │       └── futures-lite feature "parking"
│   │   │   │       │           └── futures-lite v2.6.1 (*)
│   │   │   │       └── pin-project feature "default"
│   │   │   │           └── pin-project v1.1.11
│   │   │   │               └── pin-project-internal feature "default"
│   │   │   │                   └── pin-project-internal v1.1.11 (proc-macro)
│   │   │   │                       ├── proc-macro2 feature "default" (*)
│   │   │   │                       ├── quote feature "default" (*)
│   │   │   │                       ├── syn feature "clone-impls" (*)
│   │   │   │                       ├── syn feature "full" (*)
│   │   │   │                       ├── syn feature "parsing" (*)
│   │   │   │                       ├── syn feature "printing" (*)
│   │   │   │                       ├── syn feature "proc-macro" (*)
│   │   │   │                       └── syn feature "visit-mut" (*)
│   │   │   │       [build-dependencies]
│   │   │   │       └── cfg_aliases feature "default"
│   │   │   │           └── cfg_aliases v0.2.1
│   │   │   ├── pin-project feature "default" (*)
│   │   │   ├── noq feature "rustls-ring"
│   │   │   │   ├── noq v0.17.0
│   │   │   │   │   ├── noq-proto v0.16.0
│   │   │   │   │   │   ├── thiserror feature "default"
│   │   │   │   │   │   │   ├── thiserror v2.0.18 (*)
│   │   │   │   │   │   │   └── thiserror feature "std" (*)
│   │   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   │   ├── derive_more feature "debug" (*)
│   │   │   │   │   │   ├── derive_more feature "default" (*)
│   │   │   │   │   │   ├── derive_more feature "deref" (*)
│   │   │   │   │   │   ├── derive_more feature "deref_mut"
│   │   │   │   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   │   │   │   └── derive_more-impl feature "deref_mut"
│   │   │   │   │   │   │       └── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │   │   │   │   ├── derive_more feature "display" (*)
│   │   │   │   │   │   ├── derive_more feature "from" (*)
│   │   │   │   │   │   ├── slab feature "default" (*)
│   │   │   │   │   │   ├── tracing feature "std" (*)
│   │   │   │   │   │   ├── rand feature "default" (*)
│   │   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   │   ├── ring feature "default" (*)
│   │   │   │   │   │   ├── tinyvec feature "alloc" (*)
│   │   │   │   │   │   ├── tinyvec feature "default" (*)
│   │   │   │   │   │   ├── aes-gcm feature "aes"
│   │   │   │   │   │   │   └── aes-gcm v0.10.3
│   │   │   │   │   │   │       ├── aead v0.5.2
│   │   │   │   │   │   │       │   ├── generic-array v0.14.7
│   │   │   │   │   │   │       │   │   └── typenum feature "default" (*)
│   │   │   │   │   │   │       │   │   [build-dependencies]
│   │   │   │   │   │   │       │   │   └── version_check feature "default"
│   │   │   │   │   │   │       │   │       └── version_check v0.9.5
│   │   │   │   │   │   │       │   └── crypto-common feature "default"
│   │   │   │   │   │   │       │       └── crypto-common v0.1.7
│   │   │   │   │   │   │       │           ├── typenum feature "default" (*)
│   │   │   │   │   │   │       │           ├── generic-array feature "default"
│   │   │   │   │   │   │       │           │   └── generic-array v0.14.7 (*)
│   │   │   │   │   │   │       │           └── generic-array feature "more_lengths"
│   │   │   │   │   │   │       │               └── generic-array v0.14.7 (*)
│   │   │   │   │   │   │       ├── ghash v0.5.1
│   │   │   │   │   │   │       │   ├── opaque-debug feature "default"
│   │   │   │   │   │   │       │   │   └── opaque-debug v0.3.1
│   │   │   │   │   │   │       │   └── polyval feature "default"
│   │   │   │   │   │   │       │       └── polyval v0.6.2
│   │   │   │   │   │   │       │           ├── universal-hash v0.5.1
│   │   │   │   │   │   │       │           │   ├── subtle v2.6.1
│   │   │   │   │   │   │       │           │   └── crypto-common feature "default" (*)
│   │   │   │   │   │   │       │           ├── cfg-if feature "default" (*)
│   │   │   │   │   │   │       │           ├── cpufeatures feature "default" (*)
│   │   │   │   │   │   │       │           └── opaque-debug feature "default" (*)
│   │   │   │   │   │   │       ├── subtle v2.6.1
│   │   │   │   │   │   │       ├── aes feature "default"
│   │   │   │   │   │   │       │   └── aes v0.8.4
│   │   │   │   │   │   │       │       ├── cfg-if feature "default" (*)
│   │   │   │   │   │   │       │       ├── cpufeatures feature "default" (*)
│   │   │   │   │   │   │       │       └── cipher feature "default"
│   │   │   │   │   │   │       │           └── cipher v0.4.4
│   │   │   │   │   │   │       │               ├── crypto-common feature "default" (*)
│   │   │   │   │   │   │       │               └── inout feature "default"
│   │   │   │   │   │   │       │                   └── inout v0.1.4
│   │   │   │   │   │   │       │                       └── generic-array feature "default" (*)
│   │   │   │   │   │   │       ├── cipher feature "default" (*)
│   │   │   │   │   │   │       └── ctr feature "default"
│   │   │   │   │   │   │           └── ctr v0.9.2
│   │   │   │   │   │   │               └── cipher feature "default" (*)
│   │   │   │   │   │   ├── enum-assoc feature "default"
│   │   │   │   │   │   │   └── enum-assoc v1.3.0 (proc-macro)
│   │   │   │   │   │   │       ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   │       ├── quote feature "default" (*)
│   │   │   │   │   │   │       ├── syn feature "default" (*)
│   │   │   │   │   │   │       └── syn feature "full" (*)
│   │   │   │   │   │   ├── fastbloom feature "default"
│   │   │   │   │   │   │   ├── fastbloom v0.14.1
│   │   │   │   │   │   │   │   ├── getrandom feature "default" (*)
│   │   │   │   │   │   │   │   ├── rand feature "default" (*)
│   │   │   │   │   │   │   │   ├── libm feature "default"
│   │   │   │   │   │   │   │   │   ├── libm v0.2.16
│   │   │   │   │   │   │   │   │   └── libm feature "arch"
│   │   │   │   │   │   │   │   │       └── libm v0.2.16
│   │   │   │   │   │   │   │   └── siphasher feature "default"
│   │   │   │   │   │   │   │       ├── siphasher v1.0.2
│   │   │   │   │   │   │   │       └── siphasher feature "std"
│   │   │   │   │   │   │   │           └── siphasher v1.0.2
│   │   │   │   │   │   │   ├── fastbloom feature "rand"
│   │   │   │   │   │   │   │   └── fastbloom v0.14.1 (*)
│   │   │   │   │   │   │   └── fastbloom feature "std"
│   │   │   │   │   │   │       └── fastbloom v0.14.1 (*)
│   │   │   │   │   │   ├── identity-hash feature "default"
│   │   │   │   │   │   │   ├── identity-hash v0.1.0
│   │   │   │   │   │   │   └── identity-hash feature "std"
│   │   │   │   │   │   │       └── identity-hash v0.1.0
│   │   │   │   │   │   ├── lru-slab feature "default"
│   │   │   │   │   │   │   └── lru-slab v0.1.2
│   │   │   │   │   │   ├── rustc-hash feature "default"
│   │   │   │   │   │   │   ├── rustc-hash v2.1.1
│   │   │   │   │   │   │   └── rustc-hash feature "std"
│   │   │   │   │   │   │       └── rustc-hash v2.1.1
│   │   │   │   │   │   └── sorted-index-buffer feature "default"
│   │   │   │   │   │       └── sorted-index-buffer v0.2.1
│   │   │   │   │   ├── thiserror feature "default" (*)
│   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   ├── tokio feature "sync" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   ├── socket2 feature "default" (*)
│   │   │   │   │   ├── tracing feature "std" (*)
│   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   ├── rustc-hash feature "default" (*)
│   │   │   │   │   ├── noq-udp feature "tracing"
│   │   │   │   │   │   └── noq-udp v0.9.0
│   │   │   │   │   │       ├── libc feature "default" (*)
│   │   │   │   │   │       ├── socket2 feature "default" (*)
│   │   │   │   │   │       └── tracing feature "std" (*)
│   │   │   │   │   │       [build-dependencies]
│   │   │   │   │   │       └── cfg_aliases feature "default" (*)
│   │   │   │   │   ├── tokio-stream feature "default"
│   │   │   │   │   │   ├── tokio-stream v0.1.18
│   │   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   │   ├── tokio feature "sync" (*)
│   │   │   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   │   │   ├── futures-core feature "default" (*)
│   │   │   │   │   │   │   └── tokio-util feature "default" (*)
│   │   │   │   │   │   └── tokio-stream feature "time"
│   │   │   │   │   │       ├── tokio-stream v0.1.18 (*)
│   │   │   │   │   │       └── tokio feature "time" (*)
│   │   │   │   │   └── tokio-stream feature "sync"
│   │   │   │   │       ├── tokio-stream v0.1.18 (*)
│   │   │   │   │       ├── tokio feature "sync" (*)
│   │   │   │   │       └── tokio-stream feature "tokio-util"
│   │   │   │   │           └── tokio-stream v0.1.18 (*)
│   │   │   │   │   [build-dependencies]
│   │   │   │   │   └── cfg_aliases feature "default" (*)
│   │   │   │   ├── noq feature "ring"
│   │   │   │   │   ├── noq v0.17.0 (*)
│   │   │   │   │   └── noq-proto feature "ring"
│   │   │   │   │       ├── noq-proto v0.16.0 (*)
│   │   │   │   │       └── rustls feature "ring" (*)
│   │   │   │   └── noq feature "rustls"
│   │   │   │       ├── noq v0.17.0 (*)
│   │   │   │       └── noq-proto feature "rustls"
│   │   │   │           └── noq-proto v0.16.0 (*)
│   │   │   ├── noq-proto feature "default"
│   │   │   │   ├── noq-proto v0.16.0 (*)
│   │   │   │   ├── noq-proto feature "bloom"
│   │   │   │   │   └── noq-proto v0.16.0 (*)
│   │   │   │   ├── noq-proto feature "ring" (*)
│   │   │   │   ├── noq-proto feature "rustls" (*)
│   │   │   │   └── noq-proto feature "tracing-log"
│   │   │   │       ├── noq-proto v0.16.0 (*)
│   │   │   │       └── tracing feature "log"
│   │   │   │           └── tracing v0.1.44 (*)
│   │   │   ├── num_enum feature "default"
│   │   │   │   ├── num_enum v0.7.6
│   │   │   │   │   ├── num_enum_derive v0.7.6 (proc-macro)
│   │   │   │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   ├── quote feature "default" (*)
│   │   │   │   │   │   ├── syn feature "default" (*)
│   │   │   │   │   │   ├── syn feature "derive" (*)
│   │   │   │   │   │   ├── syn feature "extra-traits" (*)
│   │   │   │   │   │   ├── syn feature "parsing" (*)
│   │   │   │   │   │   └── proc-macro-crate feature "default"
│   │   │   │   │   │       └── proc-macro-crate v3.5.0
│   │   │   │   │   │           └── toml_edit feature "parse"
│   │   │   │   │   │               └── toml_edit v0.25.5+spec-1.1.0
│   │   │   │   │   │                   ├── indexmap feature "default" (*)
│   │   │   │   │   │                   ├── indexmap feature "std" (*)
│   │   │   │   │   │                   ├── toml_datetime feature "default"
│   │   │   │   │   │                   │   ├── toml_datetime v1.0.1+spec-1.1.0
│   │   │   │   │   │                   │   └── toml_datetime feature "std"
│   │   │   │   │   │                   │       ├── toml_datetime v1.0.1+spec-1.1.0
│   │   │   │   │   │                   │       └── toml_datetime feature "alloc"
│   │   │   │   │   │                   │           └── toml_datetime v1.0.1+spec-1.1.0
│   │   │   │   │   │                   ├── toml_parser feature "default"
│   │   │   │   │   │                   │   ├── toml_parser v1.0.10+spec-1.1.0
│   │   │   │   │   │                   │   │   └── winnow v1.0.0
│   │   │   │   │   │                   │   └── toml_parser feature "std"
│   │   │   │   │   │                   │       ├── toml_parser v1.0.10+spec-1.1.0 (*)
│   │   │   │   │   │                   │       └── toml_parser feature "alloc"
│   │   │   │   │   │                   │           └── toml_parser v1.0.10+spec-1.1.0 (*)
│   │   │   │   │   │                   └── winnow feature "default"
│   │   │   │   │   │                       ├── winnow v1.0.0
│   │   │   │   │   │                       ├── winnow feature "ascii"
│   │   │   │   │   │                       │   ├── winnow v1.0.0
│   │   │   │   │   │                       │   └── winnow feature "parser"
│   │   │   │   │   │                       │       └── winnow v1.0.0
│   │   │   │   │   │                       ├── winnow feature "binary"
│   │   │   │   │   │                       │   ├── winnow v1.0.0
│   │   │   │   │   │                       │   └── winnow feature "parser" (*)
│   │   │   │   │   │                       └── winnow feature "std"
│   │   │   │   │   │                           ├── winnow v1.0.0
│   │   │   │   │   │                           └── winnow feature "alloc"
│   │   │   │   │   │                               └── winnow v1.0.0
│   │   │   │   │   └── rustversion feature "default"
│   │   │   │   │       └── rustversion v1.0.22 (proc-macro)
│   │   │   │   └── num_enum feature "std"
│   │   │   │       ├── num_enum v0.7.6 (*)
│   │   │   │       └── num_enum_derive feature "std"
│   │   │   │           ├── num_enum_derive v0.7.6 (proc-macro) (*)
│   │   │   │           └── num_enum_derive feature "proc-macro-crate"
│   │   │   │               └── num_enum_derive v0.7.6 (proc-macro) (*)
│   │   │   ├── pkarr feature "signed_packet"
│   │   │   │   ├── pkarr v5.0.2
│   │   │   │   │   ├── getrandom v0.3.4 (*)
│   │   │   │   │   ├── serde feature "default" (*)
│   │   │   │   │   ├── serde feature "derive" (*)
│   │   │   │   │   ├── thiserror feature "default" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── ed25519-dalek feature "alloc"
│   │   │   │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│   │   │   │   │   │   ├── serde feature "alloc" (*)
│   │   │   │   │   │   ├── ed25519-dalek feature "signature"
│   │   │   │   │   │   │   └── ed25519-dalek v3.0.0-pre.1 (*)
│   │   │   │   │   │   ├── curve25519-dalek feature "alloc" (*)
│   │   │   │   │   │   ├── zeroize feature "alloc" (*)
│   │   │   │   │   │   ├── ed25519 feature "alloc"
│   │   │   │   │   │   │   ├── ed25519 v3.0.0-rc.4 (*)
│   │   │   │   │   │   │   └── pkcs8 feature "alloc"
│   │   │   │   │   │   │       ├── pkcs8 v0.11.0-rc.11 (*)
│   │   │   │   │   │   │       ├── der feature "alloc"
│   │   │   │   │   │   │       │   ├── der v0.8.0 (*)
│   │   │   │   │   │   │       │   └── zeroize feature "alloc" (*)
│   │   │   │   │   │   │       ├── der feature "zeroize"
│   │   │   │   │   │   │       │   └── der v0.8.0 (*)
│   │   │   │   │   │   │       └── spki feature "alloc"
│   │   │   │   │   │   │           ├── spki v0.8.0-rc.4 (*)
│   │   │   │   │   │   │           └── der feature "alloc" (*)
│   │   │   │   │   │   └── signature feature "alloc"
│   │   │   │   │   │       └── signature v3.0.0-rc.10
│   │   │   │   │   ├── ed25519-dalek feature "default" (*)
│   │   │   │   │   ├── base32 feature "default"
│   │   │   │   │   │   └── base32 v0.5.1
│   │   │   │   │   ├── document-features feature "default"
│   │   │   │   │   │   └── document-features v0.2.12 (proc-macro)
│   │   │   │   │   │       └── litrs feature "default"
│   │   │   │   │   │           └── litrs v1.0.0
│   │   │   │   │   ├── ntimestamp feature "default"
│   │   │   │   │   │   └── ntimestamp v1.0.0
│   │   │   │   │   │       ├── getrandom v0.2.17 (*)
│   │   │   │   │   │       ├── serde feature "derive" (*)
│   │   │   │   │   │       ├── once_cell feature "default" (*)
│   │   │   │   │   │       ├── httpdate feature "default" (*)
│   │   │   │   │   │       ├── base32 feature "default" (*)
│   │   │   │   │   │       └── document-features feature "default" (*)
│   │   │   │   │   ├── ntimestamp feature "full"
│   │   │   │   │   │   ├── ntimestamp v1.0.0 (*)
│   │   │   │   │   │   ├── ntimestamp feature "base32"
│   │   │   │   │   │   │   └── ntimestamp v1.0.0 (*)
│   │   │   │   │   │   ├── ntimestamp feature "httpdate"
│   │   │   │   │   │   │   └── ntimestamp v1.0.0 (*)
│   │   │   │   │   │   └── ntimestamp feature "serde"
│   │   │   │   │   │       └── ntimestamp v1.0.0 (*)
│   │   │   │   │   ├── self_cell feature "default"
│   │   │   │   │   │   └── self_cell v1.2.2
│   │   │   │   │   └── simple-dns feature "default"
│   │   │   │   │       └── simple-dns v0.9.3
│   │   │   │   │           └── bitflags feature "default"
│   │   │   │   │               └── bitflags v2.11.0
│   │   │   │   │   [build-dependencies]
│   │   │   │   │   └── cfg_aliases feature "default" (*)
│   │   │   │   └── pkarr feature "keys"
│   │   │   │       └── pkarr v5.0.2 (*)
│   │   │   ├── reqwest feature "rustls-tls"
│   │   │   │   ├── reqwest v0.12.28
│   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   ├── serde feature "default" (*)
│   │   │   │   │   ├── tokio feature "net" (*)
│   │   │   │   │   ├── tokio feature "time" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   ├── tokio-util feature "io" (*)
│   │   │   │   │   ├── log feature "default" (*)
│   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   ├── rustls feature "tls12" (*)
│   │   │   │   │   ├── rustls-pki-types feature "default" (*)
│   │   │   │   │   ├── rustls-pki-types feature "std" (*)
│   │   │   │   │   ├── tokio-rustls feature "tls12"
│   │   │   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   │   │   └── rustls feature "tls12" (*)
│   │   │   │   │   ├── url feature "default" (*)
│   │   │   │   │   ├── percent-encoding feature "default" (*)
│   │   │   │   │   ├── http-body-util feature "default" (*)
│   │   │   │   │   ├── http-body feature "default" (*)
│   │   │   │   │   ├── hyper feature "client" (*)
│   │   │   │   │   ├── hyper feature "default" (*)
│   │   │   │   │   ├── hyper feature "http1" (*)
│   │   │   │   │   ├── hyper-util feature "client"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   ├── tokio feature "net" (*)
│   │   │   │   │   │   ├── hyper feature "client" (*)
│   │   │   │   │   │   └── hyper-util feature "tokio"
│   │   │   │   │   │       ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │       ├── tokio feature "rt" (*)
│   │   │   │   │   │       ├── tokio feature "time" (*)
│   │   │   │   │   │       └── hyper-util feature "tokio" (*)
│   │   │   │   │   ├── hyper-util feature "client-legacy"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   ├── tokio feature "sync" (*)
│   │   │   │   │   │   ├── hyper-util feature "client" (*)
│   │   │   │   │   │   └── hyper-util feature "tokio" (*)
│   │   │   │   │   ├── hyper-util feature "client-proxy"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   └── hyper-util feature "client" (*)
│   │   │   │   │   ├── hyper-util feature "default" (*)
│   │   │   │   │   ├── hyper-util feature "http1"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   └── hyper feature "http1" (*)
│   │   │   │   │   ├── hyper-util feature "tokio" (*)
│   │   │   │   │   ├── base64 feature "default" (*)
│   │   │   │   │   ├── tower-service feature "default" (*)
│   │   │   │   │   ├── hyper-rustls feature "http1"
│   │   │   │   │   │   ├── hyper-rustls v0.27.7
│   │   │   │   │   │   │   ├── hyper v1.8.1 (*)
│   │   │   │   │   │   │   ├── rustls v0.23.37 (*)
│   │   │   │   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   │   │   ├── rustls-pki-types feature "default" (*)
│   │   │   │   │   │   │   ├── hyper-util feature "client-legacy" (*)
│   │   │   │   │   │   │   ├── hyper-util feature "tokio" (*)
│   │   │   │   │   │   │   ├── tower-service feature "default" (*)
│   │   │   │   │   │   │   └── webpki-roots feature "default"
│   │   │   │   │   │   │       └── webpki-roots v1.0.6
│   │   │   │   │   │   │           └── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   │   └── hyper-util feature "http1" (*)
│   │   │   │   │   ├── hyper-rustls feature "tls12"
│   │   │   │   │   │   ├── hyper-rustls v0.27.7 (*)
│   │   │   │   │   │   ├── rustls feature "tls12" (*)
│   │   │   │   │   │   └── tokio-rustls feature "tls12" (*)
│   │   │   │   │   ├── webpki-roots feature "default" (*)
│   │   │   │   │   ├── serde_urlencoded feature "default"
│   │   │   │   │   │   └── serde_urlencoded v0.7.1
│   │   │   │   │   │       ├── serde feature "default" (*)
│   │   │   │   │   │       ├── itoa feature "default" (*)
│   │   │   │   │   │       ├── form_urlencoded feature "default"
│   │   │   │   │   │       │   ├── form_urlencoded v1.2.2 (*)
│   │   │   │   │   │       │   └── form_urlencoded feature "std" (*)
│   │   │   │   │   │       └── ryu feature "default" (*)
│   │   │   │   │   ├── sync_wrapper feature "default"
│   │   │   │   │   │   └── sync_wrapper v1.0.2
│   │   │   │   │   │       └── futures-core v0.3.32
│   │   │   │   │   ├── sync_wrapper feature "futures"
│   │   │   │   │   │   ├── sync_wrapper v1.0.2 (*)
│   │   │   │   │   │   └── sync_wrapper feature "futures-core"
│   │   │   │   │   │       └── sync_wrapper v1.0.2 (*)
│   │   │   │   │   ├── tower feature "retry"
│   │   │   │   │   │   ├── tower v0.5.3
│   │   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   │   │   ├── futures-util feature "alloc" (*)
│   │   │   │   │   │   │   ├── futures-core feature "default" (*)
│   │   │   │   │   │   │   ├── tower-service feature "default" (*)
│   │   │   │   │   │   │   ├── sync_wrapper feature "default" (*)
│   │   │   │   │   │   │   └── tower-layer feature "default"
│   │   │   │   │   │   │       └── tower-layer v0.3.3
│   │   │   │   │   │   ├── tokio feature "time" (*)
│   │   │   │   │   │   ├── tower feature "tokio"
│   │   │   │   │   │   │   └── tower v0.5.3 (*)
│   │   │   │   │   │   └── tower feature "util"
│   │   │   │   │   │       ├── tower v0.5.3 (*)
│   │   │   │   │   │       ├── tower feature "futures-core"
│   │   │   │   │   │       │   └── tower v0.5.3 (*)
│   │   │   │   │   │       ├── tower feature "futures-util"
│   │   │   │   │   │       │   └── tower v0.5.3 (*)
│   │   │   │   │   │       ├── tower feature "pin-project-lite"
│   │   │   │   │   │       │   └── tower v0.5.3 (*)
│   │   │   │   │   │       └── tower feature "sync_wrapper"
│   │   │   │   │   │           └── tower v0.5.3 (*)
│   │   │   │   │   ├── tower feature "timeout"
│   │   │   │   │   │   ├── tower v0.5.3 (*)
│   │   │   │   │   │   ├── tokio feature "time" (*)
│   │   │   │   │   │   ├── tower feature "pin-project-lite" (*)
│   │   │   │   │   │   └── tower feature "tokio" (*)
│   │   │   │   │   ├── tower feature "util" (*)
│   │   │   │   │   └── tower-http feature "follow-redirect"
│   │   │   │   │       ├── tower-http v0.6.8
│   │   │   │   │       │   ├── futures-util v0.3.32 (*)
│   │   │   │   │       │   ├── bytes feature "default" (*)
│   │   │   │   │       │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │       │   ├── http feature "default" (*)
│   │   │   │   │       │   ├── http-body feature "default" (*)
│   │   │   │   │       │   ├── tower-service feature "default" (*)
│   │   │   │   │       │   ├── bitflags feature "default" (*)
│   │   │   │   │       │   ├── tower feature "default"
│   │   │   │   │       │   │   └── tower v0.5.3 (*)
│   │   │   │   │       │   ├── tower-layer feature "default" (*)
│   │   │   │   │       │   └── iri-string feature "default"
│   │   │   │   │       │       ├── iri-string v0.7.10
│   │   │   │   │       │       └── iri-string feature "std"
│   │   │   │   │       │           ├── iri-string v0.7.10
│   │   │   │   │       │           └── iri-string feature "alloc"
│   │   │   │   │       │               └── iri-string v0.7.10
│   │   │   │   │       ├── tower feature "util" (*)
│   │   │   │   │       ├── tower-http feature "futures-util"
│   │   │   │   │       │   └── tower-http v0.6.8 (*)
│   │   │   │   │       ├── tower-http feature "iri-string"
│   │   │   │   │       │   └── tower-http v0.6.8 (*)
│   │   │   │   │       └── tower-http feature "tower"
│   │   │   │   │           └── tower-http v0.6.8 (*)
│   │   │   │   └── reqwest feature "rustls-tls-webpki-roots"
│   │   │   │       ├── reqwest v0.12.28 (*)
│   │   │   │       ├── reqwest feature "__rustls-ring"
│   │   │   │       │   ├── reqwest v0.12.28 (*)
│   │   │   │       │   ├── rustls feature "ring" (*)
│   │   │   │       │   ├── tokio-rustls feature "ring" (*)
│   │   │   │       │   └── hyper-rustls feature "ring"
│   │   │   │       │       ├── hyper-rustls v0.27.7 (*)
│   │   │   │       │       └── rustls feature "ring" (*)
│   │   │   │       └── reqwest feature "rustls-tls-webpki-roots-no-provider"
│   │   │   │           ├── reqwest v0.12.28 (*)
│   │   │   │           ├── reqwest feature "__rustls"
│   │   │   │           │   ├── reqwest v0.12.28 (*)
│   │   │   │           │   └── reqwest feature "__tls"
│   │   │   │           │       ├── reqwest v0.12.28 (*)
│   │   │   │           │       └── tokio feature "io-util" (*)
│   │   │   │           └── hyper-rustls feature "webpki-tokio"
│   │   │   │               ├── hyper-rustls v0.27.7 (*)
│   │   │   │               └── hyper-rustls feature "webpki-roots"
│   │   │   │                   └── hyper-rustls v0.27.7 (*)
│   │   │   ├── webpki-roots feature "default" (*)
│   │   │   ├── serde_bytes feature "default"
│   │   │   │   ├── serde_bytes v0.11.19
│   │   │   │   │   └── serde_core v1.0.228
│   │   │   │   └── serde_bytes feature "std"
│   │   │   │       ├── serde_bytes v0.11.19 (*)
│   │   │   │       └── serde_core feature "std" (*)
│   │   │   ├── strum feature "default"
│   │   │   │   ├── strum v0.28.0
│   │   │   │   │   └── strum_macros feature "default"
│   │   │   │   │       └── strum_macros v0.28.0 (proc-macro)
│   │   │   │   │           ├── proc-macro2 feature "default" (*)
│   │   │   │   │           ├── quote feature "default" (*)
│   │   │   │   │           ├── syn feature "default" (*)
│   │   │   │   │           ├── syn feature "parsing" (*)
│   │   │   │   │           └── heck feature "default" (*)
│   │   │   │   └── strum feature "std"
│   │   │   │       └── strum v0.28.0 (*)
│   │   │   ├── strum feature "derive"
│   │   │   │   ├── strum v0.28.0 (*)
│   │   │   │   └── strum feature "strum_macros"
│   │   │   │       └── strum v0.28.0 (*)
│   │   │   ├── tokio-websockets feature "client"
│   │   │   │   ├── tokio-websockets v0.12.3
│   │   │   │   │   ├── getrandom v0.3.4 (*)
│   │   │   │   │   ├── ring v0.17.14 (*)
│   │   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── futures-core feature "default" (*)
│   │   │   │   │   ├── futures-sink feature "default" (*)
│   │   │   │   │   ├── http feature "std" (*)
│   │   │   │   │   ├── tokio-util feature "codec" (*)
│   │   │   │   │   ├── tokio-util feature "default" (*)
│   │   │   │   │   ├── tokio-util feature "io" (*)
│   │   │   │   │   ├── rand feature "thread_rng" (*)
│   │   │   │   │   ├── rustls-pki-types feature "default" (*)
│   │   │   │   │   ├── httparse feature "default" (*)
│   │   │   │   │   ├── base64 feature "default" (*)
│   │   │   │   │   ├── simdutf8 feature "aarch64_neon"
│   │   │   │   │   │   └── simdutf8 v0.1.5
│   │   │   │   │   └── simdutf8 feature "std"
│   │   │   │   │       └── simdutf8 v0.1.5
│   │   │   │   ├── tokio feature "io-util" (*)
│   │   │   │   └── tokio feature "net" (*)
│   │   │   ├── tokio-websockets feature "default"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   ├── tokio-websockets feature "getrandom"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   ├── tokio-websockets feature "rand"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   ├── tokio-websockets feature "ring"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   ├── tokio-websockets feature "rustls-bring-your-own-connector"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   └── z32 feature "default"
│   │   │       └── z32 v1.3.0
│   │   │   [build-dependencies]
│   │   │   ├── cfg_aliases feature "default" (*)
│   │   │   └── vergen-gitcl feature "default"
│   │   │       └── vergen-gitcl v1.0.8
│   │   │           ├── vergen v9.1.0
│   │   │           │   ├── anyhow feature "default"
│   │   │           │   │   ├── anyhow v1.0.102
│   │   │           │   │   └── anyhow feature "std"
│   │   │           │   │       └── anyhow v1.0.102
│   │   │           │   ├── derive_builder feature "default"
│   │   │           │   │   ├── derive_builder v0.20.2
│   │   │           │   │   │   └── derive_builder_macro feature "default"
│   │   │           │   │   │       └── derive_builder_macro v0.20.2 (proc-macro)
│   │   │           │   │   │           ├── syn feature "default" (*)
│   │   │           │   │   │           ├── syn feature "extra-traits" (*)
│   │   │           │   │   │           ├── syn feature "full" (*)
│   │   │           │   │   │           └── derive_builder_core feature "default"
│   │   │           │   │   │               └── derive_builder_core v0.20.2
│   │   │           │   │   │                   ├── proc-macro2 feature "default" (*)
│   │   │           │   │   │                   ├── quote feature "default" (*)
│   │   │           │   │   │                   ├── syn feature "default" (*)
│   │   │           │   │   │                   ├── syn feature "extra-traits" (*)
│   │   │           │   │   │                   ├── syn feature "full" (*)
│   │   │           │   │   │                   └── darling feature "default"
│   │   │           │   │   │                       ├── darling v0.20.11
│   │   │           │   │   │                       │   ├── darling_core feature "default"
│   │   │           │   │   │                       │   │   └── darling_core v0.20.11
│   │   │           │   │   │                       │   │       ├── proc-macro2 feature "default" (*)
│   │   │           │   │   │                       │   │       ├── quote feature "default" (*)
│   │   │           │   │   │                       │   │       ├── syn feature "default" (*)
│   │   │           │   │   │                       │   │       ├── syn feature "extra-traits" (*)
│   │   │           │   │   │                       │   │       ├── syn feature "full" (*)
│   │   │           │   │   │                       │   │       ├── fnv feature "default" (*)
│   │   │           │   │   │                       │   │       ├── ident_case feature "default"
│   │   │           │   │   │                       │   │       │   └── ident_case v1.0.1
│   │   │           │   │   │                       │   │       └── strsim feature "default"
│   │   │           │   │   │                       │   │           └── strsim v0.11.1
│   │   │           │   │   │                       │   └── darling_macro feature "default"
│   │   │           │   │   │                       │       └── darling_macro v0.20.11 (proc-macro)
│   │   │           │   │   │                       │           ├── quote feature "default" (*)
│   │   │           │   │   │                       │           ├── syn feature "default" (*)
│   │   │           │   │   │                       │           └── darling_core feature "default" (*)
│   │   │           │   │   │                       └── darling feature "suggestions"
│   │   │           │   │   │                           ├── darling v0.20.11 (*)
│   │   │           │   │   │                           └── darling_core feature "suggestions"
│   │   │           │   │   │                               ├── darling_core v0.20.11 (*)
│   │   │           │   │   │                               └── darling_core feature "strsim"
│   │   │           │   │   │                                   └── darling_core v0.20.11 (*)
│   │   │           │   │   └── derive_builder feature "std"
│   │   │           │   │       ├── derive_builder v0.20.2 (*)
│   │   │           │   │       └── derive_builder_macro feature "lib_has_std"
│   │   │           │   │           ├── derive_builder_macro v0.20.2 (proc-macro) (*)
│   │   │           │   │           └── derive_builder_core feature "lib_has_std"
│   │   │           │   │               └── derive_builder_core v0.20.2 (*)
│   │   │           │   └── vergen-lib feature "default"
│   │   │           │       └── vergen-lib v9.1.0
│   │   │           │           ├── anyhow feature "default" (*)
│   │   │           │           └── derive_builder feature "default" (*)
│   │   │           │           [build-dependencies]
│   │   │           │           └── rustversion feature "default" (*)
│   │   │           │   [build-dependencies]
│   │   │           │   └── rustversion feature "default" (*)
│   │   │           ├── anyhow feature "default" (*)
│   │   │           ├── derive_builder feature "default" (*)
│   │   │           ├── time feature "default"
│   │   │           │   ├── time v0.3.47
│   │   │           │   │   ├── powerfmt v0.2.0
│   │   │           │   │   ├── libc feature "default" (*)
│   │   │           │   │   ├── itoa feature "default" (*)
│   │   │           │   │   ├── deranged feature "default"
│   │   │           │   │   │   └── deranged v0.5.8
│   │   │           │   │   │       └── powerfmt v0.2.0
│   │   │           │   │   ├── deranged feature "powerfmt"
│   │   │           │   │   │   └── deranged v0.5.8 (*)
│   │   │           │   │   ├── num-conv feature "default"
│   │   │           │   │   │   └── num-conv v0.2.0
│   │   │           │   │   ├── num_threads feature "default"
│   │   │           │   │   │   └── num_threads v0.1.7
│   │   │           │   │   └── time-core feature "default"
│   │   │           │   │       └── time-core v0.1.8
│   │   │           │   └── time feature "std"
│   │   │           │       ├── time v0.3.47 (*)
│   │   │           │       └── time feature "alloc"
│   │   │           │           └── time v0.3.47 (*)
│   │   │           ├── time feature "formatting"
│   │   │           │   ├── time v0.3.47 (*)
│   │   │           │   └── time feature "std" (*)
│   │   │           ├── time feature "local-offset"
│   │   │           │   ├── time v0.3.47 (*)
│   │   │           │   └── time feature "std" (*)
│   │   │           ├── time feature "parsing"
│   │   │           │   └── time v0.3.47 (*)
│   │   │           ├── vergen-lib feature "default"
│   │   │           │   └── vergen-lib v0.1.6
│   │   │           │       ├── anyhow feature "default" (*)
│   │   │           │       └── derive_builder feature "default" (*)
│   │   │           │       [build-dependencies]
│   │   │           │       └── rustversion feature "default" (*)
│   │   │           └── vergen-lib feature "git"
│   │   │               └── vergen-lib v0.1.6 (*)
│   │   │           [build-dependencies]
│   │   │           └── rustversion feature "default" (*)
│   │   ├── papaya v0.2.3
│   │   │   ├── equivalent feature "default" (*)
│   │   │   └── seize feature "default"
│   │   │       ├── seize v0.5.1
│   │   │       │   └── libc feature "default" (*)
│   │   │       └── seize feature "fast-barrier"
│   │   │           ├── seize v0.5.1 (*)
│   │   │           ├── seize feature "libc"
│   │   │           │   └── seize v0.5.1 (*)
│   │   │           └── seize feature "windows-sys"
│   │   │               └── seize v0.5.1 (*)
│   │   ├── pkarr v5.0.2 (*)
│   │   ├── portmapper v0.15.0
│   │   │   ├── iroh-metrics v0.38.3 (*)
│   │   │   ├── serde feature "default" (*)
│   │   │   ├── serde feature "derive" (*)
│   │   │   ├── serde feature "rc" (*)
│   │   │   ├── tokio feature "default" (*)
│   │   │   ├── tokio feature "fs" (*)
│   │   │   ├── tokio feature "io-std" (*)
│   │   │   ├── tokio feature "io-util" (*)
│   │   │   ├── tokio feature "macros" (*)
│   │   │   ├── tokio feature "net" (*)
│   │   │   ├── tokio feature "rt" (*)
│   │   │   ├── tokio feature "sync" (*)
│   │   │   ├── bytes feature "default" (*)
│   │   │   ├── libc feature "default" (*)
│   │   │   ├── socket2 feature "default" (*)
│   │   │   ├── derive_more feature "debug" (*)
│   │   │   ├── derive_more feature "default" (*)
│   │   │   ├── derive_more feature "deref" (*)
│   │   │   ├── derive_more feature "display" (*)
│   │   │   ├── derive_more feature "from" (*)
│   │   │   ├── derive_more feature "try_into" (*)
│   │   │   ├── futures-util feature "default" (*)
│   │   │   ├── tokio-util feature "codec" (*)
│   │   │   ├── tokio-util feature "default" (*)
│   │   │   ├── tokio-util feature "io" (*)
│   │   │   ├── tokio-util feature "io-util" (*)
│   │   │   ├── tokio-util feature "rt" (*)
│   │   │   ├── tracing feature "default" (*)
│   │   │   ├── smallvec feature "default" (*)
│   │   │   ├── rand feature "default" (*)
│   │   │   ├── url feature "default" (*)
│   │   │   ├── url feature "serde" (*)
│   │   │   ├── n0-error feature "default" (*)
│   │   │   ├── hyper-util feature "default" (*)
│   │   │   ├── base64 feature "default" (*)
│   │   │   ├── futures-lite feature "default" (*)
│   │   │   ├── num_enum feature "default" (*)
│   │   │   ├── tower-layer feature "default" (*)
│   │   │   ├── netwatch feature "default"
│   │   │   │   └── netwatch v0.15.0
│   │   │   │       ├── tokio feature "default" (*)
│   │   │   │       ├── tokio feature "fs" (*)
│   │   │   │       ├── tokio feature "io-std" (*)
│   │   │   │       ├── tokio feature "io-util" (*)
│   │   │   │       ├── tokio feature "macros" (*)
│   │   │   │       ├── tokio feature "net" (*)
│   │   │   │       ├── tokio feature "rt" (*)
│   │   │   │       ├── tokio feature "sync" (*)
│   │   │   │       ├── tokio feature "time" (*)
│   │   │   │       ├── bytes feature "default" (*)
│   │   │   │       ├── libc feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── socket2 feature "all" (*)
│   │   │   │       ├── socket2 feature "default" (*)
│   │   │   │       ├── atomic-waker feature "default" (*)
│   │   │   │       ├── tokio-util feature "default" (*)
│   │   │   │       ├── tokio-util feature "rt" (*)
│   │   │   │       ├── tracing feature "default" (*)
│   │   │   │       ├── n0-error feature "default" (*)
│   │   │   │       ├── n0-future feature "default" (*)
│   │   │   │       ├── noq-udp feature "default"
│   │   │   │       │   ├── noq-udp v0.9.0 (*)
│   │   │   │       │   ├── noq-udp feature "tracing" (*)
│   │   │   │       │   └── noq-udp feature "tracing-log"
│   │   │   │       │       ├── noq-udp v0.9.0 (*)
│   │   │   │       │       ├── tracing feature "log" (*)
│   │   │   │       │       └── noq-udp feature "tracing" (*)
│   │   │   │       ├── n0-watcher feature "default"
│   │   │   │       │   └── n0-watcher v0.6.1
│   │   │   │       │       ├── derive_more feature "debug" (*)
│   │   │   │       │       ├── derive_more feature "default" (*)
│   │   │   │       │       ├── n0-error feature "default" (*)
│   │   │   │       │       └── n0-future feature "default" (*)
│   │   │   │       ├── netdev feature "default"
│   │   │   │       │   ├── netdev v0.40.1
│   │   │   │       │   │   ├── libc feature "default" (*)
│   │   │   │       │   │   ├── ipnet feature "default" (*)
│   │   │   │       │   │   ├── mac-addr feature "default"
│   │   │   │       │   │   │   ├── mac-addr v0.3.0
│   │   │   │       │   │   │   └── mac-addr feature "std"
│   │   │   │       │   │   │       └── mac-addr v0.3.0
│   │   │   │       │   │   ├── netlink-packet-core feature "default"
│   │   │   │       │   │   │   └── netlink-packet-core v0.8.1
│   │   │   │       │   │   │       └── paste feature "default"
│   │   │   │       │   │   │           └── paste v1.0.15 (proc-macro)
│   │   │   │       │   │   ├── netlink-packet-route feature "default"
│   │   │   │       │   │   │   └── netlink-packet-route v0.29.0
│   │   │   │       │   │   │       ├── libc feature "default" (*)
│   │   │   │       │   │   │       ├── log feature "default" (*)
│   │   │   │       │   │   │       ├── log feature "std"
│   │   │   │       │   │   │       │   └── log v0.4.29
│   │   │   │       │   │   │       ├── bitflags feature "default" (*)
│   │   │   │       │   │   │       └── netlink-packet-core feature "default" (*)
│   │   │   │       │   │   └── netlink-sys feature "default"
│   │   │   │       │   │       └── netlink-sys v0.8.8
│   │   │   │       │   │           ├── tokio feature "net" (*)
│   │   │   │       │   │           ├── bytes feature "default" (*)
│   │   │   │       │   │           ├── libc feature "default" (*)
│   │   │   │       │   │           ├── futures-util feature "default" (*)
│   │   │   │       │   │           └── log feature "default" (*)
│   │   │   │       │   └── netdev feature "gateway"
│   │   │   │       │       └── netdev v0.40.1 (*)
│   │   │   │       ├── netlink-packet-core feature "default" (*)
│   │   │   │       ├── netlink-packet-route feature "default" (*)
│   │   │   │       ├── netlink-sys feature "default" (*)
│   │   │   │       ├── netlink-proto feature "default"
│   │   │   │       │   ├── netlink-proto v0.12.0
│   │   │   │       │   │   ├── netlink-sys v0.8.8 (*)
│   │   │   │       │   │   ├── thiserror feature "default" (*)
│   │   │   │       │   │   ├── bytes feature "default" (*)
│   │   │   │       │   │   ├── log feature "default" (*)
│   │   │   │       │   │   ├── netlink-packet-core feature "default" (*)
│   │   │   │       │   │   └── futures feature "default"
│   │   │   │       │   │       ├── futures v0.3.32
│   │   │   │       │   │       │   ├── futures-core v0.3.32
│   │   │   │       │   │       │   ├── futures-executor v0.3.32
│   │   │   │       │   │       │   │   ├── futures-core v0.3.32
│   │   │   │       │   │       │   │   ├── futures-task v0.3.32
│   │   │   │       │   │       │   │   └── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   ├── futures-io v0.3.32
│   │   │   │       │   │       │   ├── futures-sink v0.3.32
│   │   │   │       │   │       │   ├── futures-task v0.3.32
│   │   │   │       │   │       │   ├── futures-util feature "sink" (*)
│   │   │   │       │   │       │   └── futures-channel feature "sink"
│   │   │   │       │   │       │       ├── futures-channel v0.3.32 (*)
│   │   │   │       │   │       │       └── futures-channel feature "futures-sink"
│   │   │   │       │   │       │           └── futures-channel v0.3.32 (*)
│   │   │   │       │   │       ├── futures feature "async-await"
│   │   │   │       │   │       │   ├── futures v0.3.32 (*)
│   │   │   │       │   │       │   ├── futures-util feature "async-await" (*)
│   │   │   │       │   │       │   └── futures-util feature "async-await-macro" (*)
│   │   │   │       │   │       ├── futures feature "executor"
│   │   │   │       │   │       │   ├── futures v0.3.32 (*)
│   │   │   │       │   │       │   ├── futures feature "futures-executor"
│   │   │   │       │   │       │   │   └── futures v0.3.32 (*)
│   │   │   │       │   │       │   ├── futures feature "std"
│   │   │   │       │   │       │   │   ├── futures v0.3.32 (*)
│   │   │   │       │   │       │   │   ├── futures-util feature "channel"
│   │   │   │       │   │       │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   ├── futures-util feature "futures-channel"
│   │   │   │       │   │       │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   └── futures-util feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-util feature "io"
│   │   │   │       │   │       │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   ├── futures-util feature "futures-io"
│   │   │   │       │   │       │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   ├── futures-util feature "memchr"
│   │   │   │       │   │       │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   └── futures-util feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-util feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-core feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-sink feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-io feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-task feature "std" (*)
│   │   │   │       │   │       │   │   └── futures feature "alloc"
│   │   │   │       │   │       │   │       ├── futures v0.3.32 (*)
│   │   │   │       │   │       │   │       ├── futures-util feature "alloc" (*)
│   │   │   │       │   │       │   │       ├── futures-channel feature "alloc" (*)
│   │   │   │       │   │       │   │       ├── futures-core feature "alloc" (*)
│   │   │   │       │   │       │   │       ├── futures-sink feature "alloc" (*)
│   │   │   │       │   │       │   │       └── futures-task feature "alloc" (*)
│   │   │   │       │   │       │   └── futures-executor feature "std"
│   │   │   │       │   │       │       ├── futures-executor v0.3.32 (*)
│   │   │   │       │   │       │       ├── futures-util feature "std" (*)
│   │   │   │       │   │       │       ├── futures-core feature "std" (*)
│   │   │   │       │   │       │       └── futures-task feature "std" (*)
│   │   │   │       │   │       └── futures feature "std" (*)
│   │   │   │       │   └── netlink-proto feature "tokio_socket"
│   │   │   │       │       ├── netlink-proto v0.12.0 (*)
│   │   │   │       │       └── netlink-sys feature "tokio_socket"
│   │   │   │       │           ├── netlink-sys v0.8.8 (*)
│   │   │   │       │           ├── netlink-sys feature "futures-util"
│   │   │   │       │           │   └── netlink-sys v0.8.8 (*)
│   │   │   │       │           └── netlink-sys feature "tokio"
│   │   │   │       │               └── netlink-sys v0.8.8 (*)
│   │   │   │       └── time feature "default"
│   │   │   │           ├── time v0.3.47
│   │   │   │           │   ├── powerfmt v0.2.0
│   │   │   │           │   ├── deranged feature "default" (*)
│   │   │   │           │   ├── deranged feature "powerfmt" (*)
│   │   │   │           │   ├── num-conv feature "default" (*)
│   │   │   │           │   └── time-core feature "default" (*)
│   │   │   │           └── time feature "std"
│   │   │   │               ├── time v0.3.47 (*)
│   │   │   │               └── time feature "alloc"
│   │   │   │                   └── time v0.3.47 (*)
│   │   │   │       [build-dependencies]
│   │   │   │       └── cfg_aliases feature "default" (*)
│   │   │   ├── time feature "default" (*)
│   │   │   ├── igd-next feature "aio_tokio"
│   │   │   │   ├── igd-next v0.16.2
│   │   │   │   │   ├── attohttpc v0.30.1
│   │   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   │   ├── log feature "default" (*)
│   │   │   │   │   │   ├── url feature "default" (*)
│   │   │   │   │   │   └── base64 feature "default" (*)
│   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   ├── tokio feature "net" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── async-trait feature "default" (*)
│   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   ├── log feature "default" (*)
│   │   │   │   │   ├── rand feature "default" (*)
│   │   │   │   │   ├── url feature "default" (*)
│   │   │   │   │   ├── http-body-util feature "default" (*)
│   │   │   │   │   ├── hyper feature "client" (*)
│   │   │   │   │   ├── hyper feature "http1" (*)
│   │   │   │   │   ├── hyper feature "http2"
│   │   │   │   │   │   └── hyper v1.8.1 (*)
│   │   │   │   │   ├── hyper-util feature "client" (*)
│   │   │   │   │   ├── hyper-util feature "client-legacy" (*)
│   │   │   │   │   ├── hyper-util feature "http1" (*)
│   │   │   │   │   ├── hyper-util feature "http2"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   └── hyper feature "http2" (*)
│   │   │   │   │   ├── futures feature "default" (*)
│   │   │   │   │   └── xmltree feature "default"
│   │   │   │   │       └── xmltree v0.10.3
│   │   │   │   │           └── xml-rs feature "default"
│   │   │   │   │               └── xml-rs v0.8.28
│   │   │   │   ├── igd-next feature "async-trait"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "bytes"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "futures"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "http"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "http-body-util"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "hyper"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "hyper-util"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   └── igd-next feature "tokio"
│   │   │   │       └── igd-next v0.16.2 (*)
│   │   │   └── igd-next feature "default"
│   │   │       └── igd-next v0.16.2 (*)
│   │   ├── serde feature "default" (*)
│   │   ├── serde feature "derive" (*)
│   │   ├── serde feature "rc" (*)
│   │   ├── backon feature "default"
│   │   │   ├── backon v1.6.0
│   │   │   │   ├── fastrand v2.3.0
│   │   │   │   └── tokio feature "default" (*)
│   │   │   ├── backon feature "gloo-timers-sleep"
│   │   │   │   └── backon v1.6.0 (*)
│   │   │   ├── backon feature "std"
│   │   │   │   ├── backon v1.6.0 (*)
│   │   │   │   └── fastrand feature "std" (*)
│   │   │   ├── backon feature "std-blocking-sleep"
│   │   │   │   └── backon v1.6.0 (*)
│   │   │   └── backon feature "tokio-sleep"
│   │   │       ├── backon v1.6.0 (*)
│   │   │       ├── backon feature "tokio"
│   │   │       │   └── backon v1.6.0 (*)
│   │   │       └── tokio feature "time" (*)
│   │   ├── tokio feature "default" (*)
│   │   ├── tokio feature "fs" (*)
│   │   ├── tokio feature "io-std" (*)
│   │   ├── tokio feature "io-util" (*)
│   │   ├── tokio feature "macros" (*)
│   │   ├── tokio feature "net" (*)
│   │   ├── tokio feature "rt" (*)
│   │   ├── tokio feature "sync" (*)
│   │   ├── bytes feature "default" (*)
│   │   ├── data-encoding feature "default" (*)
│   │   ├── derive_more feature "debug" (*)
│   │   ├── derive_more feature "default" (*)
│   │   ├── derive_more feature "deref" (*)
│   │   ├── derive_more feature "display" (*)
│   │   ├── derive_more feature "from" (*)
│   │   ├── derive_more feature "from_str"
│   │   │   ├── derive_more v2.1.1 (*)
│   │   │   └── derive_more-impl feature "from_str"
│   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │       ├── syn feature "full" (*)
│   │   │       └── syn feature "visit" (*)
│   │   ├── derive_more feature "into_iterator"
│   │   │   ├── derive_more v2.1.1 (*)
│   │   │   └── derive_more-impl feature "into_iterator"
│   │   │       └── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   ├── derive_more feature "try_into" (*)
│   │   ├── ed25519-dalek feature "default" (*)
│   │   ├── ed25519-dalek feature "pem"
│   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│   │   │   ├── ed25519-dalek feature "alloc" (*)
│   │   │   ├── ed25519-dalek feature "pkcs8"
│   │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│   │   │   │   └── ed25519 feature "pkcs8"
│   │   │   │       └── ed25519 v3.0.0-rc.4 (*)
│   │   │   └── ed25519 feature "pem"
│   │   │       ├── ed25519 v3.0.0-rc.4 (*)
│   │   │       ├── ed25519 feature "alloc" (*)
│   │   │       ├── ed25519 feature "pkcs8" (*)
│   │   │       └── pkcs8 feature "pem"
│   │   │           ├── pkcs8 v0.11.0-rc.11 (*)
│   │   │           ├── pkcs8 feature "alloc" (*)
│   │   │           ├── der feature "pem"
│   │   │           │   ├── der v0.8.0 (*)
│   │   │           │   ├── der feature "alloc" (*)
│   │   │           │   └── der feature "zeroize" (*)
│   │   │           └── spki feature "pem"
│   │   │               ├── spki v0.8.0-rc.4 (*)
│   │   │               ├── der feature "pem" (*)
│   │   │               └── spki feature "alloc" (*)
│   │   ├── ed25519-dalek feature "pkcs8" (*)
│   │   ├── ed25519-dalek feature "rand_core" (*)
│   │   ├── ed25519-dalek feature "serde" (*)
│   │   ├── ed25519-dalek feature "zeroize" (*)
│   │   ├── pkcs8 feature "default" (*)
│   │   ├── futures-util feature "default" (*)
│   │   ├── hickory-resolver feature "default" (*)
│   │   ├── http feature "default" (*)
│   │   ├── tokio-util feature "default" (*)
│   │   ├── tokio-util feature "io" (*)
│   │   ├── tokio-util feature "io-util" (*)
│   │   ├── tokio-util feature "rt" (*)
│   │   ├── tracing feature "default" (*)
│   │   ├── portable-atomic feature "default" (*)
│   │   ├── smallvec feature "default" (*)
│   │   ├── ipnet feature "default" (*)
│   │   ├── rand feature "default" (*)
│   │   ├── rustls feature "ring" (*)
│   │   ├── rustls-pki-types feature "default" (*)
│   │   ├── rustls-webpki feature "default"
│   │   │   ├── rustls-webpki v0.103.9 (*)
│   │   │   └── rustls-webpki feature "std" (*)
│   │   ├── rustls-webpki feature "ring" (*)
│   │   ├── url feature "default" (*)
│   │   ├── url feature "serde" (*)
│   │   ├── iroh-base feature "key" (*)
│   │   ├── iroh-base feature "relay" (*)
│   │   ├── n0-error feature "default" (*)
│   │   ├── n0-future feature "default" (*)
│   │   ├── pin-project feature "default" (*)
│   │   ├── noq feature "runtime-tokio"
│   │   │   ├── noq v0.17.0 (*)
│   │   │   ├── tokio feature "net" (*)
│   │   │   ├── tokio feature "rt" (*)
│   │   │   └── tokio feature "time" (*)
│   │   ├── noq feature "rustls-ring" (*)
│   │   ├── noq-proto feature "default" (*)
│   │   ├── rustc-hash feature "default" (*)
│   │   ├── noq-udp feature "default" (*)
│   │   ├── tokio-stream feature "default" (*)
│   │   ├── tokio-stream feature "sync" (*)
│   │   ├── reqwest feature "rustls-tls" (*)
│   │   ├── reqwest feature "stream"
│   │   │   ├── reqwest v0.12.28 (*)
│   │   │   └── tokio feature "fs" (*)
│   │   ├── webpki-roots feature "default" (*)
│   │   ├── sync_wrapper feature "default" (*)
│   │   ├── sync_wrapper feature "futures" (*)
│   │   ├── strum feature "default" (*)
│   │   ├── strum feature "derive" (*)
│   │   ├── n0-watcher feature "default" (*)
│   │   └── netwatch feature "default" (*)
│   │   [build-dependencies]
│   │   └── cfg_aliases feature "default" (*)
│   ├── iroh feature "fast-apple-datapath"
│   │   ├── iroh v0.97.0 (*)
│   │   └── noq feature "fast-apple-datapath"
│   │       ├── noq v0.17.0 (*)
│   │       └── noq-udp feature "fast-apple-datapath"
│   │           └── noq-udp v0.9.0 (*)
│   ├── iroh feature "metrics"
│   │   ├── iroh v0.97.0 (*)
│   │   ├── iroh-metrics feature "metrics"
│   │   │   └── iroh-metrics v0.38.3 (*)
│   │   ├── iroh-relay feature "metrics"
│   │   │   ├── iroh-relay v0.97.0 (*)
│   │   │   └── iroh-metrics feature "metrics" (*)
│   │   └── portmapper feature "metrics"
│   │       ├── portmapper v0.15.0 (*)
│   │       └── iroh-metrics feature "metrics" (*)
│   └── iroh feature "portmapper"
│       └── iroh v0.97.0 (*)
├── proptest feature "default"
│   ├── proptest v1.10.0
│   │   ├── num-traits v0.2.19
│   │   │   [build-dependencies]
│   │   │   └── autocfg feature "default"
│   │   │       └── autocfg v1.5.0
│   │   ├── rand_chacha v0.9.0 (*)
│   │   ├── rusty-fork v0.3.1
│   │   │   ├── fnv feature "default" (*)
│   │   │   ├── quick-error feature "default"
│   │   │   │   └── quick-error v1.2.3
│   │   │   ├── tempfile feature "default"
│   │   │   │   ├── tempfile v3.27.0
│   │   │   │   │   ├── getrandom v0.4.2 (*)
│   │   │   │   │   ├── fastrand feature "default"
│   │   │   │   │   │   ├── fastrand v2.3.0
│   │   │   │   │   │   └── fastrand feature "std" (*)
│   │   │   │   │   ├── once_cell feature "std" (*)
│   │   │   │   │   ├── rustix feature "default"
│   │   │   │   │   │   ├── rustix v1.1.4
│   │   │   │   │   │   │   ├── bitflags v2.11.0
│   │   │   │   │   │   │   ├── linux-raw-sys feature "auxvec"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   ├── linux-raw-sys feature "elf"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   ├── linux-raw-sys feature "errno"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   ├── linux-raw-sys feature "general"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   ├── linux-raw-sys feature "ioctl"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   └── linux-raw-sys feature "no_std"
│   │   │   │   │   │   │       └── linux-raw-sys v0.12.1
│   │   │   │   │   │   └── rustix feature "std"
│   │   │   │   │   │       ├── rustix v1.1.4 (*)
│   │   │   │   │   │       ├── bitflags feature "std"
│   │   │   │   │   │       │   └── bitflags v2.11.0
│   │   │   │   │   │       └── rustix feature "alloc"
│   │   │   │   │   │           └── rustix v1.1.4 (*)
│   │   │   │   │   └── rustix feature "fs"
│   │   │   │   │       └── rustix v1.1.4 (*)
│   │   │   │   └── tempfile feature "getrandom"
│   │   │   │       └── tempfile v3.27.0 (*)
│   │   │   └── wait-timeout feature "default"
│   │   │       └── wait-timeout v0.2.1
│   │   │           └── libc feature "default" (*)
│   │   ├── rand feature "alloc" (*)
│   │   ├── bitflags feature "default" (*)
│   │   ├── bit-set feature "default"
│   │   │   ├── bit-set v0.8.0
│   │   │   │   └── bit-vec v0.8.0
│   │   │   └── bit-set feature "std"
│   │   │       ├── bit-set v0.8.0 (*)
│   │   │       └── bit-vec feature "std"
│   │   │           └── bit-vec v0.8.0
│   │   ├── bit-vec feature "default"
│   │   │   ├── bit-vec v0.8.0
│   │   │   └── bit-vec feature "std" (*)
│   │   ├── rand_xorshift feature "default"
│   │   │   └── rand_xorshift v0.4.0
│   │   │       └── rand_core feature "default" (*)
│   │   ├── regex-syntax feature "default"
│   │   │   ├── regex-syntax v0.8.10
│   │   │   ├── regex-syntax feature "std"
│   │   │   │   └── regex-syntax v0.8.10
│   │   │   └── regex-syntax feature "unicode"
│   │   │       ├── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-age"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-bool"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-case"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-gencat"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-perl"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-script"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       └── regex-syntax feature "unicode-segment"
│   │   │           └── regex-syntax v0.8.10
│   │   ├── tempfile feature "default" (*)
│   │   └── unarray feature "default"
│   │       └── unarray v0.1.4
│   ├── proptest feature "bit-set"
│   │   └── proptest v1.10.0 (*)
│   ├── proptest feature "fork"
│   │   ├── proptest v1.10.0 (*)
│   │   ├── proptest feature "rusty-fork"
│   │   │   └── proptest v1.10.0 (*)
│   │   ├── proptest feature "std"
│   │   │   ├── proptest v1.10.0 (*)
│   │   │   ├── rand feature "os_rng" (*)
│   │   │   ├── rand feature "std" (*)
│   │   │   ├── proptest feature "regex-syntax"
│   │   │   │   └── proptest v1.10.0 (*)
│   │   │   └── num-traits feature "std"
│   │   │       └── num-traits v0.2.19 (*)
│   │   └── proptest feature "tempfile"
│   │       └── proptest v1.10.0 (*)
│   ├── proptest feature "std" (*)
│   └── proptest feature "timeout"
│       ├── proptest v1.10.0 (*)
│       ├── proptest feature "fork" (*)
│       ├── proptest feature "rusty-fork" (*)
│       └── rusty-fork feature "timeout"
│           ├── rusty-fork v0.3.1 (*)
│           └── rusty-fork feature "wait-timeout"
│               └── rusty-fork v0.3.1 (*)
└── trybuild feature "default"
    └── trybuild v1.0.116
        ├── serde feature "default" (*)
        ├── serde_derive feature "default" (*)
        ├── serde_json feature "default"
        │   ├── serde_json v1.0.149 (*)
        │   └── serde_json feature "std"
        │       ├── serde_json v1.0.149 (*)
        │       ├── serde_core feature "std" (*)
        │       └── memchr feature "std" (*)
        ├── glob feature "default"
        │   └── glob v0.3.3
        ├── target-triple feature "default"
        │   └── target-triple v1.0.0
        ├── termcolor feature "default"
        │   └── termcolor v1.4.1
        └── toml feature "default"
            ├── toml v1.0.7+spec-1.1.0
            │   ├── winnow v1.0.0
            │   ├── serde_core feature "alloc" (*)
            │   ├── serde_spanned feature "alloc"
            │   │   ├── serde_spanned v1.0.4
            │   │   │   └── serde_core v1.0.228
            │   │   └── serde_core feature "alloc" (*)
            │   ├── toml_datetime feature "alloc"
            │   │   ├── toml_datetime v1.0.1+spec-1.1.0
            │   │   │   └── serde_core v1.0.228
            │   │   └── serde_core feature "alloc" (*)
            │   ├── toml_parser feature "alloc"
            │   │   └── toml_parser v1.0.10+spec-1.1.0
            │   │       └── winnow v1.0.0
            │   └── toml_writer feature "alloc"
            │       └── toml_writer v1.0.7+spec-1.1.0
            ├── toml feature "display"
            │   └── toml v1.0.7+spec-1.1.0 (*)
            ├── toml feature "parse"
            │   └── toml v1.0.7+spec-1.1.0 (*)
            ├── toml feature "serde"
            │   ├── toml v1.0.7+spec-1.1.0 (*)
            │   ├── serde_spanned feature "serde"
            │   │   └── serde_spanned v1.0.4 (*)
            │   └── toml_datetime feature "serde"
            │       └── toml_datetime v1.0.1+spec-1.1.0 (*)
            └── toml feature "std"
                ├── toml v1.0.7+spec-1.1.0 (*)
                ├── serde_core feature "std" (*)
                ├── serde_spanned feature "std"
                │   ├── serde_spanned v1.0.4 (*)
                │   ├── serde_core feature "std" (*)
                │   └── serde_spanned feature "alloc" (*)
                ├── toml_datetime feature "std"
                │   ├── toml_datetime v1.0.1+spec-1.1.0 (*)
                │   ├── serde_core feature "std" (*)
                │   └── toml_datetime feature "alloc" (*)
                ├── toml_parser feature "std"
                │   ├── toml_parser v1.0.10+spec-1.1.0 (*)
                │   └── toml_parser feature "alloc" (*)
                └── toml_writer feature "std"
                    ├── toml_writer v1.0.7+spec-1.1.0
                    └── toml_writer feature "alloc" (*)
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo tree -p aspen-hooks-ticket --no-default-features -e normal`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-hooks-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks-ticket)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   ├── serde v1.0.228
│   │   ├── serde_core v1.0.228
│   │   └── serde_derive v1.0.228 (proc-macro)
│   │       ├── proc-macro2 v1.0.106
│   │       │   └── unicode-ident v1.0.24
│   │       ├── quote v1.0.45
│   │       │   └── proc-macro2 v1.0.106 (*)
│   │       └── syn v2.0.117
│   │           ├── proc-macro2 v1.0.106 (*)
│   │           ├── quote v1.0.45 (*)
│   │           └── unicode-ident v1.0.24
│   └── thiserror v2.0.18
│       └── thiserror-impl v2.0.18 (proc-macro)
│           ├── proc-macro2 v1.0.106 (*)
│           ├── quote v1.0.45 (*)
│           └── syn v2.0.117 (*)
├── iroh-tickets v0.4.0
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1
│   │   └── derive_more-impl v2.1.1 (proc-macro)
│   │       ├── convert_case v0.10.0
│   │       │   └── unicode-segmentation v1.12.0
│   │       ├── proc-macro2 v1.0.106 (*)
│   │       ├── quote v1.0.45 (*)
│   │       ├── syn v2.0.117 (*)
│   │       └── unicode-xid v0.2.6
│   ├── iroh-base v0.97.0
│   │   ├── curve25519-dalek v5.0.0-pre.1
│   │   │   ├── cfg-if v1.0.4
│   │   │   ├── cpufeatures v0.2.17
│   │   │   ├── curve25519-dalek-derive v0.1.1 (proc-macro)
│   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   └── syn v2.0.117 (*)
│   │   │   ├── digest v0.11.0-rc.10
│   │   │   │   ├── block-buffer v0.11.0
│   │   │   │   │   └── hybrid-array v0.4.8
│   │   │   │   │       └── typenum v1.19.0
│   │   │   │   ├── const-oid v0.10.2
│   │   │   │   └── crypto-common v0.2.1
│   │   │   │       └── hybrid-array v0.4.8 (*)
│   │   │   ├── rand_core v0.9.5
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── subtle v2.6.1
│   │   │   └── zeroize v1.8.2
│   │   │       └── zeroize_derive v1.4.3 (proc-macro)
│   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │           ├── quote v1.0.45 (*)
│   │   │           └── syn v2.0.117 (*)
│   │   ├── data-encoding v2.10.0
│   │   ├── derive_more v2.1.1 (*)
│   │   ├── digest v0.11.0-rc.10 (*)
│   │   ├── ed25519-dalek v3.0.0-pre.1
│   │   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│   │   │   ├── ed25519 v3.0.0-rc.4
│   │   │   │   ├── serde v1.0.228 (*)
│   │   │   │   └── signature v3.0.0-rc.10
│   │   │   ├── rand_core v0.9.5
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── sha2 v0.11.0-rc.2
│   │   │   │   ├── cfg-if v1.0.4
│   │   │   │   ├── cpufeatures v0.2.17
│   │   │   │   └── digest v0.11.0-rc.10 (*)
│   │   │   ├── subtle v2.6.1
│   │   │   └── zeroize v1.8.2 (*)
│   │   ├── n0-error v0.1.3
│   │   │   ├── n0-error-macros v0.1.3 (proc-macro)
│   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   └── syn v2.0.117 (*)
│   │   │   └── spez v0.1.2 (proc-macro)
│   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │       ├── quote v1.0.45 (*)
│   │   │       └── syn v2.0.117 (*)
│   │   ├── rand_core v0.9.5
│   │   ├── serde v1.0.228 (*)
│   │   ├── sha2 v0.11.0-rc.2 (*)
│   │   ├── url v2.5.8
│   │   │   ├── form_urlencoded v1.2.2
│   │   │   │   └── percent-encoding v2.3.2
│   │   │   ├── idna v1.1.0
│   │   │   │   ├── idna_adapter v1.2.1
│   │   │   │   │   ├── icu_normalizer v2.1.1
│   │   │   │   │   │   ├── icu_collections v2.1.1
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro)
│   │   │   │   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   │   │   │   ├── potential_utf v0.1.4
│   │   │   │   │   │   │   │   └── zerovec v0.11.5
│   │   │   │   │   │   │   │       ├── yoke v0.8.1
│   │   │   │   │   │   │   │       │   ├── stable_deref_trait v1.2.1
│   │   │   │   │   │   │   │       │   ├── yoke-derive v0.8.1 (proc-macro)
│   │   │   │   │   │   │   │       │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │       │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │       │   │   ├── syn v2.0.117 (*)
│   │   │   │   │   │   │   │       │   │   └── synstructure v0.13.2
│   │   │   │   │   │   │   │       │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │       │   │       ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │       │   │       └── syn v2.0.117 (*)
│   │   │   │   │   │   │   │       │   └── zerofrom v0.1.6
│   │   │   │   │   │   │   │       │       └── zerofrom-derive v0.1.6 (proc-macro)
│   │   │   │   │   │   │   │       │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │       │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │       │           ├── syn v2.0.117 (*)
│   │   │   │   │   │   │   │       │           └── synstructure v0.13.2 (*)
│   │   │   │   │   │   │   │       ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   │       └── zerovec-derive v0.11.2 (proc-macro)
│   │   │   │   │   │   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │           └── syn v2.0.117 (*)
│   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   ├── icu_normalizer_data v2.1.1
│   │   │   │   │   │   ├── icu_provider v2.1.1
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   ├── icu_locale_core v2.1.1
│   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   ├── litemap v0.8.1
│   │   │   │   │   │   │   │   ├── tinystr v0.8.2
│   │   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   ├── zerotrie v0.2.3
│   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   │   └── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   ├── smallvec v1.15.1
│   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   └── icu_properties v2.1.2
│   │   │   │   │       ├── icu_collections v2.1.1 (*)
│   │   │   │   │       ├── icu_locale_core v2.1.1 (*)
│   │   │   │   │       ├── icu_properties_data v2.1.2
│   │   │   │   │       ├── icu_provider v2.1.1 (*)
│   │   │   │   │       ├── zerotrie v0.2.3 (*)
│   │   │   │   │       └── zerovec v0.11.5 (*)
│   │   │   │   ├── smallvec v1.15.1
│   │   │   │   └── utf8_iter v1.0.4
│   │   │   ├── percent-encoding v2.3.2
│   │   │   ├── serde v1.0.228 (*)
│   │   │   └── serde_derive v1.0.228 (proc-macro) (*)
│   │   ├── zeroize v1.8.2 (*)
│   │   └── zeroize_derive v1.4.3 (proc-macro) (*)
│   ├── n0-error v0.1.3 (*)
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18 (*)
│   │   ├── heapless v0.7.17
│   │   │   ├── hash32 v0.2.1
│   │   │   │   └── byteorder v1.5.0
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── spin v0.9.8
│   │   │   │   └── lock_api v0.4.14
│   │   │   │       └── scopeguard v1.2.0
│   │   │   └── stable_deref_trait v1.2.1
│   │   └── serde v1.0.228 (*)
│   └── serde v1.0.228 (*)
├── postcard v1.1.3 (*)
├── serde v1.0.228 (*)
├── serde_json v1.0.149
│   ├── itoa v1.0.17
│   ├── memchr v2.8.0
│   ├── serde_core v1.0.228
│   └── zmij v1.0.21
└── thiserror v2.0.18 (*)
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo tree -p aspen-hooks-ticket --no-default-features -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-hooks-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks-ticket)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   ├── thiserror v2.0.18
│   │   └── thiserror-impl feature "default"
│   │       └── thiserror-impl v2.0.18 (proc-macro)
│   │           ├── proc-macro2 feature "default"
│   │           │   ├── proc-macro2 v1.0.106
│   │           │   │   └── unicode-ident feature "default"
│   │           │   │       └── unicode-ident v1.0.24
│   │           │   └── proc-macro2 feature "proc-macro"
│   │           │       └── proc-macro2 v1.0.106 (*)
│   │           ├── quote feature "default"
│   │           │   ├── quote v1.0.45
│   │           │   │   └── proc-macro2 v1.0.106 (*)
│   │           │   └── quote feature "proc-macro"
│   │           │       ├── quote v1.0.45 (*)
│   │           │       └── proc-macro2 feature "proc-macro" (*)
│   │           └── syn feature "default"
│   │               ├── syn v2.0.117
│   │               │   ├── proc-macro2 v1.0.106 (*)
│   │               │   ├── quote v1.0.45 (*)
│   │               │   └── unicode-ident feature "default" (*)
│   │               ├── syn feature "clone-impls"
│   │               │   └── syn v2.0.117 (*)
│   │               ├── syn feature "derive"
│   │               │   └── syn v2.0.117 (*)
│   │               ├── syn feature "parsing"
│   │               │   └── syn v2.0.117 (*)
│   │               ├── syn feature "printing"
│   │               │   └── syn v2.0.117 (*)
│   │               └── syn feature "proc-macro"
│   │                   ├── syn v2.0.117 (*)
│   │                   ├── proc-macro2 feature "proc-macro" (*)
│   │                   └── quote feature "proc-macro" (*)
│   ├── serde feature "alloc"
│   │   ├── serde v1.0.228
│   │   │   ├── serde_core feature "result"
│   │   │   │   └── serde_core v1.0.228
│   │   │   └── serde_derive feature "default"
│   │   │       └── serde_derive v1.0.228 (proc-macro)
│   │   │           ├── proc-macro2 feature "proc-macro" (*)
│   │   │           ├── quote feature "proc-macro" (*)
│   │   │           ├── syn feature "clone-impls" (*)
│   │   │           ├── syn feature "derive" (*)
│   │   │           ├── syn feature "parsing" (*)
│   │   │           ├── syn feature "printing" (*)
│   │   │           └── syn feature "proc-macro" (*)
│   │   └── serde_core feature "alloc"
│   │       └── serde_core v1.0.228
│   └── serde feature "derive"
│       ├── serde v1.0.228 (*)
│       └── serde feature "serde_derive"
│           └── serde v1.0.228 (*)
├── thiserror v2.0.18 (*)
├── serde feature "derive" (*)
├── postcard feature "alloc"
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18 (*)
│   │   ├── serde feature "derive" (*)
│   │   ├── heapless feature "serde"
│   │   │   └── heapless v0.7.17
│   │   │       ├── serde v1.0.228 (*)
│   │   │       ├── stable_deref_trait v1.2.1
│   │   │       ├── hash32 feature "default"
│   │   │       │   └── hash32 v0.2.1
│   │   │       │       └── byteorder v1.5.0
│   │   │       └── spin feature "default"
│   │   │           ├── spin v0.9.8
│   │   │           │   └── lock_api feature "default"
│   │   │           │       ├── lock_api v0.4.14
│   │   │           │       │   └── scopeguard v1.2.0
│   │   │           │       └── lock_api feature "atomic_usize"
│   │   │           │           └── lock_api v0.4.14 (*)
│   │   │           ├── spin feature "barrier"
│   │   │           │   ├── spin v0.9.8 (*)
│   │   │           │   └── spin feature "mutex"
│   │   │           │       └── spin v0.9.8 (*)
│   │   │           ├── spin feature "lazy"
│   │   │           │   ├── spin v0.9.8 (*)
│   │   │           │   └── spin feature "once"
│   │   │           │       └── spin v0.9.8 (*)
│   │   │           ├── spin feature "lock_api"
│   │   │           │   ├── spin v0.9.8 (*)
│   │   │           │   └── spin feature "lock_api_crate"
│   │   │           │       └── spin v0.9.8 (*)
│   │   │           ├── spin feature "mutex" (*)
│   │   │           ├── spin feature "once" (*)
│   │   │           ├── spin feature "rwlock"
│   │   │           │   └── spin v0.9.8 (*)
│   │   │           └── spin feature "spin_mutex"
│   │   │               ├── spin v0.9.8 (*)
│   │   │               └── spin feature "mutex" (*)
│   │   │       [build-dependencies]
│   │   │       └── rustc_version feature "default"
│   │   │           └── rustc_version v0.4.1
│   │   │               └── semver feature "default"
│   │   │                   ├── semver v1.0.27
│   │   │                   └── semver feature "std"
│   │   │                       └── semver v1.0.27
│   │   └── postcard-derive feature "default"
│   │       └── postcard-derive v0.2.2 (proc-macro)
│   │           ├── proc-macro2 feature "default" (*)
│   │           ├── quote feature "default" (*)
│   │           └── syn feature "default" (*)
│   └── serde feature "alloc" (*)
├── iroh-tickets feature "default"
│   └── iroh-tickets v0.4.0
│       ├── serde feature "default"
│       │   ├── serde v1.0.228 (*)
│       │   └── serde feature "std"
│       │       ├── serde v1.0.228 (*)
│       │       └── serde_core feature "std"
│       │           └── serde_core v1.0.228
│       ├── serde feature "derive" (*)
│       ├── data-encoding feature "default"
│       │   ├── data-encoding v2.10.0
│       │   └── data-encoding feature "std"
│       │       ├── data-encoding v2.10.0
│       │       └── data-encoding feature "alloc"
│       │           └── data-encoding v2.10.0
│       ├── derive_more feature "default"
│       │   ├── derive_more v2.1.1
│       │   │   └── derive_more-impl feature "default"
│       │   │       └── derive_more-impl v2.1.1 (proc-macro)
│       │   │           ├── proc-macro2 feature "default" (*)
│       │   │           ├── quote feature "default" (*)
│       │   │           ├── syn feature "default" (*)
│       │   │           ├── convert_case feature "default"
│       │   │           │   └── convert_case v0.10.0
│       │   │           │       └── unicode-segmentation feature "default"
│       │   │           │           └── unicode-segmentation v1.12.0
│       │   │           └── unicode-xid feature "default"
│       │   │               └── unicode-xid v0.2.6
│       │   │           [build-dependencies]
│       │   │           └── rustc_version feature "default" (*)
│       │   └── derive_more feature "std"
│       │       └── derive_more v2.1.1 (*)
│       ├── derive_more feature "display"
│       │   ├── derive_more v2.1.1 (*)
│       │   └── derive_more-impl feature "display"
│       │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│       │       └── syn feature "extra-traits"
│       │           └── syn v2.0.117 (*)
│       ├── iroh-base feature "default"
│       │   ├── iroh-base v0.97.0
│       │   │   ├── serde feature "default" (*)
│       │   │   ├── serde feature "derive" (*)
│       │   │   ├── serde feature "rc"
│       │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   └── serde_core feature "rc"
│       │   │   │       └── serde_core v1.0.228
│       │   │   ├── data-encoding feature "default" (*)
│       │   │   ├── derive_more feature "debug"
│       │   │   │   ├── derive_more v2.1.1 (*)
│       │   │   │   └── derive_more-impl feature "debug"
│       │   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│       │   │   │       └── syn feature "extra-traits" (*)
│       │   │   ├── derive_more feature "default" (*)
│       │   │   ├── derive_more feature "display" (*)
│       │   │   ├── ed25519-dalek feature "default"
│       │   │   │   ├── ed25519-dalek v3.0.0-pre.1
│       │   │   │   │   ├── ed25519 v3.0.0-rc.4
│       │   │   │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   │   │   ├── signature v3.0.0-rc.10
│       │   │   │   │   │   └── pkcs8 feature "default"
│       │   │   │   │   │       └── pkcs8 v0.11.0-rc.11
│       │   │   │   │   │           ├── der feature "default"
│       │   │   │   │   │           │   └── der v0.8.0
│       │   │   │   │   │           │       ├── zeroize v1.8.2
│       │   │   │   │   │           │       │   └── zeroize_derive feature "default"
│       │   │   │   │   │           │       │       └── zeroize_derive v1.4.3 (proc-macro)
│       │   │   │   │   │           │       │           ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │           │       │           ├── quote feature "default" (*)
│       │   │   │   │   │           │       │           ├── syn feature "default" (*)
│       │   │   │   │   │           │       │           ├── syn feature "extra-traits" (*)
│       │   │   │   │   │           │       │           ├── syn feature "full"
│       │   │   │   │   │           │       │           │   └── syn v2.0.117 (*)
│       │   │   │   │   │           │       │           └── syn feature "visit"
│       │   │   │   │   │           │       │               └── syn v2.0.117 (*)
│       │   │   │   │   │           │       ├── const-oid feature "default"
│       │   │   │   │   │           │       │   └── const-oid v0.10.2
│       │   │   │   │   │           │       ├── pem-rfc7468 feature "alloc"
│       │   │   │   │   │           │       │   ├── pem-rfc7468 v1.0.0
│       │   │   │   │   │           │       │   │   └── base64ct feature "default"
│       │   │   │   │   │           │       │   │       └── base64ct v1.8.3
│       │   │   │   │   │           │       │   └── base64ct feature "alloc"
│       │   │   │   │   │           │       │       └── base64ct v1.8.3
│       │   │   │   │   │           │       └── pem-rfc7468 feature "default"
│       │   │   │   │   │           │           └── pem-rfc7468 v1.0.0 (*)
│       │   │   │   │   │           ├── der feature "oid"
│       │   │   │   │   │           │   └── der v0.8.0 (*)
│       │   │   │   │   │           └── spki feature "default"
│       │   │   │   │   │               └── spki v0.8.0-rc.4
│       │   │   │   │   │                   ├── der feature "default" (*)
│       │   │   │   │   │                   └── der feature "oid" (*)
│       │   │   │   │   ├── rand_core v0.9.5
│       │   │   │   │   │   └── getrandom feature "default"
│       │   │   │   │   │       └── getrandom v0.3.4
│       │   │   │   │   │           ├── libc v0.2.183
│       │   │   │   │   │           └── cfg-if feature "default"
│       │   │   │   │   │               └── cfg-if v1.0.4
│       │   │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   │   ├── sha2 v0.11.0-rc.2
│       │   │   │   │   │   ├── cfg-if feature "default" (*)
│       │   │   │   │   │   ├── cpufeatures feature "default"
│       │   │   │   │   │   │   └── cpufeatures v0.2.17
│       │   │   │   │   │   └── digest feature "default"
│       │   │   │   │   │       ├── digest v0.11.0-rc.10
│       │   │   │   │   │       │   ├── block-buffer feature "default"
│       │   │   │   │   │       │   │   └── block-buffer v0.11.0
│       │   │   │   │   │       │   │       └── hybrid-array feature "default"
│       │   │   │   │   │       │   │           └── hybrid-array v0.4.8
│       │   │   │   │   │       │   │               ├── typenum feature "const-generics"
│       │   │   │   │   │       │   │               │   └── typenum v1.19.0
│       │   │   │   │   │       │   │               └── typenum feature "default"
│       │   │   │   │   │       │   │                   └── typenum v1.19.0
│       │   │   │   │   │       │   ├── const-oid feature "default" (*)
│       │   │   │   │   │       │   └── crypto-common feature "default"
│       │   │   │   │   │       │       └── crypto-common v0.2.1
│       │   │   │   │   │       │           └── hybrid-array feature "default" (*)
│       │   │   │   │   │       └── digest feature "block-api"
│       │   │   │   │   │           ├── digest v0.11.0-rc.10 (*)
│       │   │   │   │   │           └── digest feature "block-buffer"
│       │   │   │   │   │               └── digest v0.11.0-rc.10 (*)
│       │   │   │   │   ├── signature v3.0.0-rc.10
│       │   │   │   │   ├── subtle v2.6.1
│       │   │   │   │   ├── zeroize v1.8.2 (*)
│       │   │   │   │   └── curve25519-dalek feature "digest"
│       │   │   │   │       └── curve25519-dalek v5.0.0-pre.1
│       │   │   │   │           ├── rand_core v0.9.5 (*)
│       │   │   │   │           ├── zeroize v1.8.2 (*)
│       │   │   │   │           ├── serde feature "derive" (*)
│       │   │   │   │           ├── cfg-if feature "default" (*)
│       │   │   │   │           ├── cpufeatures feature "default" (*)
│       │   │   │   │           ├── curve25519-dalek-derive feature "default"
│       │   │   │   │           │   └── curve25519-dalek-derive v0.1.1 (proc-macro)
│       │   │   │   │           │       ├── proc-macro2 feature "default" (*)
│       │   │   │   │           │       ├── quote feature "default" (*)
│       │   │   │   │           │       ├── syn feature "default" (*)
│       │   │   │   │           │       └── syn feature "full" (*)
│       │   │   │   │           ├── digest feature "block-api" (*)
│       │   │   │   │           └── subtle feature "const-generics"
│       │   │   │   │               └── subtle v2.6.1
│       │   │   │   │           [build-dependencies]
│       │   │   │   │           └── rustc_version feature "default" (*)
│       │   │   │   ├── ed25519-dalek feature "fast"
│       │   │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   │   │   └── curve25519-dalek feature "precomputed-tables"
│       │   │   │   │       └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   │   └── ed25519-dalek feature "zeroize"
│       │   │   │       ├── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   │       └── curve25519-dalek feature "zeroize"
│       │   │   │           └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   ├── ed25519-dalek feature "rand_core"
│       │   │   │   └── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   ├── ed25519-dalek feature "serde"
│       │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   │   └── ed25519 feature "serde"
│       │   │   │       └── ed25519 v3.0.0-rc.4 (*)
│       │   │   ├── ed25519-dalek feature "zeroize" (*)
│       │   │   ├── curve25519-dalek feature "default"
│       │   │   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   │   ├── curve25519-dalek feature "alloc"
│       │   │   │   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   │   │   └── zeroize feature "alloc"
│       │   │   │   │       └── zeroize v1.8.2 (*)
│       │   │   │   ├── curve25519-dalek feature "precomputed-tables" (*)
│       │   │   │   └── curve25519-dalek feature "zeroize" (*)
│       │   │   ├── curve25519-dalek feature "rand_core"
│       │   │   │   └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   ├── curve25519-dalek feature "serde"
│       │   │   │   └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   ├── curve25519-dalek feature "zeroize" (*)
│       │   │   ├── digest feature "default" (*)
│       │   │   ├── rand_core feature "default"
│       │   │   │   └── rand_core v0.9.5 (*)
│       │   │   ├── zeroize feature "default"
│       │   │   │   ├── zeroize v1.8.2 (*)
│       │   │   │   └── zeroize feature "alloc" (*)
│       │   │   ├── zeroize feature "derive"
│       │   │   │   ├── zeroize v1.8.2 (*)
│       │   │   │   └── zeroize feature "zeroize_derive"
│       │   │   │       └── zeroize v1.8.2 (*)
│       │   │   ├── zeroize_derive feature "default" (*)
│       │   │   ├── sha2 feature "default"
│       │   │   │   ├── sha2 v0.11.0-rc.2 (*)
│       │   │   │   ├── sha2 feature "alloc"
│       │   │   │   │   ├── sha2 v0.11.0-rc.2 (*)
│       │   │   │   │   └── digest feature "alloc"
│       │   │   │   │       └── digest v0.11.0-rc.10 (*)
│       │   │   │   └── sha2 feature "oid"
│       │   │   │       ├── sha2 v0.11.0-rc.2 (*)
│       │   │   │       └── digest feature "oid"
│       │   │   │           ├── digest v0.11.0-rc.10 (*)
│       │   │   │           └── digest feature "const-oid"
│       │   │   │               └── digest v0.11.0-rc.10 (*)
│       │   │   ├── url feature "default"
│       │   │   │   ├── url v2.5.8
│       │   │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   │   ├── serde_derive v1.0.228 (proc-macro) (*)
│       │   │   │   │   ├── idna feature "alloc"
│       │   │   │   │   │   └── idna v1.1.0
│       │   │   │   │   │       ├── idna_adapter feature "default"
│       │   │   │   │   │       │   └── idna_adapter v1.2.1
│       │   │   │   │   │       │       ├── icu_normalizer v2.1.1
│       │   │   │   │   │       │       │   ├── icu_collections v2.1.1
│       │   │   │   │   │       │       │   │   ├── displaydoc v0.2.5 (proc-macro)
│       │   │   │   │   │       │       │   │   │   ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │       │       │   │   │   ├── quote feature "default" (*)
│       │   │   │   │   │       │       │   │   │   └── syn feature "default" (*)
│       │   │   │   │   │       │       │   │   ├── potential_utf feature "zerovec"
│       │   │   │   │   │       │       │   │   │   └── potential_utf v0.1.4
│       │   │   │   │   │       │       │   │   │       └── zerovec v0.11.5
│       │   │   │   │   │       │       │   │   │           ├── yoke v0.8.1
│       │   │   │   │   │       │       │   │   │           │   ├── stable_deref_trait v1.2.1
│       │   │   │   │   │       │       │   │   │           │   ├── yoke-derive v0.8.1 (proc-macro)
│       │   │   │   │   │       │       │   │   │           │   │   ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │   │   ├── quote feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │   │   ├── syn feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │   │   ├── syn feature "fold"
│       │   │   │   │   │       │       │   │   │           │   │   │   └── syn v2.0.117 (*)
│       │   │   │   │   │       │       │   │   │           │   │   └── synstructure feature "default"
│       │   │   │   │   │       │       │   │   │           │   │       ├── synstructure v0.13.2
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── proc-macro2 v1.0.106 (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── quote v1.0.45 (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "clone-impls" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "derive" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "extra-traits" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "parsing" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   ├── syn feature "printing" (*)
│       │   │   │   │   │       │       │   │   │           │   │       │   └── syn feature "visit" (*)
│       │   │   │   │   │       │       │   │   │           │   │       └── synstructure feature "proc-macro"
│       │   │   │   │   │       │       │   │   │           │   │           ├── synstructure v0.13.2 (*)
│       │   │   │   │   │       │       │   │   │           │   │           ├── proc-macro2 feature "proc-macro" (*)
│       │   │   │   │   │       │       │   │   │           │   │           ├── quote feature "proc-macro" (*)
│       │   │   │   │   │       │       │   │   │           │   │           └── syn feature "proc-macro" (*)
│       │   │   │   │   │       │       │   │   │           │   └── zerofrom v0.1.6
│       │   │   │   │   │       │       │   │   │           │       └── zerofrom-derive v0.1.6 (proc-macro)
│       │   │   │   │   │       │       │   │   │           │           ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │           ├── quote feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │           ├── syn feature "default" (*)
│       │   │   │   │   │       │       │   │   │           │           ├── syn feature "fold" (*)
│       │   │   │   │   │       │       │   │   │           │           └── synstructure feature "default" (*)
│       │   │   │   │   │       │       │   │   │           ├── zerofrom v0.1.6 (*)
│       │   │   │   │   │       │       │   │   │           └── zerovec-derive v0.11.2 (proc-macro)
│       │   │   │   │   │       │       │   │   │               ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │       │       │   │   │               ├── quote feature "default" (*)
│       │   │   │   │   │       │       │   │   │               ├── syn feature "default" (*)
│       │   │   │   │   │       │       │   │   │               └── syn feature "extra-traits" (*)
│       │   │   │   │   │       │       │   │   ├── zerovec feature "derive"
│       │   │   │   │   │       │       │   │   │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       │   │   ├── zerovec feature "yoke"
│       │   │   │   │   │       │       │   │   │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       │   │   ├── yoke feature "derive"
│       │   │   │   │   │       │       │   │   │   ├── yoke v0.8.1 (*)
│       │   │   │   │   │       │       │   │   │   ├── yoke feature "zerofrom"
│       │   │   │   │   │       │       │   │   │   │   └── yoke v0.8.1 (*)
│       │   │   │   │   │       │       │   │   │   └── zerofrom feature "derive"
│       │   │   │   │   │       │       │   │   │       └── zerofrom v0.1.6 (*)
│       │   │   │   │   │       │       │   │   └── zerofrom feature "derive" (*)
│       │   │   │   │   │       │       │   ├── icu_normalizer_data v2.1.1
│       │   │   │   │   │       │       │   ├── icu_provider v2.1.1
│       │   │   │   │   │       │       │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│       │   │   │   │   │       │       │   │   ├── icu_locale_core v2.1.1
│       │   │   │   │   │       │       │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│       │   │   │   │   │       │       │   │   │   ├── litemap v0.8.1
│       │   │   │   │   │       │       │   │   │   ├── tinystr v0.8.2
│       │   │   │   │   │       │       │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│       │   │   │   │   │       │       │   │   │   │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       │   │   │   ├── writeable v0.6.2
│       │   │   │   │   │       │       │   │   │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       │   │   ├── writeable v0.6.2
│       │   │   │   │   │       │       │   │   ├── zerotrie v0.2.3
│       │   │   │   │   │       │       │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│       │   │   │   │   │       │       │   │   │   ├── zerofrom v0.1.6 (*)
│       │   │   │   │   │       │       │   │   │   └── yoke feature "derive" (*)
│       │   │   │   │   │       │       │   │   ├── zerovec feature "derive" (*)
│       │   │   │   │   │       │       │   │   ├── yoke feature "derive" (*)
│       │   │   │   │   │       │       │   │   └── zerofrom feature "derive" (*)
│       │   │   │   │   │       │       │   ├── smallvec v1.15.1
│       │   │   │   │   │       │       │   └── zerovec v0.11.5 (*)
│       │   │   │   │   │       │       └── icu_properties v2.1.2
│       │   │   │   │   │       │           ├── icu_collections v2.1.1 (*)
│       │   │   │   │   │       │           ├── icu_properties_data v2.1.2
│       │   │   │   │   │       │           ├── icu_provider v2.1.1 (*)
│       │   │   │   │   │       │           ├── zerovec feature "derive" (*)
│       │   │   │   │   │       │           ├── zerovec feature "yoke" (*)
│       │   │   │   │   │       │           ├── icu_locale_core feature "zerovec"
│       │   │   │   │   │       │           │   ├── icu_locale_core v2.1.1 (*)
│       │   │   │   │   │       │           │   └── tinystr feature "zerovec"
│       │   │   │   │   │       │           │       └── tinystr v0.8.2 (*)
│       │   │   │   │   │       │           ├── zerotrie feature "yoke"
│       │   │   │   │   │       │           │   └── zerotrie v0.2.3 (*)
│       │   │   │   │   │       │           └── zerotrie feature "zerofrom"
│       │   │   │   │   │       │               └── zerotrie v0.2.3 (*)
│       │   │   │   │   │       ├── smallvec feature "const_generics"
│       │   │   │   │   │       │   └── smallvec v1.15.1
│       │   │   │   │   │       ├── smallvec feature "default"
│       │   │   │   │   │       │   └── smallvec v1.15.1
│       │   │   │   │   │       └── utf8_iter feature "default"
│       │   │   │   │   │           └── utf8_iter v1.0.4
│       │   │   │   │   ├── idna feature "compiled_data"
│       │   │   │   │   │   ├── idna v1.1.0 (*)
│       │   │   │   │   │   └── idna_adapter feature "compiled_data"
│       │   │   │   │   │       ├── idna_adapter v1.2.1 (*)
│       │   │   │   │   │       ├── icu_normalizer feature "compiled_data"
│       │   │   │   │   │       │   ├── icu_normalizer v2.1.1 (*)
│       │   │   │   │   │       │   └── icu_provider feature "baked"
│       │   │   │   │   │       │       └── icu_provider v2.1.1 (*)
│       │   │   │   │   │       └── icu_properties feature "compiled_data"
│       │   │   │   │   │           ├── icu_properties v2.1.2 (*)
│       │   │   │   │   │           └── icu_provider feature "baked" (*)
│       │   │   │   │   ├── form_urlencoded feature "alloc"
│       │   │   │   │   │   ├── form_urlencoded v1.2.2
│       │   │   │   │   │   │   └── percent-encoding v2.3.2
│       │   │   │   │   │   └── percent-encoding feature "alloc"
│       │   │   │   │   │       └── percent-encoding v2.3.2
│       │   │   │   │   └── percent-encoding feature "alloc" (*)
│       │   │   │   └── url feature "std"
│       │   │   │       ├── url v2.5.8 (*)
│       │   │   │       ├── serde feature "std" (*)
│       │   │   │       ├── idna feature "std"
│       │   │   │       │   ├── idna v1.1.0 (*)
│       │   │   │       │   └── idna feature "alloc" (*)
│       │   │   │       ├── form_urlencoded feature "std"
│       │   │   │       │   ├── form_urlencoded v1.2.2 (*)
│       │   │   │       │   ├── form_urlencoded feature "alloc" (*)
│       │   │   │       │   └── percent-encoding feature "std"
│       │   │   │       │       ├── percent-encoding v2.3.2
│       │   │   │       │       └── percent-encoding feature "alloc" (*)
│       │   │   │       └── percent-encoding feature "std" (*)
│       │   │   ├── url feature "serde"
│       │   │   │   └── url v2.5.8 (*)
│       │   │   └── n0-error feature "default"
│       │   │       └── n0-error v0.1.3
│       │   │           ├── n0-error-macros feature "default"
│       │   │           │   └── n0-error-macros v0.1.3 (proc-macro)
│       │   │           │       ├── proc-macro2 feature "default" (*)
│       │   │           │       ├── quote feature "default" (*)
│       │   │           │       ├── syn feature "default" (*)
│       │   │           │       ├── syn feature "extra-traits" (*)
│       │   │           │       └── syn feature "full" (*)
│       │   │           └── spez feature "default"
│       │   │               └── spez v0.1.2 (proc-macro)
│       │   │                   ├── proc-macro2 feature "default" (*)
│       │   │                   ├── quote feature "default" (*)
│       │   │                   ├── syn feature "default" (*)
│       │   │                   └── syn feature "full" (*)
│       │   └── iroh-base feature "relay"
│       │       └── iroh-base v0.97.0 (*)
│       ├── iroh-base feature "key"
│       │   ├── iroh-base v0.97.0 (*)
│       │   └── iroh-base feature "relay" (*)
│       ├── n0-error feature "default" (*)
│       ├── postcard feature "default"
│       │   ├── postcard v1.1.3 (*)
│       │   └── postcard feature "heapless-cas"
│       │       ├── postcard v1.1.3 (*)
│       │       ├── postcard feature "heapless"
│       │       │   └── postcard v1.1.3 (*)
│       │       └── heapless feature "cas"
│       │           ├── heapless v0.7.17 (*)
│       │           └── heapless feature "atomic-polyfill"
│       │               └── heapless v0.7.17 (*)
│       └── postcard feature "use-std"
│           ├── postcard v1.1.3 (*)
│           ├── serde feature "std" (*)
│           └── postcard feature "alloc" (*)
└── serde_json feature "alloc"
    ├── serde_json v1.0.149
    │   ├── memchr v2.8.0
    │   ├── serde_core v1.0.228
    │   ├── itoa feature "default"
    │   │   └── itoa v1.0.17
    │   └── zmij feature "default"
    │       └── zmij v1.0.21
    └── serde_core feature "alloc" (*)
[dev-dependencies]
├── iroh feature "default"
│   ├── iroh v0.97.0
│   │   ├── iroh-metrics v0.38.3
│   │   │   ├── serde feature "default" (*)
│   │   │   ├── serde feature "derive" (*)
│   │   │   ├── serde feature "rc" (*)
│   │   │   ├── itoa feature "default" (*)
│   │   │   ├── tracing feature "default"
│   │   │   │   ├── tracing v0.1.44
│   │   │   │   │   ├── tracing-core v0.1.36
│   │   │   │   │   │   └── once_cell feature "default"
│   │   │   │   │   │       ├── once_cell v1.21.4
│   │   │   │   │   │       │   ├── portable-atomic v1.13.1
│   │   │   │   │   │       │   │   └── serde v1.0.228 (*)
│   │   │   │   │   │       │   └── critical-section feature "default"
│   │   │   │   │   │       │       └── critical-section v1.2.0
│   │   │   │   │   │       └── once_cell feature "std"
│   │   │   │   │   │           ├── once_cell v1.21.4 (*)
│   │   │   │   │   │           └── once_cell feature "alloc"
│   │   │   │   │   │               ├── once_cell v1.21.4 (*)
│   │   │   │   │   │               └── once_cell feature "race"
│   │   │   │   │   │                   └── once_cell v1.21.4 (*)
│   │   │   │   │   ├── pin-project-lite feature "default"
│   │   │   │   │   │   └── pin-project-lite v0.2.17
│   │   │   │   │   ├── log feature "default"
│   │   │   │   │   │   └── log v0.4.29
│   │   │   │   │   └── tracing-attributes feature "default"
│   │   │   │   │       └── tracing-attributes v0.1.31 (proc-macro)
│   │   │   │   │           ├── proc-macro2 feature "default" (*)
│   │   │   │   │           ├── quote feature "default" (*)
│   │   │   │   │           ├── syn feature "clone-impls" (*)
│   │   │   │   │           ├── syn feature "extra-traits" (*)
│   │   │   │   │           ├── syn feature "full" (*)
│   │   │   │   │           ├── syn feature "parsing" (*)
│   │   │   │   │           ├── syn feature "printing" (*)
│   │   │   │   │           ├── syn feature "proc-macro" (*)
│   │   │   │   │           └── syn feature "visit-mut"
│   │   │   │   │               └── syn v2.0.117 (*)
│   │   │   │   ├── tracing feature "attributes"
│   │   │   │   │   ├── tracing v0.1.44 (*)
│   │   │   │   │   └── tracing feature "tracing-attributes"
│   │   │   │   │       └── tracing v0.1.44 (*)
│   │   │   │   └── tracing feature "std"
│   │   │   │       ├── tracing v0.1.44 (*)
│   │   │   │       └── tracing-core feature "std"
│   │   │   │           ├── tracing-core v0.1.36 (*)
│   │   │   │           └── tracing-core feature "once_cell"
│   │   │   │               └── tracing-core v0.1.36 (*)
│   │   │   ├── portable-atomic feature "default"
│   │   │   │   ├── portable-atomic v1.13.1 (*)
│   │   │   │   └── portable-atomic feature "fallback"
│   │   │   │       └── portable-atomic v1.13.1 (*)
│   │   │   ├── portable-atomic feature "serde"
│   │   │   │   └── portable-atomic v1.13.1 (*)
│   │   │   ├── n0-error feature "default" (*)
│   │   │   ├── iroh-metrics-derive feature "default"
│   │   │   │   └── iroh-metrics-derive v0.4.1 (proc-macro)
│   │   │   │       ├── proc-macro2 feature "default" (*)
│   │   │   │       ├── quote feature "default" (*)
│   │   │   │       ├── syn feature "default" (*)
│   │   │   │       └── heck feature "default"
│   │   │   │           └── heck v0.5.0
│   │   │   ├── postcard feature "default" (*)
│   │   │   ├── postcard feature "use-std" (*)
│   │   │   └── ryu feature "default"
│   │   │       └── ryu v1.0.23
│   │   ├── iroh-relay v0.97.0
│   │   │   ├── iroh-metrics v0.38.3 (*)
│   │   │   ├── serde feature "default" (*)
│   │   │   ├── serde feature "derive" (*)
│   │   │   ├── serde feature "rc" (*)
│   │   │   ├── tokio feature "default"
│   │   │   │   └── tokio v1.50.0
│   │   │   │       ├── mio v1.1.1
│   │   │   │       │   └── libc feature "default"
│   │   │   │       │       ├── libc v0.2.183
│   │   │   │       │       └── libc feature "std"
│   │   │   │       │           └── libc v0.2.183
│   │   │   │       ├── bytes feature "default"
│   │   │   │       │   ├── bytes v1.11.1
│   │   │   │       │   └── bytes feature "std"
│   │   │   │       │       └── bytes v1.11.1
│   │   │   │       ├── libc feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── socket2 feature "all"
│   │   │   │       │   └── socket2 v0.6.3
│   │   │   │       │       └── libc feature "default" (*)
│   │   │   │       ├── socket2 feature "default"
│   │   │   │       │   └── socket2 v0.6.3 (*)
│   │   │   │       └── tokio-macros feature "default"
│   │   │   │           └── tokio-macros v2.6.1 (proc-macro)
│   │   │   │               ├── proc-macro2 feature "default" (*)
│   │   │   │               ├── quote feature "default" (*)
│   │   │   │               ├── syn feature "default" (*)
│   │   │   │               └── syn feature "full" (*)
│   │   │   ├── tokio feature "fs"
│   │   │   │   └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "io-std"
│   │   │   │   └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "io-util"
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   └── tokio feature "bytes"
│   │   │   │       └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "macros"
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   └── tokio feature "tokio-macros"
│   │   │   │       └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "net"
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   ├── tokio feature "libc"
│   │   │   │   │   └── tokio v1.50.0 (*)
│   │   │   │   ├── tokio feature "mio"
│   │   │   │   │   └── tokio v1.50.0 (*)
│   │   │   │   ├── tokio feature "socket2"
│   │   │   │   │   └── tokio v1.50.0 (*)
│   │   │   │   ├── mio feature "net"
│   │   │   │   │   └── mio v1.1.1 (*)
│   │   │   │   ├── mio feature "os-ext"
│   │   │   │   │   ├── mio v1.1.1 (*)
│   │   │   │   │   └── mio feature "os-poll"
│   │   │   │   │       └── mio v1.1.1 (*)
│   │   │   │   └── mio feature "os-poll" (*)
│   │   │   ├── tokio feature "rt"
│   │   │   │   └── tokio v1.50.0 (*)
│   │   │   ├── tokio feature "sync"
│   │   │   │   └── tokio v1.50.0 (*)
│   │   │   ├── bytes feature "default" (*)
│   │   │   ├── data-encoding feature "default" (*)
│   │   │   ├── derive_more feature "debug" (*)
│   │   │   ├── derive_more feature "default" (*)
│   │   │   ├── derive_more feature "deref"
│   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   └── derive_more-impl feature "deref"
│   │   │   │       └── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │   ├── derive_more feature "display" (*)
│   │   │   ├── derive_more feature "from"
│   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   └── derive_more-impl feature "from"
│   │   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │   │       └── syn feature "extra-traits" (*)
│   │   │   ├── derive_more feature "try_into"
│   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   └── derive_more-impl feature "try_into"
│   │   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │   │       ├── syn feature "extra-traits" (*)
│   │   │   │       ├── syn feature "full" (*)
│   │   │   │       └── syn feature "visit-mut" (*)
│   │   │   ├── hickory-resolver feature "default"
│   │   │   │   ├── hickory-resolver v0.25.2
│   │   │   │   │   ├── thiserror v2.0.18 (*)
│   │   │   │   │   ├── tokio-rustls v0.26.4
│   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   └── rustls feature "std"
│   │   │   │   │   │       ├── rustls v0.23.37
│   │   │   │   │   │       │   ├── subtle v2.6.1
│   │   │   │   │   │       │   ├── zeroize feature "default" (*)
│   │   │   │   │   │       │   ├── log feature "default" (*)
│   │   │   │   │   │       │   ├── once_cell feature "alloc" (*)
│   │   │   │   │   │       │   ├── once_cell feature "race" (*)
│   │   │   │   │   │       │   ├── ring feature "default"
│   │   │   │   │   │       │   │   ├── ring v0.17.14
│   │   │   │   │   │       │   │   │   ├── cfg-if v1.0.4
│   │   │   │   │   │       │   │   │   ├── getrandom feature "default"
│   │   │   │   │   │       │   │   │   │   └── getrandom v0.2.17
│   │   │   │   │   │       │   │   │   │       ├── libc v0.2.183
│   │   │   │   │   │       │   │   │   │       └── cfg-if feature "default" (*)
│   │   │   │   │   │       │   │   │   └── untrusted feature "default"
│   │   │   │   │   │       │   │   │       └── untrusted v0.9.0
│   │   │   │   │   │       │   │   │   [build-dependencies]
│   │   │   │   │   │       │   │   │   └── cc v1.2.57
│   │   │   │   │   │       │   │   │       ├── find-msvc-tools feature "default"
│   │   │   │   │   │       │   │   │       │   └── find-msvc-tools v0.1.9
│   │   │   │   │   │       │   │   │       └── shlex feature "default"
│   │   │   │   │   │       │   │   │           ├── shlex v1.3.0
│   │   │   │   │   │       │   │   │           └── shlex feature "std"
│   │   │   │   │   │       │   │   │               └── shlex v1.3.0
│   │   │   │   │   │       │   │   ├── ring feature "alloc"
│   │   │   │   │   │       │   │   │   └── ring v0.17.14 (*)
│   │   │   │   │   │       │   │   └── ring feature "dev_urandom_fallback"
│   │   │   │   │   │       │   │       └── ring v0.17.14 (*)
│   │   │   │   │   │       │   ├── rustls-pki-types feature "alloc"
│   │   │   │   │   │       │   │   └── rustls-pki-types v1.14.0
│   │   │   │   │   │       │   │       └── zeroize feature "default" (*)
│   │   │   │   │   │       │   ├── rustls-pki-types feature "default"
│   │   │   │   │   │       │   │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   │       │   │   └── rustls-pki-types feature "alloc" (*)
│   │   │   │   │   │       │   └── rustls-webpki feature "alloc"
│   │   │   │   │   │       │       ├── rustls-webpki v0.103.9
│   │   │   │   │   │       │       │   ├── ring v0.17.14 (*)
│   │   │   │   │   │       │       │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   │       │       │   └── untrusted feature "default" (*)
│   │   │   │   │   │       │       ├── ring feature "alloc" (*)
│   │   │   │   │   │       │       └── rustls-pki-types feature "alloc" (*)
│   │   │   │   │   │       ├── once_cell feature "std" (*)
│   │   │   │   │   │       ├── rustls-pki-types feature "std"
│   │   │   │   │   │       │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   │       │   └── rustls-pki-types feature "alloc" (*)
│   │   │   │   │   │       └── rustls-webpki feature "std"
│   │   │   │   │   │           ├── rustls-webpki v0.103.9 (*)
│   │   │   │   │   │           ├── rustls-pki-types feature "std" (*)
│   │   │   │   │   │           └── rustls-webpki feature "alloc" (*)
│   │   │   │   │   ├── tracing v0.1.44 (*)
│   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   ├── cfg-if feature "default" (*)
│   │   │   │   │   ├── futures-util feature "std"
│   │   │   │   │   │   ├── futures-util v0.3.32
│   │   │   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   │   │   ├── futures-macro v0.3.32 (proc-macro)
│   │   │   │   │   │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   │   │   ├── quote feature "default" (*)
│   │   │   │   │   │   │   │   ├── syn feature "default" (*)
│   │   │   │   │   │   │   │   └── syn feature "full" (*)
│   │   │   │   │   │   │   ├── futures-sink v0.3.32
│   │   │   │   │   │   │   ├── futures-task v0.3.32
│   │   │   │   │   │   │   ├── slab v0.4.12
│   │   │   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   │   │   ├── futures-channel feature "std"
│   │   │   │   │   │   │   │   ├── futures-channel v0.3.32
│   │   │   │   │   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   │   │   │   │   └── futures-sink v0.3.32
│   │   │   │   │   │   │   │   ├── futures-channel feature "alloc"
│   │   │   │   │   │   │   │   │   ├── futures-channel v0.3.32 (*)
│   │   │   │   │   │   │   │   │   └── futures-core feature "alloc"
│   │   │   │   │   │   │   │   │       └── futures-core v0.3.32
│   │   │   │   │   │   │   │   └── futures-core feature "std"
│   │   │   │   │   │   │   │       ├── futures-core v0.3.32
│   │   │   │   │   │   │   │       └── futures-core feature "alloc" (*)
│   │   │   │   │   │   │   ├── futures-io feature "std"
│   │   │   │   │   │   │   │   └── futures-io v0.3.32
│   │   │   │   │   │   │   └── memchr feature "default"
│   │   │   │   │   │   │       ├── memchr v2.8.0
│   │   │   │   │   │   │       └── memchr feature "std"
│   │   │   │   │   │   │           ├── memchr v2.8.0
│   │   │   │   │   │   │           └── memchr feature "alloc"
│   │   │   │   │   │   │               └── memchr v2.8.0
│   │   │   │   │   │   ├── futures-util feature "alloc"
│   │   │   │   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   ├── futures-util feature "slab"
│   │   │   │   │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   ├── futures-core feature "alloc" (*)
│   │   │   │   │   │   │   └── futures-task feature "alloc"
│   │   │   │   │   │   │       └── futures-task v0.3.32
│   │   │   │   │   │   ├── futures-util feature "slab" (*)
│   │   │   │   │   │   ├── futures-core feature "std" (*)
│   │   │   │   │   │   ├── futures-task feature "std"
│   │   │   │   │   │   │   ├── futures-task v0.3.32
│   │   │   │   │   │   │   └── futures-task feature "alloc" (*)
│   │   │   │   │   │   └── slab feature "std"
│   │   │   │   │   │       └── slab v0.4.12
│   │   │   │   │   ├── hickory-proto feature "std"
│   │   │   │   │   │   ├── hickory-proto v0.25.2
│   │   │   │   │   │   │   ├── futures-io v0.3.32
│   │   │   │   │   │   │   ├── ipnet v2.12.0
│   │   │   │   │   │   │   ├── thiserror v2.0.18 (*)
│   │   │   │   │   │   │   ├── tracing v0.1.44 (*)
│   │   │   │   │   │   │   ├── url v2.5.8 (*)
│   │   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   │   ├── tokio feature "io-util" (*)
│   │   │   │   │   │   │   ├── tokio feature "macros" (*)
│   │   │   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   │   │   ├── data-encoding feature "alloc" (*)
│   │   │   │   │   │   │   ├── cfg-if feature "default" (*)
│   │   │   │   │   │   │   ├── futures-util feature "alloc" (*)
│   │   │   │   │   │   │   ├── futures-channel feature "alloc" (*)
│   │   │   │   │   │   │   ├── async-trait feature "default"
│   │   │   │   │   │   │   │   └── async-trait v0.1.89 (proc-macro)
│   │   │   │   │   │   │   │       ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   │   │       ├── quote feature "default" (*)
│   │   │   │   │   │   │   │       ├── syn feature "clone-impls" (*)
│   │   │   │   │   │   │   │       ├── syn feature "full" (*)
│   │   │   │   │   │   │   │       ├── syn feature "parsing" (*)
│   │   │   │   │   │   │   │       ├── syn feature "printing" (*)
│   │   │   │   │   │   │   │       ├── syn feature "proc-macro" (*)
│   │   │   │   │   │   │   │       └── syn feature "visit-mut" (*)
│   │   │   │   │   │   │   ├── enum-as-inner feature "default"
│   │   │   │   │   │   │   │   └── enum-as-inner v0.6.1 (proc-macro)
│   │   │   │   │   │   │   │       ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   │   │       ├── quote feature "default" (*)
│   │   │   │   │   │   │   │       ├── syn feature "default" (*)
│   │   │   │   │   │   │   │       └── heck feature "default" (*)
│   │   │   │   │   │   │   ├── h2 feature "default"
│   │   │   │   │   │   │   │   └── h2 v0.4.13
│   │   │   │   │   │   │   │       ├── futures-core v0.3.32
│   │   │   │   │   │   │   │       ├── futures-sink v0.3.32
│   │   │   │   │   │   │   │       ├── tokio feature "default" (*)
│   │   │   │   │   │   │   │       ├── tokio feature "io-util" (*)
│   │   │   │   │   │   │   │       ├── bytes feature "default" (*)
│   │   │   │   │   │   │   │       ├── slab feature "default"
│   │   │   │   │   │   │   │       │   ├── slab v0.4.12
│   │   │   │   │   │   │   │       │   └── slab feature "std" (*)
│   │   │   │   │   │   │   │       ├── atomic-waker feature "default"
│   │   │   │   │   │   │   │       │   └── atomic-waker v1.1.2
│   │   │   │   │   │   │   │       ├── fnv feature "default"
│   │   │   │   │   │   │   │       │   ├── fnv v1.0.7
│   │   │   │   │   │   │   │       │   └── fnv feature "std"
│   │   │   │   │   │   │   │       │       └── fnv v1.0.7
│   │   │   │   │   │   │   │       ├── http feature "default"
│   │   │   │   │   │   │   │       │   ├── http v1.4.0
│   │   │   │   │   │   │   │       │   │   ├── bytes feature "default" (*)
│   │   │   │   │   │   │   │       │   │   └── itoa feature "default" (*)
│   │   │   │   │   │   │   │       │   └── http feature "std"
│   │   │   │   │   │   │   │       │       └── http v1.4.0 (*)
│   │   │   │   │   │   │   │       ├── indexmap feature "default"
│   │   │   │   │   │   │   │       │   ├── indexmap v2.13.0
│   │   │   │   │   │   │   │       │   │   ├── equivalent v1.0.2
│   │   │   │   │   │   │   │       │   │   └── hashbrown v0.16.1
│   │   │   │   │   │   │   │       │   │       ├── equivalent v1.0.2
│   │   │   │   │   │   │   │       │   │       ├── foldhash v0.2.0
│   │   │   │   │   │   │   │       │   │       └── allocator-api2 feature "alloc"
│   │   │   │   │   │   │   │       │   │           └── allocator-api2 v0.2.21
│   │   │   │   │   │   │   │       │   └── indexmap feature "std"
│   │   │   │   │   │   │   │       │       └── indexmap v2.13.0 (*)
│   │   │   │   │   │   │   │       ├── indexmap feature "std" (*)
│   │   │   │   │   │   │   │       ├── tokio-util feature "codec"
│   │   │   │   │   │   │   │       │   └── tokio-util v0.7.18
│   │   │   │   │   │   │   │       │       ├── tokio feature "default" (*)
│   │   │   │   │   │   │   │       │       ├── tokio feature "sync" (*)
│   │   │   │   │   │   │   │       │       ├── bytes feature "default" (*)
│   │   │   │   │   │   │   │       │       ├── pin-project-lite feature "default" (*)
│   │   │   │   │   │   │   │       │       ├── futures-util feature "default"
│   │   │   │   │   │   │   │       │       │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   │       │       │   ├── futures-util feature "async-await"
│   │   │   │   │   │   │   │       │       │   │   └── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   │       │       │   ├── futures-util feature "async-await-macro"
│   │   │   │   │   │   │   │       │       │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   │       │       │   │   ├── futures-util feature "async-await" (*)
│   │   │   │   │   │   │   │       │       │   │   └── futures-util feature "futures-macro"
│   │   │   │   │   │   │   │       │       │   │       └── futures-util v0.3.32 (*)
│   │   │   │   │   │   │   │       │       │   └── futures-util feature "std" (*)
│   │   │   │   │   │   │   │       │       ├── futures-core feature "default"
│   │   │   │   │   │   │   │       │       │   ├── futures-core v0.3.32
│   │   │   │   │   │   │   │       │       │   └── futures-core feature "std" (*)
│   │   │   │   │   │   │   │       │       └── futures-sink feature "default"
│   │   │   │   │   │   │   │       │           ├── futures-sink v0.3.32
│   │   │   │   │   │   │   │       │           └── futures-sink feature "std"
│   │   │   │   │   │   │   │       │               ├── futures-sink v0.3.32
│   │   │   │   │   │   │   │       │               └── futures-sink feature "alloc"
│   │   │   │   │   │   │   │       │                   └── futures-sink v0.3.32
│   │   │   │   │   │   │   │       ├── tokio-util feature "default"
│   │   │   │   │   │   │   │       │   └── tokio-util v0.7.18 (*)
│   │   │   │   │   │   │   │       ├── tokio-util feature "io"
│   │   │   │   │   │   │   │       │   └── tokio-util v0.7.18 (*)
│   │   │   │   │   │   │   │       └── tracing feature "std" (*)
│   │   │   │   │   │   │   ├── h2 feature "stream"
│   │   │   │   │   │   │   │   └── h2 v0.4.13 (*)
│   │   │   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   │   │   ├── once_cell feature "critical-section"
│   │   │   │   │   │   │   │   ├── once_cell v1.21.4 (*)
│   │   │   │   │   │   │   │   └── once_cell feature "portable-atomic"
│   │   │   │   │   │   │   │       └── once_cell v1.21.4 (*)
│   │   │   │   │   │   │   ├── idna feature "alloc" (*)
│   │   │   │   │   │   │   ├── idna feature "compiled_data" (*)
│   │   │   │   │   │   │   ├── rand feature "alloc"
│   │   │   │   │   │   │   │   └── rand v0.9.2
│   │   │   │   │   │   │   │       ├── rand_chacha v0.9.0
│   │   │   │   │   │   │   │       │   ├── rand_core feature "default" (*)
│   │   │   │   │   │   │   │       │   └── ppv-lite86 feature "simd"
│   │   │   │   │   │   │   │       │       └── ppv-lite86 v0.2.21
│   │   │   │   │   │   │   │       │           ├── zerocopy feature "default"
│   │   │   │   │   │   │   │       │           │   └── zerocopy v0.8.42
│   │   │   │   │   │   │   │       │           └── zerocopy feature "simd"
│   │   │   │   │   │   │   │       │               └── zerocopy v0.8.42
│   │   │   │   │   │   │   │       └── rand_core v0.9.5 (*)
│   │   │   │   │   │   │   ├── rand feature "std_rng"
│   │   │   │   │   │   │   │   └── rand v0.9.2 (*)
│   │   │   │   │   │   │   ├── rustls feature "logging"
│   │   │   │   │   │   │   │   ├── rustls v0.23.37 (*)
│   │   │   │   │   │   │   │   └── rustls feature "log"
│   │   │   │   │   │   │   │       └── rustls v0.23.37 (*)
│   │   │   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   │   │   ├── rustls feature "tls12"
│   │   │   │   │   │   │   │   └── rustls v0.23.37 (*)
│   │   │   │   │   │   │   ├── tinyvec feature "alloc"
│   │   │   │   │   │   │   │   ├── tinyvec v1.11.0
│   │   │   │   │   │   │   │   │   └── tinyvec_macros feature "default"
│   │   │   │   │   │   │   │   │       └── tinyvec_macros v0.1.1
│   │   │   │   │   │   │   │   └── tinyvec feature "tinyvec_macros"
│   │   │   │   │   │   │   │       └── tinyvec v1.11.0 (*)
│   │   │   │   │   │   │   ├── tinyvec feature "default"
│   │   │   │   │   │   │   │   └── tinyvec v1.11.0 (*)
│   │   │   │   │   │   │   └── tokio-rustls feature "early-data"
│   │   │   │   │   │   │       └── tokio-rustls v0.26.4 (*)
│   │   │   │   │   │   ├── thiserror feature "std"
│   │   │   │   │   │   │   └── thiserror v2.0.18 (*)
│   │   │   │   │   │   ├── data-encoding feature "std" (*)
│   │   │   │   │   │   ├── futures-util feature "std" (*)
│   │   │   │   │   │   ├── futures-channel feature "std" (*)
│   │   │   │   │   │   ├── futures-io feature "std" (*)
│   │   │   │   │   │   ├── hickory-proto feature "futures-io"
│   │   │   │   │   │   │   └── hickory-proto v0.25.2 (*)
│   │   │   │   │   │   ├── tracing feature "std" (*)
│   │   │   │   │   │   ├── ipnet feature "std"
│   │   │   │   │   │   │   └── ipnet v2.12.0
│   │   │   │   │   │   ├── rand feature "std"
│   │   │   │   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   │   │   │   ├── rand_core feature "std"
│   │   │   │   │   │   │   │   ├── rand_core v0.9.5 (*)
│   │   │   │   │   │   │   │   └── getrandom feature "std"
│   │   │   │   │   │   │   │       └── getrandom v0.3.4 (*)
│   │   │   │   │   │   │   ├── rand feature "alloc" (*)
│   │   │   │   │   │   │   └── rand_chacha feature "std"
│   │   │   │   │   │   │       ├── rand_chacha v0.9.0 (*)
│   │   │   │   │   │   │       ├── rand_core feature "std" (*)
│   │   │   │   │   │   │       └── ppv-lite86 feature "std"
│   │   │   │   │   │   │           └── ppv-lite86 v0.2.21 (*)
│   │   │   │   │   │   ├── rand feature "thread_rng"
│   │   │   │   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   │   │   │   ├── rand feature "os_rng"
│   │   │   │   │   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   │   │   │   │   └── rand_core feature "os_rng"
│   │   │   │   │   │   │   │       └── rand_core v0.9.5 (*)
│   │   │   │   │   │   │   ├── rand feature "std" (*)
│   │   │   │   │   │   │   └── rand feature "std_rng" (*)
│   │   │   │   │   │   └── url feature "std" (*)
│   │   │   │   │   ├── once_cell feature "critical-section" (*)
│   │   │   │   │   ├── smallvec feature "default" (*)
│   │   │   │   │   ├── rand feature "alloc" (*)
│   │   │   │   │   ├── rustls feature "logging" (*)
│   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   ├── rustls feature "tls12" (*)
│   │   │   │   │   ├── moka feature "default"
│   │   │   │   │   │   └── moka v0.12.14
│   │   │   │   │   │       ├── equivalent feature "default"
│   │   │   │   │   │       │   └── equivalent v1.0.2
│   │   │   │   │   │       ├── portable-atomic feature "default" (*)
│   │   │   │   │   │       ├── smallvec feature "default" (*)
│   │   │   │   │   │       ├── crossbeam-channel feature "default"
│   │   │   │   │   │       │   ├── crossbeam-channel v0.5.15
│   │   │   │   │   │       │   │   └── crossbeam-utils v0.8.21
│   │   │   │   │   │       │   └── crossbeam-channel feature "std"
│   │   │   │   │   │       │       ├── crossbeam-channel v0.5.15 (*)
│   │   │   │   │   │       │       └── crossbeam-utils feature "std"
│   │   │   │   │   │       │           └── crossbeam-utils v0.8.21
│   │   │   │   │   │       ├── crossbeam-utils feature "default"
│   │   │   │   │   │       │   ├── crossbeam-utils v0.8.21
│   │   │   │   │   │       │   └── crossbeam-utils feature "std" (*)
│   │   │   │   │   │       ├── crossbeam-epoch feature "default"
│   │   │   │   │   │       │   ├── crossbeam-epoch v0.9.18
│   │   │   │   │   │       │   │   └── crossbeam-utils v0.8.21
│   │   │   │   │   │       │   └── crossbeam-epoch feature "std"
│   │   │   │   │   │       │       ├── crossbeam-epoch v0.9.18 (*)
│   │   │   │   │   │       │       ├── crossbeam-utils feature "std" (*)
│   │   │   │   │   │       │       └── crossbeam-epoch feature "alloc"
│   │   │   │   │   │       │           └── crossbeam-epoch v0.9.18 (*)
│   │   │   │   │   │       ├── parking_lot feature "default"
│   │   │   │   │   │       │   └── parking_lot v0.12.5
│   │   │   │   │   │       │       ├── lock_api feature "default" (*)
│   │   │   │   │   │       │       └── parking_lot_core feature "default"
│   │   │   │   │   │       │           └── parking_lot_core v0.9.12
│   │   │   │   │   │       │               ├── libc feature "default" (*)
│   │   │   │   │   │       │               ├── cfg-if feature "default" (*)
│   │   │   │   │   │       │               └── smallvec feature "default" (*)
│   │   │   │   │   │       ├── tagptr feature "default"
│   │   │   │   │   │       │   └── tagptr v0.2.0
│   │   │   │   │   │       ├── uuid feature "default"
│   │   │   │   │   │       │   ├── uuid v1.22.0
│   │   │   │   │   │       │   │   └── getrandom feature "default"
│   │   │   │   │   │       │   │       └── getrandom v0.4.2
│   │   │   │   │   │       │   │           ├── libc v0.2.183
│   │   │   │   │   │       │   │           └── cfg-if feature "default" (*)
│   │   │   │   │   │       │   └── uuid feature "std"
│   │   │   │   │   │       │       └── uuid v1.22.0 (*)
│   │   │   │   │   │       └── uuid feature "v4"
│   │   │   │   │   │           ├── uuid v1.22.0 (*)
│   │   │   │   │   │           └── uuid feature "rng"
│   │   │   │   │   │               └── uuid v1.22.0 (*)
│   │   │   │   │   ├── moka feature "sync"
│   │   │   │   │   │   └── moka v0.12.14 (*)
│   │   │   │   │   ├── parking_lot feature "default" (*)
│   │   │   │   │   ├── resolv-conf feature "default"
│   │   │   │   │   │   └── resolv-conf v0.7.6
│   │   │   │   │   └── resolv-conf feature "system"
│   │   │   │   │       └── resolv-conf v0.7.6
│   │   │   │   ├── hickory-resolver feature "system-config"
│   │   │   │   │   └── hickory-resolver v0.25.2 (*)
│   │   │   │   └── hickory-resolver feature "tokio"
│   │   │   │       ├── hickory-resolver v0.25.2 (*)
│   │   │   │       ├── tokio feature "rt" (*)
│   │   │   │       ├── hickory-resolver feature "tokio" (*)
│   │   │   │       └── hickory-proto feature "tokio"
│   │   │   │           ├── hickory-proto v0.25.2 (*)
│   │   │   │           ├── tokio feature "net" (*)
│   │   │   │           ├── tokio feature "rt" (*)
│   │   │   │           ├── tokio feature "rt-multi-thread"
│   │   │   │           │   ├── tokio v1.50.0 (*)
│   │   │   │           │   └── tokio feature "rt" (*)
│   │   │   │           ├── tokio feature "time"
│   │   │   │           │   └── tokio v1.50.0 (*)
│   │   │   │           ├── hickory-proto feature "std" (*)
│   │   │   │           └── hickory-proto feature "tokio" (*)
│   │   │   ├── hickory-resolver feature "https-ring"
│   │   │   │   ├── hickory-resolver v0.25.2 (*)
│   │   │   │   ├── hickory-resolver feature "__https"
│   │   │   │   │   ├── hickory-resolver v0.25.2 (*)
│   │   │   │   │   └── hickory-resolver feature "__tls"
│   │   │   │   │       ├── hickory-resolver v0.25.2 (*)
│   │   │   │   │       └── hickory-resolver feature "tokio" (*)
│   │   │   │   └── hickory-proto feature "https-ring"
│   │   │   │       ├── hickory-proto v0.25.2 (*)
│   │   │   │       ├── hickory-proto feature "__https"
│   │   │   │       │   ├── hickory-proto v0.25.2 (*)
│   │   │   │       │   └── hickory-proto feature "std" (*)
│   │   │   │       └── hickory-proto feature "tls-ring"
│   │   │   │           ├── hickory-proto v0.25.2 (*)
│   │   │   │           ├── hickory-proto feature "__tls"
│   │   │   │           │   ├── hickory-proto v0.25.2 (*)
│   │   │   │           │   ├── hickory-proto feature "std" (*)
│   │   │   │           │   └── hickory-proto feature "tokio" (*)
│   │   │   │           ├── hickory-proto feature "tokio-rustls"
│   │   │   │           │   └── hickory-proto v0.25.2 (*)
│   │   │   │           └── tokio-rustls feature "ring"
│   │   │   │               ├── tokio-rustls v0.26.4 (*)
│   │   │   │               └── rustls feature "ring"
│   │   │   │                   ├── rustls v0.23.37 (*)
│   │   │   │                   └── rustls-webpki feature "ring"
│   │   │   │                       └── rustls-webpki v0.103.9 (*)
│   │   │   ├── hickory-resolver feature "tokio" (*)
│   │   │   ├── http feature "default" (*)
│   │   │   ├── tokio-util feature "codec" (*)
│   │   │   ├── tokio-util feature "default" (*)
│   │   │   ├── tokio-util feature "io" (*)
│   │   │   ├── tokio-util feature "io-util"
│   │   │   │   ├── tokio-util v0.7.18 (*)
│   │   │   │   ├── tokio feature "io-util" (*)
│   │   │   │   ├── tokio feature "rt" (*)
│   │   │   │   └── tokio-util feature "io" (*)
│   │   │   ├── tokio-util feature "rt"
│   │   │   │   ├── tokio-util v0.7.18 (*)
│   │   │   │   ├── tokio feature "rt" (*)
│   │   │   │   ├── tokio feature "sync" (*)
│   │   │   │   └── tokio-util feature "futures-util"
│   │   │   │       └── tokio-util v0.7.18 (*)
│   │   │   ├── tracing feature "default" (*)
│   │   │   ├── rand feature "default"
│   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   ├── rand feature "os_rng" (*)
│   │   │   │   ├── rand feature "small_rng"
│   │   │   │   │   └── rand v0.9.2 (*)
│   │   │   │   ├── rand feature "std" (*)
│   │   │   │   ├── rand feature "std_rng" (*)
│   │   │   │   └── rand feature "thread_rng" (*)
│   │   │   ├── rustls feature "ring" (*)
│   │   │   ├── rustls-pki-types feature "default" (*)
│   │   │   ├── tokio-rustls feature "logging"
│   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   └── rustls feature "logging" (*)
│   │   │   ├── tokio-rustls feature "ring" (*)
│   │   │   ├── url feature "default" (*)
│   │   │   ├── url feature "serde" (*)
│   │   │   ├── iroh-base feature "key" (*)
│   │   │   ├── iroh-base feature "relay" (*)
│   │   │   ├── n0-error feature "default" (*)
│   │   │   ├── postcard feature "alloc" (*)
│   │   │   ├── postcard feature "experimental-derive"
│   │   │   │   ├── postcard v1.1.3 (*)
│   │   │   │   └── postcard feature "postcard-derive"
│   │   │   │       └── postcard v1.1.3 (*)
│   │   │   ├── postcard feature "use-std" (*)
│   │   │   ├── blake3 feature "default"
│   │   │   │   ├── blake3 v1.8.3
│   │   │   │   │   ├── arrayvec v0.7.6
│   │   │   │   │   ├── constant_time_eq v0.4.2
│   │   │   │   │   ├── cfg-if feature "default" (*)
│   │   │   │   │   ├── cpufeatures feature "default" (*)
│   │   │   │   │   └── arrayref feature "default"
│   │   │   │   │       └── arrayref v0.3.9
│   │   │   │   │   [build-dependencies]
│   │   │   │   │   └── cc feature "default"
│   │   │   │   │       └── cc v1.2.57 (*)
│   │   │   │   └── blake3 feature "std"
│   │   │   │       ├── blake3 v1.8.3 (*)
│   │   │   │       └── constant_time_eq feature "std"
│   │   │   │           └── constant_time_eq v0.4.2
│   │   │   ├── http-body-util feature "default"
│   │   │   │   └── http-body-util v0.1.3
│   │   │   │       ├── futures-core v0.3.32
│   │   │   │       ├── bytes feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── http feature "default" (*)
│   │   │   │       └── http-body feature "default"
│   │   │   │           └── http-body v1.0.1
│   │   │   │               ├── bytes feature "default" (*)
│   │   │   │               └── http feature "default" (*)
│   │   │   ├── hyper feature "client"
│   │   │   │   └── hyper v1.8.1
│   │   │   │       ├── tokio feature "default" (*)
│   │   │   │       ├── tokio feature "sync" (*)
│   │   │   │       ├── bytes feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── futures-channel feature "default"
│   │   │   │       │   ├── futures-channel v0.3.32 (*)
│   │   │   │       │   └── futures-channel feature "std" (*)
│   │   │   │       ├── futures-core feature "default" (*)
│   │   │   │       ├── h2 feature "default" (*)
│   │   │   │       ├── atomic-waker feature "default" (*)
│   │   │   │       ├── http feature "default" (*)
│   │   │   │       ├── itoa feature "default" (*)
│   │   │   │       ├── smallvec feature "const_generics" (*)
│   │   │   │       ├── smallvec feature "const_new"
│   │   │   │       │   ├── smallvec v1.15.1
│   │   │   │       │   └── smallvec feature "const_generics" (*)
│   │   │   │       ├── smallvec feature "default" (*)
│   │   │   │       ├── http-body feature "default" (*)
│   │   │   │       ├── httparse feature "default"
│   │   │   │       │   ├── httparse v1.10.1
│   │   │   │       │   └── httparse feature "std"
│   │   │   │       │       └── httparse v1.10.1
│   │   │   │       ├── httpdate feature "default"
│   │   │   │       │   └── httpdate v1.0.3
│   │   │   │       ├── pin-utils feature "default"
│   │   │   │       │   └── pin-utils v0.1.0
│   │   │   │       └── want feature "default"
│   │   │   │           └── want v0.3.1
│   │   │   │               └── try-lock feature "default"
│   │   │   │                   └── try-lock v0.2.5
│   │   │   ├── hyper feature "default"
│   │   │   │   └── hyper v1.8.1 (*)
│   │   │   ├── hyper feature "http1"
│   │   │   │   └── hyper v1.8.1 (*)
│   │   │   ├── hyper feature "server"
│   │   │   │   └── hyper v1.8.1 (*)
│   │   │   ├── hyper-util feature "default"
│   │   │   │   └── hyper-util v0.1.20
│   │   │   │       ├── futures-util v0.3.32 (*)
│   │   │   │       ├── tokio v1.50.0 (*)
│   │   │   │       ├── bytes feature "default" (*)
│   │   │   │       ├── libc feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── socket2 feature "all" (*)
│   │   │   │       ├── socket2 feature "default" (*)
│   │   │   │       ├── futures-channel feature "default" (*)
│   │   │   │       ├── http feature "default" (*)
│   │   │   │       ├── tracing feature "std" (*)
│   │   │   │       ├── ipnet feature "default"
│   │   │   │       │   ├── ipnet v2.12.0
│   │   │   │       │   └── ipnet feature "std" (*)
│   │   │   │       ├── percent-encoding feature "default"
│   │   │   │       │   ├── percent-encoding v2.3.2
│   │   │   │       │   └── percent-encoding feature "std" (*)
│   │   │   │       ├── http-body feature "default" (*)
│   │   │   │       ├── hyper feature "default" (*)
│   │   │   │       ├── base64 feature "default"
│   │   │   │       │   ├── base64 v0.22.1
│   │   │   │       │   └── base64 feature "std"
│   │   │   │       │       ├── base64 v0.22.1
│   │   │   │       │       └── base64 feature "alloc"
│   │   │   │       │           └── base64 v0.22.1
│   │   │   │       └── tower-service feature "default"
│   │   │   │           └── tower-service v0.3.3
│   │   │   ├── lru feature "default"
│   │   │   │   ├── lru v0.16.3
│   │   │   │   │   └── hashbrown feature "default"
│   │   │   │   │       ├── hashbrown v0.16.1 (*)
│   │   │   │   │       ├── hashbrown feature "allocator-api2"
│   │   │   │   │       │   └── hashbrown v0.16.1 (*)
│   │   │   │   │       ├── hashbrown feature "default-hasher"
│   │   │   │   │       │   └── hashbrown v0.16.1 (*)
│   │   │   │   │       ├── hashbrown feature "equivalent"
│   │   │   │   │       │   └── hashbrown v0.16.1 (*)
│   │   │   │   │       ├── hashbrown feature "inline-more"
│   │   │   │   │       │   └── hashbrown v0.16.1 (*)
│   │   │   │   │       └── hashbrown feature "raw-entry"
│   │   │   │   │           └── hashbrown v0.16.1 (*)
│   │   │   │   └── lru feature "hashbrown"
│   │   │   │       └── lru v0.16.3 (*)
│   │   │   ├── n0-future feature "default"
│   │   │   │   └── n0-future v0.3.2
│   │   │   │       ├── tokio feature "default" (*)
│   │   │   │       ├── tokio feature "macros" (*)
│   │   │   │       ├── tokio feature "rt" (*)
│   │   │   │       ├── tokio feature "test-util"
│   │   │   │       │   ├── tokio v1.50.0 (*)
│   │   │   │       │   ├── tokio feature "rt" (*)
│   │   │   │       │   ├── tokio feature "sync" (*)
│   │   │   │       │   └── tokio feature "time" (*)
│   │   │   │       ├── tokio feature "time" (*)
│   │   │   │       ├── derive_more feature "debug" (*)
│   │   │   │       ├── derive_more feature "default" (*)
│   │   │   │       ├── derive_more feature "deref" (*)
│   │   │   │       ├── derive_more feature "display" (*)
│   │   │   │       ├── futures-util feature "default" (*)
│   │   │   │       ├── futures-util feature "sink"
│   │   │   │       │   ├── futures-util v0.3.32 (*)
│   │   │   │       │   └── futures-util feature "futures-sink"
│   │   │   │       │       └── futures-util v0.3.32 (*)
│   │   │   │       ├── tokio-util feature "default" (*)
│   │   │   │       ├── tokio-util feature "rt" (*)
│   │   │   │       ├── futures-buffered feature "default"
│   │   │   │       │   └── futures-buffered v0.2.13
│   │   │   │       │       ├── futures-core v0.3.32
│   │   │   │       │       ├── pin-project-lite feature "default" (*)
│   │   │   │       │       ├── cordyceps feature "default"
│   │   │   │       │       │   └── cordyceps v0.3.4
│   │   │   │       │       ├── diatomic-waker feature "default"
│   │   │   │       │       │   ├── diatomic-waker v0.2.3
│   │   │   │       │       │   └── diatomic-waker feature "alloc"
│   │   │   │       │       │       └── diatomic-waker v0.2.3
│   │   │   │       │       └── spin feature "spin_mutex"
│   │   │   │       │           ├── spin v0.10.0
│   │   │   │       │           └── spin feature "mutex"
│   │   │   │       │               └── spin v0.10.0
│   │   │   │       ├── futures-lite feature "default"
│   │   │   │       │   ├── futures-lite v2.6.1
│   │   │   │       │   │   ├── fastrand v2.3.0
│   │   │   │       │   │   ├── futures-core v0.3.32
│   │   │   │       │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │       │   │   ├── futures-io feature "default"
│   │   │   │       │   │   │   ├── futures-io v0.3.32
│   │   │   │       │   │   │   └── futures-io feature "std" (*)
│   │   │   │       │   │   └── parking feature "default"
│   │   │   │       │   │       └── parking v2.2.1
│   │   │   │       │   ├── futures-lite feature "race"
│   │   │   │       │   │   ├── futures-lite v2.6.1 (*)
│   │   │   │       │   │   └── futures-lite feature "fastrand"
│   │   │   │       │   │       └── futures-lite v2.6.1 (*)
│   │   │   │       │   └── futures-lite feature "std"
│   │   │   │       │       ├── futures-lite v2.6.1 (*)
│   │   │   │       │       ├── fastrand feature "std"
│   │   │   │       │       │   ├── fastrand v2.3.0
│   │   │   │       │       │   └── fastrand feature "alloc"
│   │   │   │       │       │       └── fastrand v2.3.0
│   │   │   │       │       ├── futures-lite feature "alloc"
│   │   │   │       │       │   └── futures-lite v2.6.1 (*)
│   │   │   │       │       ├── futures-lite feature "fastrand" (*)
│   │   │   │       │       ├── futures-lite feature "futures-io"
│   │   │   │       │       │   └── futures-lite v2.6.1 (*)
│   │   │   │       │       └── futures-lite feature "parking"
│   │   │   │       │           └── futures-lite v2.6.1 (*)
│   │   │   │       └── pin-project feature "default"
│   │   │   │           └── pin-project v1.1.11
│   │   │   │               └── pin-project-internal feature "default"
│   │   │   │                   └── pin-project-internal v1.1.11 (proc-macro)
│   │   │   │                       ├── proc-macro2 feature "default" (*)
│   │   │   │                       ├── quote feature "default" (*)
│   │   │   │                       ├── syn feature "clone-impls" (*)
│   │   │   │                       ├── syn feature "full" (*)
│   │   │   │                       ├── syn feature "parsing" (*)
│   │   │   │                       ├── syn feature "printing" (*)
│   │   │   │                       ├── syn feature "proc-macro" (*)
│   │   │   │                       └── syn feature "visit-mut" (*)
│   │   │   │       [build-dependencies]
│   │   │   │       └── cfg_aliases feature "default"
│   │   │   │           └── cfg_aliases v0.2.1
│   │   │   ├── pin-project feature "default" (*)
│   │   │   ├── noq feature "rustls-ring"
│   │   │   │   ├── noq v0.17.0
│   │   │   │   │   ├── noq-proto v0.16.0
│   │   │   │   │   │   ├── thiserror feature "default"
│   │   │   │   │   │   │   ├── thiserror v2.0.18 (*)
│   │   │   │   │   │   │   └── thiserror feature "std" (*)
│   │   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   │   ├── derive_more feature "debug" (*)
│   │   │   │   │   │   ├── derive_more feature "default" (*)
│   │   │   │   │   │   ├── derive_more feature "deref" (*)
│   │   │   │   │   │   ├── derive_more feature "deref_mut"
│   │   │   │   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   │   │   │   └── derive_more-impl feature "deref_mut"
│   │   │   │   │   │   │       └── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │   │   │   │   ├── derive_more feature "display" (*)
│   │   │   │   │   │   ├── derive_more feature "from" (*)
│   │   │   │   │   │   ├── slab feature "default" (*)
│   │   │   │   │   │   ├── tracing feature "std" (*)
│   │   │   │   │   │   ├── rand feature "default" (*)
│   │   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   │   ├── ring feature "default" (*)
│   │   │   │   │   │   ├── tinyvec feature "alloc" (*)
│   │   │   │   │   │   ├── tinyvec feature "default" (*)
│   │   │   │   │   │   ├── aes-gcm feature "aes"
│   │   │   │   │   │   │   └── aes-gcm v0.10.3
│   │   │   │   │   │   │       ├── aead v0.5.2
│   │   │   │   │   │   │       │   ├── generic-array v0.14.7
│   │   │   │   │   │   │       │   │   └── typenum feature "default" (*)
│   │   │   │   │   │   │       │   │   [build-dependencies]
│   │   │   │   │   │   │       │   │   └── version_check feature "default"
│   │   │   │   │   │   │       │   │       └── version_check v0.9.5
│   │   │   │   │   │   │       │   └── crypto-common feature "default"
│   │   │   │   │   │   │       │       └── crypto-common v0.1.7
│   │   │   │   │   │   │       │           ├── typenum feature "default" (*)
│   │   │   │   │   │   │       │           ├── generic-array feature "default"
│   │   │   │   │   │   │       │           │   └── generic-array v0.14.7 (*)
│   │   │   │   │   │   │       │           └── generic-array feature "more_lengths"
│   │   │   │   │   │   │       │               └── generic-array v0.14.7 (*)
│   │   │   │   │   │   │       ├── ghash v0.5.1
│   │   │   │   │   │   │       │   ├── opaque-debug feature "default"
│   │   │   │   │   │   │       │   │   └── opaque-debug v0.3.1
│   │   │   │   │   │   │       │   └── polyval feature "default"
│   │   │   │   │   │   │       │       └── polyval v0.6.2
│   │   │   │   │   │   │       │           ├── universal-hash v0.5.1
│   │   │   │   │   │   │       │           │   ├── subtle v2.6.1
│   │   │   │   │   │   │       │           │   └── crypto-common feature "default" (*)
│   │   │   │   │   │   │       │           ├── cfg-if feature "default" (*)
│   │   │   │   │   │   │       │           ├── cpufeatures feature "default" (*)
│   │   │   │   │   │   │       │           └── opaque-debug feature "default" (*)
│   │   │   │   │   │   │       ├── subtle v2.6.1
│   │   │   │   │   │   │       ├── aes feature "default"
│   │   │   │   │   │   │       │   └── aes v0.8.4
│   │   │   │   │   │   │       │       ├── cfg-if feature "default" (*)
│   │   │   │   │   │   │       │       ├── cpufeatures feature "default" (*)
│   │   │   │   │   │   │       │       └── cipher feature "default"
│   │   │   │   │   │   │       │           └── cipher v0.4.4
│   │   │   │   │   │   │       │               ├── crypto-common feature "default" (*)
│   │   │   │   │   │   │       │               └── inout feature "default"
│   │   │   │   │   │   │       │                   └── inout v0.1.4
│   │   │   │   │   │   │       │                       └── generic-array feature "default" (*)
│   │   │   │   │   │   │       ├── cipher feature "default" (*)
│   │   │   │   │   │   │       └── ctr feature "default"
│   │   │   │   │   │   │           └── ctr v0.9.2
│   │   │   │   │   │   │               └── cipher feature "default" (*)
│   │   │   │   │   │   ├── enum-assoc feature "default"
│   │   │   │   │   │   │   └── enum-assoc v1.3.0 (proc-macro)
│   │   │   │   │   │   │       ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   │       ├── quote feature "default" (*)
│   │   │   │   │   │   │       ├── syn feature "default" (*)
│   │   │   │   │   │   │       └── syn feature "full" (*)
│   │   │   │   │   │   ├── fastbloom feature "default"
│   │   │   │   │   │   │   ├── fastbloom v0.14.1
│   │   │   │   │   │   │   │   ├── getrandom feature "default" (*)
│   │   │   │   │   │   │   │   ├── rand feature "default" (*)
│   │   │   │   │   │   │   │   ├── libm feature "default"
│   │   │   │   │   │   │   │   │   ├── libm v0.2.16
│   │   │   │   │   │   │   │   │   └── libm feature "arch"
│   │   │   │   │   │   │   │   │       └── libm v0.2.16
│   │   │   │   │   │   │   │   └── siphasher feature "default"
│   │   │   │   │   │   │   │       ├── siphasher v1.0.2
│   │   │   │   │   │   │   │       └── siphasher feature "std"
│   │   │   │   │   │   │   │           └── siphasher v1.0.2
│   │   │   │   │   │   │   ├── fastbloom feature "rand"
│   │   │   │   │   │   │   │   └── fastbloom v0.14.1 (*)
│   │   │   │   │   │   │   └── fastbloom feature "std"
│   │   │   │   │   │   │       └── fastbloom v0.14.1 (*)
│   │   │   │   │   │   ├── identity-hash feature "default"
│   │   │   │   │   │   │   ├── identity-hash v0.1.0
│   │   │   │   │   │   │   └── identity-hash feature "std"
│   │   │   │   │   │   │       └── identity-hash v0.1.0
│   │   │   │   │   │   ├── lru-slab feature "default"
│   │   │   │   │   │   │   └── lru-slab v0.1.2
│   │   │   │   │   │   ├── rustc-hash feature "default"
│   │   │   │   │   │   │   ├── rustc-hash v2.1.1
│   │   │   │   │   │   │   └── rustc-hash feature "std"
│   │   │   │   │   │   │       └── rustc-hash v2.1.1
│   │   │   │   │   │   └── sorted-index-buffer feature "default"
│   │   │   │   │   │       └── sorted-index-buffer v0.2.1
│   │   │   │   │   ├── thiserror feature "default" (*)
│   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   ├── tokio feature "sync" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   ├── socket2 feature "default" (*)
│   │   │   │   │   ├── tracing feature "std" (*)
│   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   ├── rustc-hash feature "default" (*)
│   │   │   │   │   ├── noq-udp feature "tracing"
│   │   │   │   │   │   └── noq-udp v0.9.0
│   │   │   │   │   │       ├── libc feature "default" (*)
│   │   │   │   │   │       ├── socket2 feature "default" (*)
│   │   │   │   │   │       └── tracing feature "std" (*)
│   │   │   │   │   │       [build-dependencies]
│   │   │   │   │   │       └── cfg_aliases feature "default" (*)
│   │   │   │   │   ├── tokio-stream feature "default"
│   │   │   │   │   │   ├── tokio-stream v0.1.18
│   │   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   │   ├── tokio feature "sync" (*)
│   │   │   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   │   │   ├── futures-core feature "default" (*)
│   │   │   │   │   │   │   └── tokio-util feature "default" (*)
│   │   │   │   │   │   └── tokio-stream feature "time"
│   │   │   │   │   │       ├── tokio-stream v0.1.18 (*)
│   │   │   │   │   │       └── tokio feature "time" (*)
│   │   │   │   │   └── tokio-stream feature "sync"
│   │   │   │   │       ├── tokio-stream v0.1.18 (*)
│   │   │   │   │       ├── tokio feature "sync" (*)
│   │   │   │   │       └── tokio-stream feature "tokio-util"
│   │   │   │   │           └── tokio-stream v0.1.18 (*)
│   │   │   │   │   [build-dependencies]
│   │   │   │   │   └── cfg_aliases feature "default" (*)
│   │   │   │   ├── noq feature "ring"
│   │   │   │   │   ├── noq v0.17.0 (*)
│   │   │   │   │   └── noq-proto feature "ring"
│   │   │   │   │       ├── noq-proto v0.16.0 (*)
│   │   │   │   │       └── rustls feature "ring" (*)
│   │   │   │   └── noq feature "rustls"
│   │   │   │       ├── noq v0.17.0 (*)
│   │   │   │       └── noq-proto feature "rustls"
│   │   │   │           └── noq-proto v0.16.0 (*)
│   │   │   ├── noq-proto feature "default"
│   │   │   │   ├── noq-proto v0.16.0 (*)
│   │   │   │   ├── noq-proto feature "bloom"
│   │   │   │   │   └── noq-proto v0.16.0 (*)
│   │   │   │   ├── noq-proto feature "ring" (*)
│   │   │   │   ├── noq-proto feature "rustls" (*)
│   │   │   │   └── noq-proto feature "tracing-log"
│   │   │   │       ├── noq-proto v0.16.0 (*)
│   │   │   │       └── tracing feature "log"
│   │   │   │           └── tracing v0.1.44 (*)
│   │   │   ├── num_enum feature "default"
│   │   │   │   ├── num_enum v0.7.6
│   │   │   │   │   ├── num_enum_derive v0.7.6 (proc-macro)
│   │   │   │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │   │   │   │   ├── quote feature "default" (*)
│   │   │   │   │   │   ├── syn feature "default" (*)
│   │   │   │   │   │   ├── syn feature "derive" (*)
│   │   │   │   │   │   ├── syn feature "extra-traits" (*)
│   │   │   │   │   │   ├── syn feature "parsing" (*)
│   │   │   │   │   │   └── proc-macro-crate feature "default"
│   │   │   │   │   │       └── proc-macro-crate v3.5.0
│   │   │   │   │   │           └── toml_edit feature "parse"
│   │   │   │   │   │               └── toml_edit v0.25.5+spec-1.1.0
│   │   │   │   │   │                   ├── indexmap feature "default" (*)
│   │   │   │   │   │                   ├── indexmap feature "std" (*)
│   │   │   │   │   │                   ├── toml_datetime feature "default"
│   │   │   │   │   │                   │   ├── toml_datetime v1.0.1+spec-1.1.0
│   │   │   │   │   │                   │   └── toml_datetime feature "std"
│   │   │   │   │   │                   │       ├── toml_datetime v1.0.1+spec-1.1.0
│   │   │   │   │   │                   │       └── toml_datetime feature "alloc"
│   │   │   │   │   │                   │           └── toml_datetime v1.0.1+spec-1.1.0
│   │   │   │   │   │                   ├── toml_parser feature "default"
│   │   │   │   │   │                   │   ├── toml_parser v1.0.10+spec-1.1.0
│   │   │   │   │   │                   │   │   └── winnow v1.0.0
│   │   │   │   │   │                   │   └── toml_parser feature "std"
│   │   │   │   │   │                   │       ├── toml_parser v1.0.10+spec-1.1.0 (*)
│   │   │   │   │   │                   │       └── toml_parser feature "alloc"
│   │   │   │   │   │                   │           └── toml_parser v1.0.10+spec-1.1.0 (*)
│   │   │   │   │   │                   └── winnow feature "default"
│   │   │   │   │   │                       ├── winnow v1.0.0
│   │   │   │   │   │                       ├── winnow feature "ascii"
│   │   │   │   │   │                       │   ├── winnow v1.0.0
│   │   │   │   │   │                       │   └── winnow feature "parser"
│   │   │   │   │   │                       │       └── winnow v1.0.0
│   │   │   │   │   │                       ├── winnow feature "binary"
│   │   │   │   │   │                       │   ├── winnow v1.0.0
│   │   │   │   │   │                       │   └── winnow feature "parser" (*)
│   │   │   │   │   │                       └── winnow feature "std"
│   │   │   │   │   │                           ├── winnow v1.0.0
│   │   │   │   │   │                           └── winnow feature "alloc"
│   │   │   │   │   │                               └── winnow v1.0.0
│   │   │   │   │   └── rustversion feature "default"
│   │   │   │   │       └── rustversion v1.0.22 (proc-macro)
│   │   │   │   └── num_enum feature "std"
│   │   │   │       ├── num_enum v0.7.6 (*)
│   │   │   │       └── num_enum_derive feature "std"
│   │   │   │           ├── num_enum_derive v0.7.6 (proc-macro) (*)
│   │   │   │           └── num_enum_derive feature "proc-macro-crate"
│   │   │   │               └── num_enum_derive v0.7.6 (proc-macro) (*)
│   │   │   ├── pkarr feature "signed_packet"
│   │   │   │   ├── pkarr v5.0.2
│   │   │   │   │   ├── getrandom v0.3.4 (*)
│   │   │   │   │   ├── serde feature "default" (*)
│   │   │   │   │   ├── serde feature "derive" (*)
│   │   │   │   │   ├── thiserror feature "default" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── ed25519-dalek feature "alloc"
│   │   │   │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│   │   │   │   │   │   ├── serde feature "alloc" (*)
│   │   │   │   │   │   ├── ed25519-dalek feature "signature"
│   │   │   │   │   │   │   └── ed25519-dalek v3.0.0-pre.1 (*)
│   │   │   │   │   │   ├── curve25519-dalek feature "alloc" (*)
│   │   │   │   │   │   ├── zeroize feature "alloc" (*)
│   │   │   │   │   │   ├── ed25519 feature "alloc"
│   │   │   │   │   │   │   ├── ed25519 v3.0.0-rc.4 (*)
│   │   │   │   │   │   │   └── pkcs8 feature "alloc"
│   │   │   │   │   │   │       ├── pkcs8 v0.11.0-rc.11 (*)
│   │   │   │   │   │   │       ├── der feature "alloc"
│   │   │   │   │   │   │       │   ├── der v0.8.0 (*)
│   │   │   │   │   │   │       │   └── zeroize feature "alloc" (*)
│   │   │   │   │   │   │       ├── der feature "zeroize"
│   │   │   │   │   │   │       │   └── der v0.8.0 (*)
│   │   │   │   │   │   │       └── spki feature "alloc"
│   │   │   │   │   │   │           ├── spki v0.8.0-rc.4 (*)
│   │   │   │   │   │   │           └── der feature "alloc" (*)
│   │   │   │   │   │   └── signature feature "alloc"
│   │   │   │   │   │       └── signature v3.0.0-rc.10
│   │   │   │   │   ├── ed25519-dalek feature "default" (*)
│   │   │   │   │   ├── base32 feature "default"
│   │   │   │   │   │   └── base32 v0.5.1
│   │   │   │   │   ├── document-features feature "default"
│   │   │   │   │   │   └── document-features v0.2.12 (proc-macro)
│   │   │   │   │   │       └── litrs feature "default"
│   │   │   │   │   │           └── litrs v1.0.0
│   │   │   │   │   ├── ntimestamp feature "default"
│   │   │   │   │   │   └── ntimestamp v1.0.0
│   │   │   │   │   │       ├── getrandom v0.2.17 (*)
│   │   │   │   │   │       ├── serde feature "derive" (*)
│   │   │   │   │   │       ├── once_cell feature "default" (*)
│   │   │   │   │   │       ├── httpdate feature "default" (*)
│   │   │   │   │   │       ├── base32 feature "default" (*)
│   │   │   │   │   │       └── document-features feature "default" (*)
│   │   │   │   │   ├── ntimestamp feature "full"
│   │   │   │   │   │   ├── ntimestamp v1.0.0 (*)
│   │   │   │   │   │   ├── ntimestamp feature "base32"
│   │   │   │   │   │   │   └── ntimestamp v1.0.0 (*)
│   │   │   │   │   │   ├── ntimestamp feature "httpdate"
│   │   │   │   │   │   │   └── ntimestamp v1.0.0 (*)
│   │   │   │   │   │   └── ntimestamp feature "serde"
│   │   │   │   │   │       └── ntimestamp v1.0.0 (*)
│   │   │   │   │   ├── self_cell feature "default"
│   │   │   │   │   │   └── self_cell v1.2.2
│   │   │   │   │   └── simple-dns feature "default"
│   │   │   │   │       └── simple-dns v0.9.3
│   │   │   │   │           └── bitflags feature "default"
│   │   │   │   │               └── bitflags v2.11.0
│   │   │   │   │   [build-dependencies]
│   │   │   │   │   └── cfg_aliases feature "default" (*)
│   │   │   │   └── pkarr feature "keys"
│   │   │   │       └── pkarr v5.0.2 (*)
│   │   │   ├── reqwest feature "rustls-tls"
│   │   │   │   ├── reqwest v0.12.28
│   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   ├── serde feature "default" (*)
│   │   │   │   │   ├── tokio feature "net" (*)
│   │   │   │   │   ├── tokio feature "time" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   ├── tokio-util feature "io" (*)
│   │   │   │   │   ├── log feature "default" (*)
│   │   │   │   │   ├── rustls feature "std" (*)
│   │   │   │   │   ├── rustls feature "tls12" (*)
│   │   │   │   │   ├── rustls-pki-types feature "default" (*)
│   │   │   │   │   ├── rustls-pki-types feature "std" (*)
│   │   │   │   │   ├── tokio-rustls feature "tls12"
│   │   │   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   │   │   └── rustls feature "tls12" (*)
│   │   │   │   │   ├── url feature "default" (*)
│   │   │   │   │   ├── percent-encoding feature "default" (*)
│   │   │   │   │   ├── http-body-util feature "default" (*)
│   │   │   │   │   ├── http-body feature "default" (*)
│   │   │   │   │   ├── hyper feature "client" (*)
│   │   │   │   │   ├── hyper feature "default" (*)
│   │   │   │   │   ├── hyper feature "http1" (*)
│   │   │   │   │   ├── hyper-util feature "client"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   ├── tokio feature "net" (*)
│   │   │   │   │   │   ├── hyper feature "client" (*)
│   │   │   │   │   │   └── hyper-util feature "tokio"
│   │   │   │   │   │       ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │       ├── tokio feature "rt" (*)
│   │   │   │   │   │       ├── tokio feature "time" (*)
│   │   │   │   │   │       └── hyper-util feature "tokio" (*)
│   │   │   │   │   ├── hyper-util feature "client-legacy"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   ├── tokio feature "sync" (*)
│   │   │   │   │   │   ├── hyper-util feature "client" (*)
│   │   │   │   │   │   └── hyper-util feature "tokio" (*)
│   │   │   │   │   ├── hyper-util feature "client-proxy"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   └── hyper-util feature "client" (*)
│   │   │   │   │   ├── hyper-util feature "default" (*)
│   │   │   │   │   ├── hyper-util feature "http1"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   └── hyper feature "http1" (*)
│   │   │   │   │   ├── hyper-util feature "tokio" (*)
│   │   │   │   │   ├── base64 feature "default" (*)
│   │   │   │   │   ├── tower-service feature "default" (*)
│   │   │   │   │   ├── hyper-rustls feature "http1"
│   │   │   │   │   │   ├── hyper-rustls v0.27.7
│   │   │   │   │   │   │   ├── hyper v1.8.1 (*)
│   │   │   │   │   │   │   ├── rustls v0.23.37 (*)
│   │   │   │   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   │   │   ├── rustls-pki-types feature "default" (*)
│   │   │   │   │   │   │   ├── hyper-util feature "client-legacy" (*)
│   │   │   │   │   │   │   ├── hyper-util feature "tokio" (*)
│   │   │   │   │   │   │   ├── tower-service feature "default" (*)
│   │   │   │   │   │   │   └── webpki-roots feature "default"
│   │   │   │   │   │   │       └── webpki-roots v1.0.6
│   │   │   │   │   │   │           └── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   │   └── hyper-util feature "http1" (*)
│   │   │   │   │   ├── hyper-rustls feature "tls12"
│   │   │   │   │   │   ├── hyper-rustls v0.27.7 (*)
│   │   │   │   │   │   ├── rustls feature "tls12" (*)
│   │   │   │   │   │   └── tokio-rustls feature "tls12" (*)
│   │   │   │   │   ├── webpki-roots feature "default" (*)
│   │   │   │   │   ├── serde_urlencoded feature "default"
│   │   │   │   │   │   └── serde_urlencoded v0.7.1
│   │   │   │   │   │       ├── serde feature "default" (*)
│   │   │   │   │   │       ├── itoa feature "default" (*)
│   │   │   │   │   │       ├── form_urlencoded feature "default"
│   │   │   │   │   │       │   ├── form_urlencoded v1.2.2 (*)
│   │   │   │   │   │       │   └── form_urlencoded feature "std" (*)
│   │   │   │   │   │       └── ryu feature "default" (*)
│   │   │   │   │   ├── sync_wrapper feature "default"
│   │   │   │   │   │   └── sync_wrapper v1.0.2
│   │   │   │   │   │       └── futures-core v0.3.32
│   │   │   │   │   ├── sync_wrapper feature "futures"
│   │   │   │   │   │   ├── sync_wrapper v1.0.2 (*)
│   │   │   │   │   │   └── sync_wrapper feature "futures-core"
│   │   │   │   │   │       └── sync_wrapper v1.0.2 (*)
│   │   │   │   │   ├── tower feature "retry"
│   │   │   │   │   │   ├── tower v0.5.3
│   │   │   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │   │   │   ├── futures-util feature "alloc" (*)
│   │   │   │   │   │   │   ├── futures-core feature "default" (*)
│   │   │   │   │   │   │   ├── tower-service feature "default" (*)
│   │   │   │   │   │   │   ├── sync_wrapper feature "default" (*)
│   │   │   │   │   │   │   └── tower-layer feature "default"
│   │   │   │   │   │   │       └── tower-layer v0.3.3
│   │   │   │   │   │   ├── tokio feature "time" (*)
│   │   │   │   │   │   ├── tower feature "tokio"
│   │   │   │   │   │   │   └── tower v0.5.3 (*)
│   │   │   │   │   │   └── tower feature "util"
│   │   │   │   │   │       ├── tower v0.5.3 (*)
│   │   │   │   │   │       ├── tower feature "futures-core"
│   │   │   │   │   │       │   └── tower v0.5.3 (*)
│   │   │   │   │   │       ├── tower feature "futures-util"
│   │   │   │   │   │       │   └── tower v0.5.3 (*)
│   │   │   │   │   │       ├── tower feature "pin-project-lite"
│   │   │   │   │   │       │   └── tower v0.5.3 (*)
│   │   │   │   │   │       └── tower feature "sync_wrapper"
│   │   │   │   │   │           └── tower v0.5.3 (*)
│   │   │   │   │   ├── tower feature "timeout"
│   │   │   │   │   │   ├── tower v0.5.3 (*)
│   │   │   │   │   │   ├── tokio feature "time" (*)
│   │   │   │   │   │   ├── tower feature "pin-project-lite" (*)
│   │   │   │   │   │   └── tower feature "tokio" (*)
│   │   │   │   │   ├── tower feature "util" (*)
│   │   │   │   │   └── tower-http feature "follow-redirect"
│   │   │   │   │       ├── tower-http v0.6.8
│   │   │   │   │       │   ├── futures-util v0.3.32 (*)
│   │   │   │   │       │   ├── bytes feature "default" (*)
│   │   │   │   │       │   ├── pin-project-lite feature "default" (*)
│   │   │   │   │       │   ├── http feature "default" (*)
│   │   │   │   │       │   ├── http-body feature "default" (*)
│   │   │   │   │       │   ├── tower-service feature "default" (*)
│   │   │   │   │       │   ├── bitflags feature "default" (*)
│   │   │   │   │       │   ├── tower feature "default"
│   │   │   │   │       │   │   └── tower v0.5.3 (*)
│   │   │   │   │       │   ├── tower-layer feature "default" (*)
│   │   │   │   │       │   └── iri-string feature "default"
│   │   │   │   │       │       ├── iri-string v0.7.10
│   │   │   │   │       │       └── iri-string feature "std"
│   │   │   │   │       │           ├── iri-string v0.7.10
│   │   │   │   │       │           └── iri-string feature "alloc"
│   │   │   │   │       │               └── iri-string v0.7.10
│   │   │   │   │       ├── tower feature "util" (*)
│   │   │   │   │       ├── tower-http feature "futures-util"
│   │   │   │   │       │   └── tower-http v0.6.8 (*)
│   │   │   │   │       ├── tower-http feature "iri-string"
│   │   │   │   │       │   └── tower-http v0.6.8 (*)
│   │   │   │   │       └── tower-http feature "tower"
│   │   │   │   │           └── tower-http v0.6.8 (*)
│   │   │   │   └── reqwest feature "rustls-tls-webpki-roots"
│   │   │   │       ├── reqwest v0.12.28 (*)
│   │   │   │       ├── reqwest feature "__rustls-ring"
│   │   │   │       │   ├── reqwest v0.12.28 (*)
│   │   │   │       │   ├── rustls feature "ring" (*)
│   │   │   │       │   ├── tokio-rustls feature "ring" (*)
│   │   │   │       │   └── hyper-rustls feature "ring"
│   │   │   │       │       ├── hyper-rustls v0.27.7 (*)
│   │   │   │       │       └── rustls feature "ring" (*)
│   │   │   │       └── reqwest feature "rustls-tls-webpki-roots-no-provider"
│   │   │   │           ├── reqwest v0.12.28 (*)
│   │   │   │           ├── reqwest feature "__rustls"
│   │   │   │           │   ├── reqwest v0.12.28 (*)
│   │   │   │           │   └── reqwest feature "__tls"
│   │   │   │           │       ├── reqwest v0.12.28 (*)
│   │   │   │           │       └── tokio feature "io-util" (*)
│   │   │   │           └── hyper-rustls feature "webpki-tokio"
│   │   │   │               ├── hyper-rustls v0.27.7 (*)
│   │   │   │               └── hyper-rustls feature "webpki-roots"
│   │   │   │                   └── hyper-rustls v0.27.7 (*)
│   │   │   ├── webpki-roots feature "default" (*)
│   │   │   ├── serde_bytes feature "default"
│   │   │   │   ├── serde_bytes v0.11.19
│   │   │   │   │   └── serde_core v1.0.228
│   │   │   │   └── serde_bytes feature "std"
│   │   │   │       ├── serde_bytes v0.11.19 (*)
│   │   │   │       └── serde_core feature "std" (*)
│   │   │   ├── strum feature "default"
│   │   │   │   ├── strum v0.28.0
│   │   │   │   │   └── strum_macros feature "default"
│   │   │   │   │       └── strum_macros v0.28.0 (proc-macro)
│   │   │   │   │           ├── proc-macro2 feature "default" (*)
│   │   │   │   │           ├── quote feature "default" (*)
│   │   │   │   │           ├── syn feature "default" (*)
│   │   │   │   │           ├── syn feature "parsing" (*)
│   │   │   │   │           └── heck feature "default" (*)
│   │   │   │   └── strum feature "std"
│   │   │   │       └── strum v0.28.0 (*)
│   │   │   ├── strum feature "derive"
│   │   │   │   ├── strum v0.28.0 (*)
│   │   │   │   └── strum feature "strum_macros"
│   │   │   │       └── strum v0.28.0 (*)
│   │   │   ├── tokio-websockets feature "client"
│   │   │   │   ├── tokio-websockets v0.12.3
│   │   │   │   │   ├── getrandom v0.3.4 (*)
│   │   │   │   │   ├── ring v0.17.14 (*)
│   │   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── futures-core feature "default" (*)
│   │   │   │   │   ├── futures-sink feature "default" (*)
│   │   │   │   │   ├── http feature "std" (*)
│   │   │   │   │   ├── tokio-util feature "codec" (*)
│   │   │   │   │   ├── tokio-util feature "default" (*)
│   │   │   │   │   ├── tokio-util feature "io" (*)
│   │   │   │   │   ├── rand feature "thread_rng" (*)
│   │   │   │   │   ├── rustls-pki-types feature "default" (*)
│   │   │   │   │   ├── httparse feature "default" (*)
│   │   │   │   │   ├── base64 feature "default" (*)
│   │   │   │   │   ├── simdutf8 feature "aarch64_neon"
│   │   │   │   │   │   └── simdutf8 v0.1.5
│   │   │   │   │   └── simdutf8 feature "std"
│   │   │   │   │       └── simdutf8 v0.1.5
│   │   │   │   ├── tokio feature "io-util" (*)
│   │   │   │   └── tokio feature "net" (*)
│   │   │   ├── tokio-websockets feature "default"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   ├── tokio-websockets feature "getrandom"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   ├── tokio-websockets feature "rand"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   ├── tokio-websockets feature "ring"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   ├── tokio-websockets feature "rustls-bring-your-own-connector"
│   │   │   │   └── tokio-websockets v0.12.3 (*)
│   │   │   └── z32 feature "default"
│   │   │       └── z32 v1.3.0
│   │   │   [build-dependencies]
│   │   │   ├── cfg_aliases feature "default" (*)
│   │   │   └── vergen-gitcl feature "default"
│   │   │       └── vergen-gitcl v1.0.8
│   │   │           ├── vergen v9.1.0
│   │   │           │   ├── anyhow feature "default"
│   │   │           │   │   ├── anyhow v1.0.102
│   │   │           │   │   └── anyhow feature "std"
│   │   │           │   │       └── anyhow v1.0.102
│   │   │           │   ├── derive_builder feature "default"
│   │   │           │   │   ├── derive_builder v0.20.2
│   │   │           │   │   │   └── derive_builder_macro feature "default"
│   │   │           │   │   │       └── derive_builder_macro v0.20.2 (proc-macro)
│   │   │           │   │   │           ├── syn feature "default" (*)
│   │   │           │   │   │           ├── syn feature "extra-traits" (*)
│   │   │           │   │   │           ├── syn feature "full" (*)
│   │   │           │   │   │           └── derive_builder_core feature "default"
│   │   │           │   │   │               └── derive_builder_core v0.20.2
│   │   │           │   │   │                   ├── proc-macro2 feature "default" (*)
│   │   │           │   │   │                   ├── quote feature "default" (*)
│   │   │           │   │   │                   ├── syn feature "default" (*)
│   │   │           │   │   │                   ├── syn feature "extra-traits" (*)
│   │   │           │   │   │                   ├── syn feature "full" (*)
│   │   │           │   │   │                   └── darling feature "default"
│   │   │           │   │   │                       ├── darling v0.20.11
│   │   │           │   │   │                       │   ├── darling_core feature "default"
│   │   │           │   │   │                       │   │   └── darling_core v0.20.11
│   │   │           │   │   │                       │   │       ├── proc-macro2 feature "default" (*)
│   │   │           │   │   │                       │   │       ├── quote feature "default" (*)
│   │   │           │   │   │                       │   │       ├── syn feature "default" (*)
│   │   │           │   │   │                       │   │       ├── syn feature "extra-traits" (*)
│   │   │           │   │   │                       │   │       ├── syn feature "full" (*)
│   │   │           │   │   │                       │   │       ├── fnv feature "default" (*)
│   │   │           │   │   │                       │   │       ├── ident_case feature "default"
│   │   │           │   │   │                       │   │       │   └── ident_case v1.0.1
│   │   │           │   │   │                       │   │       └── strsim feature "default"
│   │   │           │   │   │                       │   │           └── strsim v0.11.1
│   │   │           │   │   │                       │   └── darling_macro feature "default"
│   │   │           │   │   │                       │       └── darling_macro v0.20.11 (proc-macro)
│   │   │           │   │   │                       │           ├── quote feature "default" (*)
│   │   │           │   │   │                       │           ├── syn feature "default" (*)
│   │   │           │   │   │                       │           └── darling_core feature "default" (*)
│   │   │           │   │   │                       └── darling feature "suggestions"
│   │   │           │   │   │                           ├── darling v0.20.11 (*)
│   │   │           │   │   │                           └── darling_core feature "suggestions"
│   │   │           │   │   │                               ├── darling_core v0.20.11 (*)
│   │   │           │   │   │                               └── darling_core feature "strsim"
│   │   │           │   │   │                                   └── darling_core v0.20.11 (*)
│   │   │           │   │   └── derive_builder feature "std"
│   │   │           │   │       ├── derive_builder v0.20.2 (*)
│   │   │           │   │       └── derive_builder_macro feature "lib_has_std"
│   │   │           │   │           ├── derive_builder_macro v0.20.2 (proc-macro) (*)
│   │   │           │   │           └── derive_builder_core feature "lib_has_std"
│   │   │           │   │               └── derive_builder_core v0.20.2 (*)
│   │   │           │   └── vergen-lib feature "default"
│   │   │           │       └── vergen-lib v9.1.0
│   │   │           │           ├── anyhow feature "default" (*)
│   │   │           │           └── derive_builder feature "default" (*)
│   │   │           │           [build-dependencies]
│   │   │           │           └── rustversion feature "default" (*)
│   │   │           │   [build-dependencies]
│   │   │           │   └── rustversion feature "default" (*)
│   │   │           ├── anyhow feature "default" (*)
│   │   │           ├── derive_builder feature "default" (*)
│   │   │           ├── time feature "default"
│   │   │           │   ├── time v0.3.47
│   │   │           │   │   ├── powerfmt v0.2.0
│   │   │           │   │   ├── libc feature "default" (*)
│   │   │           │   │   ├── itoa feature "default" (*)
│   │   │           │   │   ├── deranged feature "default"
│   │   │           │   │   │   └── deranged v0.5.8
│   │   │           │   │   │       └── powerfmt v0.2.0
│   │   │           │   │   ├── deranged feature "powerfmt"
│   │   │           │   │   │   └── deranged v0.5.8 (*)
│   │   │           │   │   ├── num-conv feature "default"
│   │   │           │   │   │   └── num-conv v0.2.0
│   │   │           │   │   ├── num_threads feature "default"
│   │   │           │   │   │   └── num_threads v0.1.7
│   │   │           │   │   └── time-core feature "default"
│   │   │           │   │       └── time-core v0.1.8
│   │   │           │   └── time feature "std"
│   │   │           │       ├── time v0.3.47 (*)
│   │   │           │       └── time feature "alloc"
│   │   │           │           └── time v0.3.47 (*)
│   │   │           ├── time feature "formatting"
│   │   │           │   ├── time v0.3.47 (*)
│   │   │           │   └── time feature "std" (*)
│   │   │           ├── time feature "local-offset"
│   │   │           │   ├── time v0.3.47 (*)
│   │   │           │   └── time feature "std" (*)
│   │   │           ├── time feature "parsing"
│   │   │           │   └── time v0.3.47 (*)
│   │   │           ├── vergen-lib feature "default"
│   │   │           │   └── vergen-lib v0.1.6
│   │   │           │       ├── anyhow feature "default" (*)
│   │   │           │       └── derive_builder feature "default" (*)
│   │   │           │       [build-dependencies]
│   │   │           │       └── rustversion feature "default" (*)
│   │   │           └── vergen-lib feature "git"
│   │   │               └── vergen-lib v0.1.6 (*)
│   │   │           [build-dependencies]
│   │   │           └── rustversion feature "default" (*)
│   │   ├── papaya v0.2.3
│   │   │   ├── equivalent feature "default" (*)
│   │   │   └── seize feature "default"
│   │   │       ├── seize v0.5.1
│   │   │       │   └── libc feature "default" (*)
│   │   │       └── seize feature "fast-barrier"
│   │   │           ├── seize v0.5.1 (*)
│   │   │           ├── seize feature "libc"
│   │   │           │   └── seize v0.5.1 (*)
│   │   │           └── seize feature "windows-sys"
│   │   │               └── seize v0.5.1 (*)
│   │   ├── pkarr v5.0.2 (*)
│   │   ├── portmapper v0.15.0
│   │   │   ├── iroh-metrics v0.38.3 (*)
│   │   │   ├── serde feature "default" (*)
│   │   │   ├── serde feature "derive" (*)
│   │   │   ├── serde feature "rc" (*)
│   │   │   ├── tokio feature "default" (*)
│   │   │   ├── tokio feature "fs" (*)
│   │   │   ├── tokio feature "io-std" (*)
│   │   │   ├── tokio feature "io-util" (*)
│   │   │   ├── tokio feature "macros" (*)
│   │   │   ├── tokio feature "net" (*)
│   │   │   ├── tokio feature "rt" (*)
│   │   │   ├── tokio feature "sync" (*)
│   │   │   ├── bytes feature "default" (*)
│   │   │   ├── libc feature "default" (*)
│   │   │   ├── socket2 feature "default" (*)
│   │   │   ├── derive_more feature "debug" (*)
│   │   │   ├── derive_more feature "default" (*)
│   │   │   ├── derive_more feature "deref" (*)
│   │   │   ├── derive_more feature "display" (*)
│   │   │   ├── derive_more feature "from" (*)
│   │   │   ├── derive_more feature "try_into" (*)
│   │   │   ├── futures-util feature "default" (*)
│   │   │   ├── tokio-util feature "codec" (*)
│   │   │   ├── tokio-util feature "default" (*)
│   │   │   ├── tokio-util feature "io" (*)
│   │   │   ├── tokio-util feature "io-util" (*)
│   │   │   ├── tokio-util feature "rt" (*)
│   │   │   ├── tracing feature "default" (*)
│   │   │   ├── smallvec feature "default" (*)
│   │   │   ├── rand feature "default" (*)
│   │   │   ├── url feature "default" (*)
│   │   │   ├── url feature "serde" (*)
│   │   │   ├── n0-error feature "default" (*)
│   │   │   ├── hyper-util feature "default" (*)
│   │   │   ├── base64 feature "default" (*)
│   │   │   ├── futures-lite feature "default" (*)
│   │   │   ├── num_enum feature "default" (*)
│   │   │   ├── tower-layer feature "default" (*)
│   │   │   ├── netwatch feature "default"
│   │   │   │   └── netwatch v0.15.0
│   │   │   │       ├── tokio feature "default" (*)
│   │   │   │       ├── tokio feature "fs" (*)
│   │   │   │       ├── tokio feature "io-std" (*)
│   │   │   │       ├── tokio feature "io-util" (*)
│   │   │   │       ├── tokio feature "macros" (*)
│   │   │   │       ├── tokio feature "net" (*)
│   │   │   │       ├── tokio feature "rt" (*)
│   │   │   │       ├── tokio feature "sync" (*)
│   │   │   │       ├── tokio feature "time" (*)
│   │   │   │       ├── bytes feature "default" (*)
│   │   │   │       ├── libc feature "default" (*)
│   │   │   │       ├── pin-project-lite feature "default" (*)
│   │   │   │       ├── socket2 feature "all" (*)
│   │   │   │       ├── socket2 feature "default" (*)
│   │   │   │       ├── atomic-waker feature "default" (*)
│   │   │   │       ├── tokio-util feature "default" (*)
│   │   │   │       ├── tokio-util feature "rt" (*)
│   │   │   │       ├── tracing feature "default" (*)
│   │   │   │       ├── n0-error feature "default" (*)
│   │   │   │       ├── n0-future feature "default" (*)
│   │   │   │       ├── noq-udp feature "default"
│   │   │   │       │   ├── noq-udp v0.9.0 (*)
│   │   │   │       │   ├── noq-udp feature "tracing" (*)
│   │   │   │       │   └── noq-udp feature "tracing-log"
│   │   │   │       │       ├── noq-udp v0.9.0 (*)
│   │   │   │       │       ├── tracing feature "log" (*)
│   │   │   │       │       └── noq-udp feature "tracing" (*)
│   │   │   │       ├── n0-watcher feature "default"
│   │   │   │       │   └── n0-watcher v0.6.1
│   │   │   │       │       ├── derive_more feature "debug" (*)
│   │   │   │       │       ├── derive_more feature "default" (*)
│   │   │   │       │       ├── n0-error feature "default" (*)
│   │   │   │       │       └── n0-future feature "default" (*)
│   │   │   │       ├── netdev feature "default"
│   │   │   │       │   ├── netdev v0.40.1
│   │   │   │       │   │   ├── libc feature "default" (*)
│   │   │   │       │   │   ├── ipnet feature "default" (*)
│   │   │   │       │   │   ├── mac-addr feature "default"
│   │   │   │       │   │   │   ├── mac-addr v0.3.0
│   │   │   │       │   │   │   └── mac-addr feature "std"
│   │   │   │       │   │   │       └── mac-addr v0.3.0
│   │   │   │       │   │   ├── netlink-packet-core feature "default"
│   │   │   │       │   │   │   └── netlink-packet-core v0.8.1
│   │   │   │       │   │   │       └── paste feature "default"
│   │   │   │       │   │   │           └── paste v1.0.15 (proc-macro)
│   │   │   │       │   │   ├── netlink-packet-route feature "default"
│   │   │   │       │   │   │   └── netlink-packet-route v0.29.0
│   │   │   │       │   │   │       ├── libc feature "default" (*)
│   │   │   │       │   │   │       ├── log feature "default" (*)
│   │   │   │       │   │   │       ├── log feature "std"
│   │   │   │       │   │   │       │   └── log v0.4.29
│   │   │   │       │   │   │       ├── bitflags feature "default" (*)
│   │   │   │       │   │   │       └── netlink-packet-core feature "default" (*)
│   │   │   │       │   │   └── netlink-sys feature "default"
│   │   │   │       │   │       └── netlink-sys v0.8.8
│   │   │   │       │   │           ├── tokio feature "net" (*)
│   │   │   │       │   │           ├── bytes feature "default" (*)
│   │   │   │       │   │           ├── libc feature "default" (*)
│   │   │   │       │   │           ├── futures-util feature "default" (*)
│   │   │   │       │   │           └── log feature "default" (*)
│   │   │   │       │   └── netdev feature "gateway"
│   │   │   │       │       └── netdev v0.40.1 (*)
│   │   │   │       ├── netlink-packet-core feature "default" (*)
│   │   │   │       ├── netlink-packet-route feature "default" (*)
│   │   │   │       ├── netlink-sys feature "default" (*)
│   │   │   │       ├── netlink-proto feature "default"
│   │   │   │       │   ├── netlink-proto v0.12.0
│   │   │   │       │   │   ├── netlink-sys v0.8.8 (*)
│   │   │   │       │   │   ├── thiserror feature "default" (*)
│   │   │   │       │   │   ├── bytes feature "default" (*)
│   │   │   │       │   │   ├── log feature "default" (*)
│   │   │   │       │   │   ├── netlink-packet-core feature "default" (*)
│   │   │   │       │   │   └── futures feature "default"
│   │   │   │       │   │       ├── futures v0.3.32
│   │   │   │       │   │       │   ├── futures-core v0.3.32
│   │   │   │       │   │       │   ├── futures-executor v0.3.32
│   │   │   │       │   │       │   │   ├── futures-core v0.3.32
│   │   │   │       │   │       │   │   ├── futures-task v0.3.32
│   │   │   │       │   │       │   │   └── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   ├── futures-io v0.3.32
│   │   │   │       │   │       │   ├── futures-sink v0.3.32
│   │   │   │       │   │       │   ├── futures-task v0.3.32
│   │   │   │       │   │       │   ├── futures-util feature "sink" (*)
│   │   │   │       │   │       │   └── futures-channel feature "sink"
│   │   │   │       │   │       │       ├── futures-channel v0.3.32 (*)
│   │   │   │       │   │       │       └── futures-channel feature "futures-sink"
│   │   │   │       │   │       │           └── futures-channel v0.3.32 (*)
│   │   │   │       │   │       ├── futures feature "async-await"
│   │   │   │       │   │       │   ├── futures v0.3.32 (*)
│   │   │   │       │   │       │   ├── futures-util feature "async-await" (*)
│   │   │   │       │   │       │   └── futures-util feature "async-await-macro" (*)
│   │   │   │       │   │       ├── futures feature "executor"
│   │   │   │       │   │       │   ├── futures v0.3.32 (*)
│   │   │   │       │   │       │   ├── futures feature "futures-executor"
│   │   │   │       │   │       │   │   └── futures v0.3.32 (*)
│   │   │   │       │   │       │   ├── futures feature "std"
│   │   │   │       │   │       │   │   ├── futures v0.3.32 (*)
│   │   │   │       │   │       │   │   ├── futures-util feature "channel"
│   │   │   │       │   │       │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   ├── futures-util feature "futures-channel"
│   │   │   │       │   │       │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   └── futures-util feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-util feature "io"
│   │   │   │       │   │       │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   ├── futures-util feature "futures-io"
│   │   │   │       │   │       │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   ├── futures-util feature "memchr"
│   │   │   │       │   │       │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │       │   │       │   │   │   └── futures-util feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-util feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-core feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-sink feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-io feature "std" (*)
│   │   │   │       │   │       │   │   ├── futures-task feature "std" (*)
│   │   │   │       │   │       │   │   └── futures feature "alloc"
│   │   │   │       │   │       │   │       ├── futures v0.3.32 (*)
│   │   │   │       │   │       │   │       ├── futures-util feature "alloc" (*)
│   │   │   │       │   │       │   │       ├── futures-channel feature "alloc" (*)
│   │   │   │       │   │       │   │       ├── futures-core feature "alloc" (*)
│   │   │   │       │   │       │   │       ├── futures-sink feature "alloc" (*)
│   │   │   │       │   │       │   │       └── futures-task feature "alloc" (*)
│   │   │   │       │   │       │   └── futures-executor feature "std"
│   │   │   │       │   │       │       ├── futures-executor v0.3.32 (*)
│   │   │   │       │   │       │       ├── futures-util feature "std" (*)
│   │   │   │       │   │       │       ├── futures-core feature "std" (*)
│   │   │   │       │   │       │       └── futures-task feature "std" (*)
│   │   │   │       │   │       └── futures feature "std" (*)
│   │   │   │       │   └── netlink-proto feature "tokio_socket"
│   │   │   │       │       ├── netlink-proto v0.12.0 (*)
│   │   │   │       │       └── netlink-sys feature "tokio_socket"
│   │   │   │       │           ├── netlink-sys v0.8.8 (*)
│   │   │   │       │           ├── netlink-sys feature "futures-util"
│   │   │   │       │           │   └── netlink-sys v0.8.8 (*)
│   │   │   │       │           └── netlink-sys feature "tokio"
│   │   │   │       │               └── netlink-sys v0.8.8 (*)
│   │   │   │       └── time feature "default"
│   │   │   │           ├── time v0.3.47
│   │   │   │           │   ├── powerfmt v0.2.0
│   │   │   │           │   ├── deranged feature "default" (*)
│   │   │   │           │   ├── deranged feature "powerfmt" (*)
│   │   │   │           │   ├── num-conv feature "default" (*)
│   │   │   │           │   └── time-core feature "default" (*)
│   │   │   │           └── time feature "std"
│   │   │   │               ├── time v0.3.47 (*)
│   │   │   │               └── time feature "alloc"
│   │   │   │                   └── time v0.3.47 (*)
│   │   │   │       [build-dependencies]
│   │   │   │       └── cfg_aliases feature "default" (*)
│   │   │   ├── time feature "default" (*)
│   │   │   ├── igd-next feature "aio_tokio"
│   │   │   │   ├── igd-next v0.16.2
│   │   │   │   │   ├── attohttpc v0.30.1
│   │   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   │   ├── log feature "default" (*)
│   │   │   │   │   │   ├── url feature "default" (*)
│   │   │   │   │   │   └── base64 feature "default" (*)
│   │   │   │   │   ├── tokio feature "default" (*)
│   │   │   │   │   ├── tokio feature "net" (*)
│   │   │   │   │   ├── bytes feature "default" (*)
│   │   │   │   │   ├── async-trait feature "default" (*)
│   │   │   │   │   ├── http feature "default" (*)
│   │   │   │   │   ├── log feature "default" (*)
│   │   │   │   │   ├── rand feature "default" (*)
│   │   │   │   │   ├── url feature "default" (*)
│   │   │   │   │   ├── http-body-util feature "default" (*)
│   │   │   │   │   ├── hyper feature "client" (*)
│   │   │   │   │   ├── hyper feature "http1" (*)
│   │   │   │   │   ├── hyper feature "http2"
│   │   │   │   │   │   └── hyper v1.8.1 (*)
│   │   │   │   │   ├── hyper-util feature "client" (*)
│   │   │   │   │   ├── hyper-util feature "client-legacy" (*)
│   │   │   │   │   ├── hyper-util feature "http1" (*)
│   │   │   │   │   ├── hyper-util feature "http2"
│   │   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   │   └── hyper feature "http2" (*)
│   │   │   │   │   ├── futures feature "default" (*)
│   │   │   │   │   └── xmltree feature "default"
│   │   │   │   │       └── xmltree v0.10.3
│   │   │   │   │           └── xml-rs feature "default"
│   │   │   │   │               └── xml-rs v0.8.28
│   │   │   │   ├── igd-next feature "async-trait"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "bytes"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "futures"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "http"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "http-body-util"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "hyper"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   ├── igd-next feature "hyper-util"
│   │   │   │   │   └── igd-next v0.16.2 (*)
│   │   │   │   └── igd-next feature "tokio"
│   │   │   │       └── igd-next v0.16.2 (*)
│   │   │   └── igd-next feature "default"
│   │   │       └── igd-next v0.16.2 (*)
│   │   ├── serde feature "default" (*)
│   │   ├── serde feature "derive" (*)
│   │   ├── serde feature "rc" (*)
│   │   ├── backon feature "default"
│   │   │   ├── backon v1.6.0
│   │   │   │   ├── fastrand v2.3.0
│   │   │   │   └── tokio feature "default" (*)
│   │   │   ├── backon feature "gloo-timers-sleep"
│   │   │   │   └── backon v1.6.0 (*)
│   │   │   ├── backon feature "std"
│   │   │   │   ├── backon v1.6.0 (*)
│   │   │   │   └── fastrand feature "std" (*)
│   │   │   ├── backon feature "std-blocking-sleep"
│   │   │   │   └── backon v1.6.0 (*)
│   │   │   └── backon feature "tokio-sleep"
│   │   │       ├── backon v1.6.0 (*)
│   │   │       ├── backon feature "tokio"
│   │   │       │   └── backon v1.6.0 (*)
│   │   │       └── tokio feature "time" (*)
│   │   ├── tokio feature "default" (*)
│   │   ├── tokio feature "fs" (*)
│   │   ├── tokio feature "io-std" (*)
│   │   ├── tokio feature "io-util" (*)
│   │   ├── tokio feature "macros" (*)
│   │   ├── tokio feature "net" (*)
│   │   ├── tokio feature "rt" (*)
│   │   ├── tokio feature "sync" (*)
│   │   ├── bytes feature "default" (*)
│   │   ├── data-encoding feature "default" (*)
│   │   ├── derive_more feature "debug" (*)
│   │   ├── derive_more feature "default" (*)
│   │   ├── derive_more feature "deref" (*)
│   │   ├── derive_more feature "display" (*)
│   │   ├── derive_more feature "from" (*)
│   │   ├── derive_more feature "from_str"
│   │   │   ├── derive_more v2.1.1 (*)
│   │   │   └── derive_more-impl feature "from_str"
│   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   │       ├── syn feature "full" (*)
│   │   │       └── syn feature "visit" (*)
│   │   ├── derive_more feature "into_iterator"
│   │   │   ├── derive_more v2.1.1 (*)
│   │   │   └── derive_more-impl feature "into_iterator"
│   │   │       └── derive_more-impl v2.1.1 (proc-macro) (*)
│   │   ├── derive_more feature "try_into" (*)
│   │   ├── ed25519-dalek feature "default" (*)
│   │   ├── ed25519-dalek feature "pem"
│   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│   │   │   ├── ed25519-dalek feature "alloc" (*)
│   │   │   ├── ed25519-dalek feature "pkcs8"
│   │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│   │   │   │   └── ed25519 feature "pkcs8"
│   │   │   │       └── ed25519 v3.0.0-rc.4 (*)
│   │   │   └── ed25519 feature "pem"
│   │   │       ├── ed25519 v3.0.0-rc.4 (*)
│   │   │       ├── ed25519 feature "alloc" (*)
│   │   │       ├── ed25519 feature "pkcs8" (*)
│   │   │       └── pkcs8 feature "pem"
│   │   │           ├── pkcs8 v0.11.0-rc.11 (*)
│   │   │           ├── pkcs8 feature "alloc" (*)
│   │   │           ├── der feature "pem"
│   │   │           │   ├── der v0.8.0 (*)
│   │   │           │   ├── der feature "alloc" (*)
│   │   │           │   └── der feature "zeroize" (*)
│   │   │           └── spki feature "pem"
│   │   │               ├── spki v0.8.0-rc.4 (*)
│   │   │               ├── der feature "pem" (*)
│   │   │               └── spki feature "alloc" (*)
│   │   ├── ed25519-dalek feature "pkcs8" (*)
│   │   ├── ed25519-dalek feature "rand_core" (*)
│   │   ├── ed25519-dalek feature "serde" (*)
│   │   ├── ed25519-dalek feature "zeroize" (*)
│   │   ├── pkcs8 feature "default" (*)
│   │   ├── futures-util feature "default" (*)
│   │   ├── hickory-resolver feature "default" (*)
│   │   ├── http feature "default" (*)
│   │   ├── tokio-util feature "default" (*)
│   │   ├── tokio-util feature "io" (*)
│   │   ├── tokio-util feature "io-util" (*)
│   │   ├── tokio-util feature "rt" (*)
│   │   ├── tracing feature "default" (*)
│   │   ├── portable-atomic feature "default" (*)
│   │   ├── smallvec feature "default" (*)
│   │   ├── ipnet feature "default" (*)
│   │   ├── rand feature "default" (*)
│   │   ├── rustls feature "ring" (*)
│   │   ├── rustls-pki-types feature "default" (*)
│   │   ├── rustls-webpki feature "default"
│   │   │   ├── rustls-webpki v0.103.9 (*)
│   │   │   └── rustls-webpki feature "std" (*)
│   │   ├── rustls-webpki feature "ring" (*)
│   │   ├── url feature "default" (*)
│   │   ├── url feature "serde" (*)
│   │   ├── iroh-base feature "key" (*)
│   │   ├── iroh-base feature "relay" (*)
│   │   ├── n0-error feature "default" (*)
│   │   ├── n0-future feature "default" (*)
│   │   ├── pin-project feature "default" (*)
│   │   ├── noq feature "runtime-tokio"
│   │   │   ├── noq v0.17.0 (*)
│   │   │   ├── tokio feature "net" (*)
│   │   │   ├── tokio feature "rt" (*)
│   │   │   └── tokio feature "time" (*)
│   │   ├── noq feature "rustls-ring" (*)
│   │   ├── noq-proto feature "default" (*)
│   │   ├── rustc-hash feature "default" (*)
│   │   ├── noq-udp feature "default" (*)
│   │   ├── tokio-stream feature "default" (*)
│   │   ├── tokio-stream feature "sync" (*)
│   │   ├── reqwest feature "rustls-tls" (*)
│   │   ├── reqwest feature "stream"
│   │   │   ├── reqwest v0.12.28 (*)
│   │   │   └── tokio feature "fs" (*)
│   │   ├── webpki-roots feature "default" (*)
│   │   ├── sync_wrapper feature "default" (*)
│   │   ├── sync_wrapper feature "futures" (*)
│   │   ├── strum feature "default" (*)
│   │   ├── strum feature "derive" (*)
│   │   ├── n0-watcher feature "default" (*)
│   │   └── netwatch feature "default" (*)
│   │   [build-dependencies]
│   │   └── cfg_aliases feature "default" (*)
│   ├── iroh feature "fast-apple-datapath"
│   │   ├── iroh v0.97.0 (*)
│   │   └── noq feature "fast-apple-datapath"
│   │       ├── noq v0.17.0 (*)
│   │       └── noq-udp feature "fast-apple-datapath"
│   │           └── noq-udp v0.9.0 (*)
│   ├── iroh feature "metrics"
│   │   ├── iroh v0.97.0 (*)
│   │   ├── iroh-metrics feature "metrics"
│   │   │   └── iroh-metrics v0.38.3 (*)
│   │   ├── iroh-relay feature "metrics"
│   │   │   ├── iroh-relay v0.97.0 (*)
│   │   │   └── iroh-metrics feature "metrics" (*)
│   │   └── portmapper feature "metrics"
│   │       ├── portmapper v0.15.0 (*)
│   │       └── iroh-metrics feature "metrics" (*)
│   └── iroh feature "portmapper"
│       └── iroh v0.97.0 (*)
├── proptest feature "default"
│   ├── proptest v1.10.0
│   │   ├── num-traits v0.2.19
│   │   │   [build-dependencies]
│   │   │   └── autocfg feature "default"
│   │   │       └── autocfg v1.5.0
│   │   ├── rand_chacha v0.9.0 (*)
│   │   ├── rusty-fork v0.3.1
│   │   │   ├── fnv feature "default" (*)
│   │   │   ├── quick-error feature "default"
│   │   │   │   └── quick-error v1.2.3
│   │   │   ├── tempfile feature "default"
│   │   │   │   ├── tempfile v3.27.0
│   │   │   │   │   ├── getrandom v0.4.2 (*)
│   │   │   │   │   ├── fastrand feature "default"
│   │   │   │   │   │   ├── fastrand v2.3.0
│   │   │   │   │   │   └── fastrand feature "std" (*)
│   │   │   │   │   ├── once_cell feature "std" (*)
│   │   │   │   │   ├── rustix feature "default"
│   │   │   │   │   │   ├── rustix v1.1.4
│   │   │   │   │   │   │   ├── bitflags v2.11.0
│   │   │   │   │   │   │   ├── linux-raw-sys feature "auxvec"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   ├── linux-raw-sys feature "elf"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   ├── linux-raw-sys feature "errno"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   ├── linux-raw-sys feature "general"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   ├── linux-raw-sys feature "ioctl"
│   │   │   │   │   │   │   │   └── linux-raw-sys v0.12.1
│   │   │   │   │   │   │   └── linux-raw-sys feature "no_std"
│   │   │   │   │   │   │       └── linux-raw-sys v0.12.1
│   │   │   │   │   │   └── rustix feature "std"
│   │   │   │   │   │       ├── rustix v1.1.4 (*)
│   │   │   │   │   │       ├── bitflags feature "std"
│   │   │   │   │   │       │   └── bitflags v2.11.0
│   │   │   │   │   │       └── rustix feature "alloc"
│   │   │   │   │   │           └── rustix v1.1.4 (*)
│   │   │   │   │   └── rustix feature "fs"
│   │   │   │   │       └── rustix v1.1.4 (*)
│   │   │   │   └── tempfile feature "getrandom"
│   │   │   │       └── tempfile v3.27.0 (*)
│   │   │   └── wait-timeout feature "default"
│   │   │       └── wait-timeout v0.2.1
│   │   │           └── libc feature "default" (*)
│   │   ├── rand feature "alloc" (*)
│   │   ├── bitflags feature "default" (*)
│   │   ├── bit-set feature "default"
│   │   │   ├── bit-set v0.8.0
│   │   │   │   └── bit-vec v0.8.0
│   │   │   └── bit-set feature "std"
│   │   │       ├── bit-set v0.8.0 (*)
│   │   │       └── bit-vec feature "std"
│   │   │           └── bit-vec v0.8.0
│   │   ├── bit-vec feature "default"
│   │   │   ├── bit-vec v0.8.0
│   │   │   └── bit-vec feature "std" (*)
│   │   ├── rand_xorshift feature "default"
│   │   │   └── rand_xorshift v0.4.0
│   │   │       └── rand_core feature "default" (*)
│   │   ├── regex-syntax feature "default"
│   │   │   ├── regex-syntax v0.8.10
│   │   │   ├── regex-syntax feature "std"
│   │   │   │   └── regex-syntax v0.8.10
│   │   │   └── regex-syntax feature "unicode"
│   │   │       ├── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-age"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-bool"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-case"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-gencat"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-perl"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       ├── regex-syntax feature "unicode-script"
│   │   │       │   └── regex-syntax v0.8.10
│   │   │       └── regex-syntax feature "unicode-segment"
│   │   │           └── regex-syntax v0.8.10
│   │   ├── tempfile feature "default" (*)
│   │   └── unarray feature "default"
│   │       └── unarray v0.1.4
│   ├── proptest feature "bit-set"
│   │   └── proptest v1.10.0 (*)
│   ├── proptest feature "fork"
│   │   ├── proptest v1.10.0 (*)
│   │   ├── proptest feature "rusty-fork"
│   │   │   └── proptest v1.10.0 (*)
│   │   ├── proptest feature "std"
│   │   │   ├── proptest v1.10.0 (*)
│   │   │   ├── rand feature "os_rng" (*)
│   │   │   ├── rand feature "std" (*)
│   │   │   ├── proptest feature "regex-syntax"
│   │   │   │   └── proptest v1.10.0 (*)
│   │   │   └── num-traits feature "std"
│   │   │       └── num-traits v0.2.19 (*)
│   │   └── proptest feature "tempfile"
│   │       └── proptest v1.10.0 (*)
│   ├── proptest feature "std" (*)
│   └── proptest feature "timeout"
│       ├── proptest v1.10.0 (*)
│       ├── proptest feature "fork" (*)
│       ├── proptest feature "rusty-fork" (*)
│       └── rusty-fork feature "timeout"
│           ├── rusty-fork v0.3.1 (*)
│           └── rusty-fork feature "wait-timeout"
│               └── rusty-fork v0.3.1 (*)
└── trybuild feature "default"
    └── trybuild v1.0.116
        ├── serde feature "default" (*)
        ├── serde_derive feature "default" (*)
        ├── serde_json feature "default"
        │   ├── serde_json v1.0.149 (*)
        │   └── serde_json feature "std"
        │       ├── serde_json v1.0.149 (*)
        │       ├── serde_core feature "std" (*)
        │       └── memchr feature "std" (*)
        ├── glob feature "default"
        │   └── glob v0.3.3
        ├── target-triple feature "default"
        │   └── target-triple v1.0.0
        ├── termcolor feature "default"
        │   └── termcolor v1.4.1
        └── toml feature "default"
            ├── toml v1.0.7+spec-1.1.0
            │   ├── winnow v1.0.0
            │   ├── serde_core feature "alloc" (*)
            │   ├── serde_spanned feature "alloc"
            │   │   ├── serde_spanned v1.0.4
            │   │   │   └── serde_core v1.0.228
            │   │   └── serde_core feature "alloc" (*)
            │   ├── toml_datetime feature "alloc"
            │   │   ├── toml_datetime v1.0.1+spec-1.1.0
            │   │   │   └── serde_core v1.0.228
            │   │   └── serde_core feature "alloc" (*)
            │   ├── toml_parser feature "alloc"
            │   │   └── toml_parser v1.0.10+spec-1.1.0
            │   │       └── winnow v1.0.0
            │   └── toml_writer feature "alloc"
            │       └── toml_writer v1.0.7+spec-1.1.0
            ├── toml feature "display"
            │   └── toml v1.0.7+spec-1.1.0 (*)
            ├── toml feature "parse"
            │   └── toml v1.0.7+spec-1.1.0 (*)
            ├── toml feature "serde"
            │   ├── toml v1.0.7+spec-1.1.0 (*)
            │   ├── serde_spanned feature "serde"
            │   │   └── serde_spanned v1.0.4 (*)
            │   └── toml_datetime feature "serde"
            │       └── toml_datetime v1.0.1+spec-1.1.0 (*)
            └── toml feature "std"
                ├── toml v1.0.7+spec-1.1.0 (*)
                ├── serde_core feature "std" (*)
                ├── serde_spanned feature "std"
                │   ├── serde_spanned v1.0.4 (*)
                │   ├── serde_core feature "std" (*)
                │   └── serde_spanned feature "alloc" (*)
                ├── toml_datetime feature "std"
                │   ├── toml_datetime v1.0.1+spec-1.1.0 (*)
                │   ├── serde_core feature "std" (*)
                │   └── toml_datetime feature "alloc" (*)
                ├── toml_parser feature "std"
                │   ├── toml_parser v1.0.10+spec-1.1.0 (*)
                │   └── toml_parser feature "alloc" (*)
                └── toml_writer feature "std"
                    ├── toml_writer v1.0.7+spec-1.1.0
                    └── toml_writer feature "alloc" (*)
```

## `python3 - <<'PY'`

```text
bash: line 1: warning: here-document at line 1 delimited by end-of-file (wanted `PY')
```

## `import subprocess`

```text
import: unable to open X server `' @ error/import.c/ImportImageCommand/348.
```

## `commands = [`

```text
bash: line 1: commands: command not found
```

## `    'cargo tree -p aspen-hooks-ticket -e normal',`

```text
bash: line 1: cargo tree -p aspen-hooks-ticket -e normal,: command not found
```

## `    'cargo tree -p aspen-hooks-ticket --no-default-features -e normal',`

```text
bash: line 1: cargo tree -p aspen-hooks-ticket --no-default-features -e normal,: command not found
```

## `]`

```text
bash: line 1: ]: command not found
```

## `for command in commands:`

```text
bash: -c: line 2: syntax error: unexpected end of file from `for' command on line 1
```

## `    output = subprocess.check_output(['bash', '-lc', f'env -u CARGO_INCREMENTAL RUSTC_WRAPPER= {command}'], text=True, stderr=subprocess.STDOUT)`

```text
bash: -c: line 1: syntax error near unexpected token `('
bash: -c: line 1: `    output = subprocess.check_output(['bash', '-lc', f'env -u CARGO_INCREMENTAL RUSTC_WRAPPER= {command}'], text=True, stderr=subprocess.STDOUT)'
```

## `    assert ' iroh v' not in output, command`

```text
bash: line 1: assert: command not found
```

## `    assert ' anyhow v' not in output, command`

```text
bash: line 1: assert: command not found
```

## `    print(f'ok: {command} excludes iroh and anyhow')`

```text
bash: -c: line 1: syntax error near unexpected token `f'ok: {command} excludes iroh and anyhow''
bash: -c: line 1: `    print(f'ok: {command} excludes iroh and anyhow')'
```

## `PY`

```text
bash: line 1: PY: command not found
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo check -p aspen-hooks`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo check -p aspen-cli`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/calendar.rs:277:5
    |
277 |     ambient_clock,
    |     ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:846:5
    |
846 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:854:5
    |
854 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
    --> crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs:1005:5
     |
1005 |     ambient_clock,
     |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:468:5
    |
468 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:476:5
    |
476 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: `aspen-cli` (bin "aspen-cli") generated 6 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.32s
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo tree -p aspen-hooks -e features -i aspen-cluster-types`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
├── aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
│   └── aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
│       ├── aspen-core-shell feature "default"
│       │   ├── aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
│       │   │   └── aspen-auth feature "default"
│       │   │       ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
│       │   │       │   └── aspen-client feature "default"
│       │   │       │       └── aspen-hooks v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks)
│       │   │       │           └── aspen-hooks feature "default" (command-line)
│       │   │       └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
│       │   │           └── aspen-transport feature "default"
│       │   │               ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client) (*)
│       │   │               └── aspen-hooks v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks) (*)
│       │   ├── aspen-hooks v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks) (*)
│       │   └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
│       └── aspen-core-shell feature "layer"
│           └── aspen-hooks v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks) (*)
│   └── aspen-core feature "default"
│       ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client) (*)
│       ├── aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination)
│       │   └── aspen-coordination feature "default"
│       │       └── aspen-jobs v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs)
│       │           └── aspen-jobs feature "default"
│       │               └── aspen-hooks v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks) (*)
│       ├── aspen-jobs v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs) (*)
│       ├── aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
│       │   └── aspen-raft-types feature "default"
│       │       └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
│       └── aspen-sharding v0.1.0 (/home/brittonr/git/aspen/crates/aspen-sharding)
│           └── aspen-sharding feature "default"
│               └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
├── aspen-hooks-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks-ticket)
│   └── aspen-hooks-ticket feature "std"
│       └── aspen-hooks v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks) (*)
├── aspen-jobs v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs) (*)
└── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
    └── aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core) (*)
    └── aspen-traits feature "default"
        ├── aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination) (*)
        └── aspen-jobs v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs) (*)
└── aspen-cluster-types feature "iroh"
    └── aspen-hooks v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks) (*)
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo tree -p aspen-cli -e features -i aspen-cluster-types`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
├── aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
│   └── aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
│       ├── aspen-core-shell feature "default"
│       │   ├── aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
│       │   │   └── aspen-auth feature "default"
│       │   │       ├── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli)
│       │   │       │   └── aspen-cli feature "default" (command-line)
│       │   │       ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
│       │   │       │   └── aspen-client feature "default"
│       │   │       │       └── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
│       │   │       ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
│       │   │       │   └── aspen-cluster feature "default"
│       │   │       │       └── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
│       │   │       ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft)
│       │   │       │   └── aspen-raft feature "default"
│       │   │       │       └── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
│       │   │       └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
│       │   │           └── aspen-transport feature "default"
│       │   │               ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client) (*)
│       │   │               ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
│       │   │               ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
│       │   │               └── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network)
│       │   │                   └── aspen-raft-network feature "default"
│       │   │                       └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
│       │   ├── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
│       │   ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
│       │   ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
│       │   └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
│       └── aspen-core-shell feature "layer"
│           ├── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
│           └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
│   └── aspen-core feature "default"
│       ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client) (*)
│       ├── aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination)
│       │   └── aspen-coordination feature "default"
│       │       └── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
│       ├── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network) (*)
│       ├── aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
│       │   └── aspen-raft-types feature "default"
│       │       ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
│       │       ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
│       │       ├── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network) (*)
│       │       └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
│       ├── aspen-redb-storage v0.1.0 (/home/brittonr/git/aspen/crates/aspen-redb-storage)
│       │   └── aspen-redb-storage feature "default"
│       │       └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
│       └── aspen-sharding v0.1.0 (/home/brittonr/git/aspen/crates/aspen-sharding)
│           └── aspen-sharding feature "default"
│               ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
│               ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
│               ├── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network) (*)
│               └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
├── aspen-hooks-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hooks-ticket)
│   └── aspen-hooks-ticket feature "std"
│       └── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
└── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
    └── aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core) (*)
    └── aspen-traits feature "default"
        ├── aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination) (*)
        └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
└── aspen-cluster-types feature "iroh"
    ├── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
    └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
```

## `python3 - <<'PY'`

```text
bash: line 1: warning: here-document at line 1 delimited by end-of-file (wanted `PY')
```

## `from pathlib import Path`

```text
bash: line 1: from: command not found
```

## `required = 'aspen-cluster-types = { workspace = true, features = ["iroh"] }'`

```text
bash: line 1: required: command not found
```

## `for path in ['crates/aspen-hooks/Cargo.toml', 'crates/aspen-cli/Cargo.toml']:`

```text
bash: -c: line 2: syntax error: unexpected end of file from `for' command on line 1
```

## `    text = Path(path).read_text()`

```text
bash: -c: line 1: syntax error near unexpected token `('
bash: -c: line 1: `    text = Path(path).read_text()'
```

## `    assert required in text, path`

```text
bash: line 1: assert: command not found
```

## `    print(f'ok: {path} declares direct aspen-cluster-types iroh opt-in')`

```text
bash: -c: line 1: syntax error near unexpected token `f'ok: {path} declares direct aspen-cluster-types iroh opt-in''
bash: -c: line 1: `    print(f'ok: {path} declares direct aspen-cluster-types iroh opt-in')'
```

## `PY`

```text
bash: line 1: PY: command not found
```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks-ticket test_ticket_roundtrip --lib`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.26s
     Running unittests src/lib.rs (target/debug/deps/aspen_hooks_ticket-8d53a5e5f26b3586)

running 1 test
test tests::test_ticket_roundtrip ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s

```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks-ticket test_validation_invalid_payload_json --lib`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.24s
     Running unittests src/lib.rs (target/debug/deps/aspen_hooks_ticket-8d53a5e5f26b3586)

running 1 test
test tests::test_validation_invalid_payload_json ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s

```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks-ticket test_deserialize_expired_ticket --lib`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.24s
     Running unittests src/lib.rs (target/debug/deps/aspen_hooks_ticket-8d53a5e5f26b3586)

running 1 test
test tests::test_deserialize_expired_ticket ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s

```

## `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks-ticket --test legacy test_legacy_serialized_ticket_is_rejected`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.26s
     Running tests/legacy.rs (target/debug/deps/legacy-3c77f777d7bad8c1)

running 1 test
test test_legacy_serialized_ticket_is_rejected ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

## `env TMPDIR=/run/user/1555/tmp bash -lc 'env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks test_convert_bootstrap_peer_accepts_valid_node_address --lib'`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.28s
     Running unittests src/lib.rs (target/debug/deps/aspen_hooks-70dbf45e8a0ce4f9)

running 1 test
test client::tests::test_convert_bootstrap_peer_accepts_valid_node_address ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 139 filtered out; finished in 0.00s

```

## `env TMPDIR=/run/user/1555/tmp bash -lc 'env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks test_convert_bootstrap_peer_rejects_invalid_node_address --lib'`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.26s
     Running unittests src/lib.rs (target/debug/deps/aspen_hooks-70dbf45e8a0ce4f9)

running 1 test
test client::tests::test_convert_bootstrap_peer_rejects_invalid_node_address ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 139 filtered out; finished in 0.00s

```

## `env TMPDIR=/run/user/1555/tmp bash -lc 'env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks test_expired_url_maps_to_expired_error --lib'`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.26s
     Running unittests src/lib.rs (target/debug/deps/aspen_hooks-70dbf45e8a0ce4f9)

running 1 test
test client::tests::test_expired_url_maps_to_expired_error ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 139 filtered out; finished in 0.00s

```

## `env TMPDIR=/run/user/1555/tmp bash -lc 'env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-hooks test_legacy_url_surfaces_decode_failure --lib'`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.27s
     Running unittests src/lib.rs (target/debug/deps/aspen_hooks-70dbf45e8a0ce4f9)

running 1 test
test client::tests::test_legacy_url_surfaces_decode_failure ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 139 filtered out; finished in 0.00s

```

## `env TMPDIR=/run/user/1555/tmp bash -lc 'env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-cli test_convert_hook_bootstrap_peer_accepts_valid_node_address --bin aspen-cli'`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/calendar.rs:277:5
    |
277 |     ambient_clock,
    |     ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:846:5
    |
846 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:854:5
    |
854 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
    --> crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs:1005:5
     |
1005 |     ambient_clock,
     |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:468:5
    |
468 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:476:5
    |
476 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: `aspen-cli` (bin "aspen-cli" test) generated 6 warnings
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.28s
     Running unittests src/bin/aspen-cli/main.rs (target/debug/deps/aspen_cli-fcf51debf74e3401)

running 1 test
test commands::hooks::tests::test_convert_hook_bootstrap_peer_accepts_valid_node_address ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 291 filtered out; finished in 0.00s

```

## `env TMPDIR=/run/user/1555/tmp bash -lc 'env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-cli test_convert_hook_bootstrap_peer_rejects_invalid_node_address --bin aspen-cli'`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/calendar.rs:277:5
    |
277 |     ambient_clock,
    |     ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:846:5
    |
846 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:854:5
    |
854 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
    --> crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs:1005:5
     |
1005 |     ambient_clock,
     |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:468:5
    |
468 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:476:5
    |
476 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: `aspen-cli` (bin "aspen-cli" test) generated 6 warnings
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.31s
     Running unittests src/bin/aspen-cli/main.rs (target/debug/deps/aspen_cli-fcf51debf74e3401)

running 1 test
test commands::hooks::tests::test_convert_hook_bootstrap_peer_rejects_invalid_node_address ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 291 filtered out; finished in 0.00s

```

## `env TMPDIR=/run/user/1555/tmp bash -lc 'env -u CARGO_INCREMENTAL RUSTC_WRAPPER= cargo test -p aspen-cli test_parse_hook_trigger_ticket_surfaces_legacy_decode_failure --bin aspen-cli'`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/calendar.rs:277:5
    |
277 |     ambient_clock,
    |     ^^^^^^^^^^^^^
    |
    = note: `#[warn(unknown_lints)]` on by default

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:846:5
    |
846 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs:854:5
    |
854 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
    --> crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs:1005:5
     |
1005 |     ambient_clock,
     |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:468:5
    |
468 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: unknown lint: `ambient_clock`
   --> crates/aspen-cli/src/bin/aspen-cli/commands/job.rs:476:5
    |
476 |     ambient_clock,
    |     ^^^^^^^^^^^^^

warning: `aspen-cli` (bin "aspen-cli" test) generated 6 warnings
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.28s
     Running unittests src/bin/aspen-cli/main.rs (target/debug/deps/aspen_cli-fcf51debf74e3401)

running 1 test
test commands::hooks::tests::test_parse_hook_trigger_ticket_surfaces_legacy_decode_failure ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 291 filtered out; finished in 0.00s

```

## `python3 - <<'PY'`

```text
bash: line 1: warning: here-document at line 1 delimited by end-of-file (wanted `PY')
```

## `from pathlib import Path`

```text
bash: line 1: from: command not found
```

## `text = Path('crates/aspen-hooks-ticket/src/lib.rs').read_text()`

```text
bash: -c: line 1: syntax error near unexpected token `('
bash: -c: line 1: `text = Path('crates/aspen-hooks-ticket/src/lib.rs').read_text()'
```

## `start = text.index('impl Ticket for AspenHookTicket')`

```text
bash: -c: line 1: syntax error near unexpected token `('
bash: -c: line 1: `start = text.index('impl Ticket for AspenHookTicket')'
```

## `end = text.index('fn validate_ticket_fields', start)`

```text
bash: -c: line 1: syntax error near unexpected token `('
bash: -c: line 1: `end = text.index('fn validate_ticket_fields', start)'
```

## `block = text[start:end]`

```text
bash: line 1: block: command not found
```

## `assert 'expect(' in block`

```text
bash: line 1: assert: command not found
```

## `assert 'Vec::new()' not in block`

```text
bash: line 1: assert: command not found
```

## `print('ok: AspenHookTicket::to_bytes uses expect and no Vec::new fallback')`

```text
bash: -c: line 1: syntax error near unexpected token `'ok: AspenHookTicket::to_bytes uses expect and no Vec::new fallback''
bash: -c: line 1: `print('ok: AspenHookTicket::to_bytes uses expect and no Vec::new fallback')'
```

## `PY`

```text
bash: line 1: PY: command not found
```

