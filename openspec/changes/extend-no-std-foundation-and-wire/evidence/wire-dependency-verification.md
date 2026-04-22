Evidence-ID: extend-no-std-foundation-and-wire.v3
Task-ID: V3
Artifact-Type: verification
Covers: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-production-modules-avoid-std-only-helpers, architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.protocol-crates-keep-json-helpers-test-only, architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-crates-reject-forbidden-std-helpers, architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.protocol-dependency-verification-is-reviewable

# Wire dependency verification

## `cargo check -p aspen-client-api`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.50s
```

## `cargo check -p aspen-client-api --no-default-features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.28s
```

## `cargo check -p aspen-client-api --no-default-features --target wasm32-unknown-unknown`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.33s
```

## `cargo check -p aspen-coordination-protocol`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.19s
```

## `cargo check -p aspen-coordination-protocol --no-default-features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
```

## `cargo check -p aspen-coordination-protocol --no-default-features --target wasm32-unknown-unknown`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
```

## `cargo check -p aspen-jobs-protocol`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
```

## `cargo check -p aspen-jobs-protocol --no-default-features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.19s
```

## `cargo check -p aspen-jobs-protocol --no-default-features --target wasm32-unknown-unknown`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
```

## `cargo check -p aspen-forge-protocol`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
```

## `cargo check -p aspen-forge-protocol --no-default-features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
```

## `cargo check -p aspen-forge-protocol --no-default-features --target wasm32-unknown-unknown`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.25s
```

## `cargo tree -p aspen-client-api -e normal`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
├── aspen-auth-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth-core)
│   ├── base64 v0.22.1
│   ├── blake3 v1.8.3
│   │   ├── arrayref v0.3.9
│   │   ├── arrayvec v0.7.6
│   │   ├── cfg-if v1.0.4
│   │   ├── constant_time_eq v0.4.2
│   │   └── cpufeatures v0.2.17
│   ├── iroh-base v0.97.0
│   │   ├── curve25519-dalek v5.0.0-pre.1
│   │   │   ├── cfg-if v1.0.4
│   │   │   ├── cpufeatures v0.2.17
│   │   │   ├── curve25519-dalek-derive v0.1.1 (proc-macro)
│   │   │   │   ├── proc-macro2 v1.0.106
│   │   │   │   │   └── unicode-ident v1.0.24
│   │   │   │   ├── quote v1.0.45
│   │   │   │   │   └── proc-macro2 v1.0.106 (*)
│   │   │   │   └── syn v2.0.117
│   │   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │       ├── quote v1.0.45 (*)
│   │   │   │       └── unicode-ident v1.0.24
│   │   │   ├── digest v0.11.0-rc.10
│   │   │   │   ├── block-buffer v0.11.0
│   │   │   │   │   └── hybrid-array v0.4.8
│   │   │   │   │       └── typenum v1.19.0
│   │   │   │   ├── const-oid v0.10.2
│   │   │   │   └── crypto-common v0.2.1
│   │   │   │       └── hybrid-array v0.4.8 (*)
│   │   │   ├── rand_core v0.9.5
│   │   │   ├── serde v1.0.228
│   │   │   │   ├── serde_core v1.0.228
│   │   │   │   └── serde_derive v1.0.228 (proc-macro)
│   │   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │       ├── quote v1.0.45 (*)
│   │   │   │       └── syn v2.0.117 (*)
│   │   │   ├── subtle v2.6.1
│   │   │   └── zeroize v1.8.2
│   │   │       └── zeroize_derive v1.4.3 (proc-macro)
│   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │           ├── quote v1.0.45 (*)
│   │   │           └── syn v2.0.117 (*)
│   │   ├── data-encoding v2.10.0
│   │   ├── derive_more v2.1.1
│   │   │   └── derive_more-impl v2.1.1 (proc-macro)
│   │   │       ├── convert_case v0.10.0
│   │   │       │   └── unicode-segmentation v1.12.0
│   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │       ├── quote v1.0.45 (*)
│   │   │       ├── syn v2.0.117 (*)
│   │   │       └── unicode-xid v0.2.6
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
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18
│   │   │       └── thiserror-impl v2.0.18 (proc-macro)
│   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │           ├── quote v1.0.45 (*)
│   │   │           └── syn v2.0.117 (*)
│   │   ├── heapless v0.7.17
│   │   │   ├── hash32 v0.2.1
│   │   │   │   └── byteorder v1.5.0
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── spin v0.9.8
│   │   │   │   └── lock_api v0.4.14
│   │   │   │       └── scopeguard v1.2.0
│   │   │   └── stable_deref_trait v1.2.1
│   │   └── serde v1.0.228 (*)
│   ├── serde v1.0.228 (*)
│   └── thiserror v2.0.18 (*)
├── aspen-coordination-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination-protocol)
│   └── serde v1.0.228 (*)
├── aspen-forge-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-forge-protocol)
│   └── serde v1.0.228 (*)
├── aspen-jobs-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs-protocol)
│   └── serde v1.0.228 (*)
├── serde v1.0.228 (*)
└── thiserror v2.0.18 (*)
```

## `cargo tree -p aspen-client-api -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
├── aspen-auth-core feature "default"
│   └── aspen-auth-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth-core)
│       ├── blake3 v1.8.3
│       │   ├── arrayvec v0.7.6
│       │   ├── constant_time_eq v0.4.2
│       │   ├── arrayref feature "default"
│       │   │   └── arrayref v0.3.9
│       │   ├── cfg-if feature "default"
│       │   │   └── cfg-if v1.0.4
│       │   └── cpufeatures feature "default"
│       │       └── cpufeatures v0.2.17
│       │   [build-dependencies]
│       │   └── cc feature "default"
│       │       └── cc v1.2.57
│       │           ├── find-msvc-tools feature "default"
│       │           │   └── find-msvc-tools v0.1.9
│       │           └── shlex feature "default"
│       │               ├── shlex v1.3.0
│       │               └── shlex feature "std"
│       │                   └── shlex v1.3.0
│       ├── base64 feature "alloc"
│       │   └── base64 v0.22.1
│       ├── iroh-base feature "key"
│       │   ├── iroh-base v0.97.0
│       │   │   ├── curve25519-dalek feature "default"
│       │   │   │   ├── curve25519-dalek v5.0.0-pre.1
│       │   │   │   │   ├── rand_core v0.9.5
│       │   │   │   │   ├── zeroize v1.8.2
│       │   │   │   │   │   └── zeroize_derive feature "default"
│       │   │   │   │   │       └── zeroize_derive v1.4.3 (proc-macro)
│       │   │   │   │   │           ├── proc-macro2 feature "default"
│       │   │   │   │   │           │   ├── proc-macro2 v1.0.106
│       │   │   │   │   │           │   │   └── unicode-ident feature "default"
│       │   │   │   │   │           │   │       └── unicode-ident v1.0.24
│       │   │   │   │   │           │   └── proc-macro2 feature "proc-macro"
│       │   │   │   │   │           │       └── proc-macro2 v1.0.106 (*)
│       │   │   │   │   │           ├── quote feature "default"
│       │   │   │   │   │           │   ├── quote v1.0.45
│       │   │   │   │   │           │   │   └── proc-macro2 v1.0.106 (*)
│       │   │   │   │   │           │   └── quote feature "proc-macro"
│       │   │   │   │   │           │       ├── quote v1.0.45 (*)
│       │   │   │   │   │           │       └── proc-macro2 feature "proc-macro" (*)
│       │   │   │   │   │           ├── syn feature "default"
│       │   │   │   │   │           │   ├── syn v2.0.117
│       │   │   │   │   │           │   │   ├── proc-macro2 v1.0.106 (*)
│       │   │   │   │   │           │   │   ├── quote v1.0.45 (*)
│       │   │   │   │   │           │   │   └── unicode-ident feature "default" (*)
│       │   │   │   │   │           │   ├── syn feature "clone-impls"
│       │   │   │   │   │           │   │   └── syn v2.0.117 (*)
│       │   │   │   │   │           │   ├── syn feature "derive"
│       │   │   │   │   │           │   │   └── syn v2.0.117 (*)
│       │   │   │   │   │           │   ├── syn feature "parsing"
│       │   │   │   │   │           │   │   └── syn v2.0.117 (*)
│       │   │   │   │   │           │   ├── syn feature "printing"
│       │   │   │   │   │           │   │   └── syn v2.0.117 (*)
│       │   │   │   │   │           │   └── syn feature "proc-macro"
│       │   │   │   │   │           │       ├── syn v2.0.117 (*)
│       │   │   │   │   │           │       ├── proc-macro2 feature "proc-macro" (*)
│       │   │   │   │   │           │       └── quote feature "proc-macro" (*)
│       │   │   │   │   │           ├── syn feature "extra-traits"
│       │   │   │   │   │           │   └── syn v2.0.117 (*)
│       │   │   │   │   │           ├── syn feature "full"
│       │   │   │   │   │           │   └── syn v2.0.117 (*)
│       │   │   │   │   │           └── syn feature "visit"
│       │   │   │   │   │               └── syn v2.0.117 (*)
│       │   │   │   │   ├── cfg-if feature "default" (*)
│       │   │   │   │   ├── cpufeatures feature "default" (*)
│       │   │   │   │   ├── curve25519-dalek-derive feature "default"
│       │   │   │   │   │   └── curve25519-dalek-derive v0.1.1 (proc-macro)
│       │   │   │   │   │       ├── proc-macro2 feature "default" (*)
│       │   │   │   │   │       ├── quote feature "default" (*)
│       │   │   │   │   │       ├── syn feature "default" (*)
│       │   │   │   │   │       └── syn feature "full" (*)
│       │   │   │   │   ├── digest feature "block-api"
│       │   │   │   │   │   ├── digest v0.11.0-rc.10
│       │   │   │   │   │   │   ├── block-buffer feature "default"
│       │   │   │   │   │   │   │   └── block-buffer v0.11.0
│       │   │   │   │   │   │   │       └── hybrid-array feature "default"
│       │   │   │   │   │   │   │           └── hybrid-array v0.4.8
│       │   │   │   │   │   │   │               ├── typenum feature "const-generics"
│       │   │   │   │   │   │   │               │   └── typenum v1.19.0
│       │   │   │   │   │   │   │               └── typenum feature "default"
│       │   │   │   │   │   │   │                   └── typenum v1.19.0
│       │   │   │   │   │   │   ├── const-oid feature "default"
│       │   │   │   │   │   │   │   └── const-oid v0.10.2
│       │   │   │   │   │   │   └── crypto-common feature "default"
│       │   │   │   │   │   │       └── crypto-common v0.2.1
│       │   │   │   │   │   │           └── hybrid-array feature "default" (*)
│       │   │   │   │   │   └── digest feature "block-buffer"
│       │   │   │   │   │       └── digest v0.11.0-rc.10 (*)
│       │   │   │   │   ├── serde feature "derive"
│       │   │   │   │   │   ├── serde v1.0.228
│       │   │   │   │   │   │   ├── serde_core feature "result"
│       │   │   │   │   │   │   │   └── serde_core v1.0.228
│       │   │   │   │   │   │   └── serde_derive feature "default"
│       │   │   │   │   │   │       └── serde_derive v1.0.228 (proc-macro)
│       │   │   │   │   │   │           ├── proc-macro2 feature "proc-macro" (*)
│       │   │   │   │   │   │           ├── quote feature "proc-macro" (*)
│       │   │   │   │   │   │           ├── syn feature "clone-impls" (*)
│       │   │   │   │   │   │           ├── syn feature "derive" (*)
│       │   │   │   │   │   │           ├── syn feature "parsing" (*)
│       │   │   │   │   │   │           ├── syn feature "printing" (*)
│       │   │   │   │   │   │           └── syn feature "proc-macro" (*)
│       │   │   │   │   │   └── serde feature "serde_derive"
│       │   │   │   │   │       └── serde v1.0.228 (*)
│       │   │   │   │   └── subtle feature "const-generics"
│       │   │   │   │       └── subtle v2.6.1
│       │   │   │   │   [build-dependencies]
│       │   │   │   │   └── rustc_version feature "default"
│       │   │   │   │       └── rustc_version v0.4.1
│       │   │   │   │           └── semver feature "default"
│       │   │   │   │               ├── semver v1.0.27
│       │   │   │   │               └── semver feature "std"
│       │   │   │   │                   └── semver v1.0.27
│       │   │   │   ├── curve25519-dalek feature "alloc"
│       │   │   │   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   │   │   └── zeroize feature "alloc"
│       │   │   │   │       └── zeroize v1.8.2 (*)
│       │   │   │   ├── curve25519-dalek feature "precomputed-tables"
│       │   │   │   │   └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   │   └── curve25519-dalek feature "zeroize"
│       │   │   │       └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   ├── curve25519-dalek feature "rand_core"
│       │   │   │   └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   ├── curve25519-dalek feature "serde"
│       │   │   │   └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   ├── curve25519-dalek feature "zeroize" (*)
│       │   │   ├── digest feature "default"
│       │   │   │   ├── digest v0.11.0-rc.10 (*)
│       │   │   │   └── digest feature "block-api" (*)
│       │   │   ├── rand_core feature "default"
│       │   │   │   └── rand_core v0.9.5
│       │   │   ├── serde feature "default"
│       │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   └── serde feature "std"
│       │   │   │       ├── serde v1.0.228 (*)
│       │   │   │       └── serde_core feature "std"
│       │   │   │           └── serde_core v1.0.228
│       │   │   ├── serde feature "derive" (*)
│       │   │   ├── serde feature "rc"
│       │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   └── serde_core feature "rc"
│       │   │   │       └── serde_core v1.0.228
│       │   │   ├── zeroize feature "default"
│       │   │   │   ├── zeroize v1.8.2 (*)
│       │   │   │   └── zeroize feature "alloc" (*)
│       │   │   ├── zeroize feature "derive"
│       │   │   │   ├── zeroize v1.8.2 (*)
│       │   │   │   └── zeroize feature "zeroize_derive"
│       │   │   │       └── zeroize v1.8.2 (*)
│       │   │   ├── zeroize_derive feature "default" (*)
│       │   │   ├── data-encoding feature "default"
│       │   │   │   ├── data-encoding v2.10.0
│       │   │   │   └── data-encoding feature "std"
│       │   │   │       ├── data-encoding v2.10.0
│       │   │   │       └── data-encoding feature "alloc"
│       │   │   │           └── data-encoding v2.10.0
│       │   │   ├── derive_more feature "debug"
│       │   │   │   ├── derive_more v2.1.1
│       │   │   │   │   └── derive_more-impl feature "default"
│       │   │   │   │       └── derive_more-impl v2.1.1 (proc-macro)
│       │   │   │   │           ├── proc-macro2 feature "default" (*)
│       │   │   │   │           ├── quote feature "default" (*)
│       │   │   │   │           ├── syn feature "default" (*)
│       │   │   │   │           ├── convert_case feature "default"
│       │   │   │   │           │   └── convert_case v0.10.0
│       │   │   │   │           │       └── unicode-segmentation feature "default"
│       │   │   │   │           │           └── unicode-segmentation v1.12.0
│       │   │   │   │           └── unicode-xid feature "default"
│       │   │   │   │               └── unicode-xid v0.2.6
│       │   │   │   │           [build-dependencies]
│       │   │   │   │           └── rustc_version feature "default" (*)
│       │   │   │   └── derive_more-impl feature "debug"
│       │   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│       │   │   │       └── syn feature "extra-traits" (*)
│       │   │   ├── derive_more feature "default"
│       │   │   │   ├── derive_more v2.1.1 (*)
│       │   │   │   └── derive_more feature "std"
│       │   │   │       └── derive_more v2.1.1 (*)
│       │   │   ├── derive_more feature "display"
│       │   │   │   ├── derive_more v2.1.1 (*)
│       │   │   │   └── derive_more-impl feature "display"
│       │   │   │       ├── derive_more-impl v2.1.1 (proc-macro) (*)
│       │   │   │       └── syn feature "extra-traits" (*)
│       │   │   ├── ed25519-dalek feature "default"
│       │   │   │   ├── ed25519-dalek v3.0.0-pre.1
│       │   │   │   │   ├── ed25519 v3.0.0-rc.4
│       │   │   │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   │   │   └── signature v3.0.0-rc.10
│       │   │   │   │   ├── rand_core v0.9.5
│       │   │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   │   ├── sha2 v0.11.0-rc.2
│       │   │   │   │   │   ├── cfg-if feature "default" (*)
│       │   │   │   │   │   ├── cpufeatures feature "default" (*)
│       │   │   │   │   │   └── digest feature "default" (*)
│       │   │   │   │   ├── subtle v2.6.1
│       │   │   │   │   ├── zeroize v1.8.2 (*)
│       │   │   │   │   └── curve25519-dalek feature "digest"
│       │   │   │   │       └── curve25519-dalek v5.0.0-pre.1 (*)
│       │   │   │   ├── ed25519-dalek feature "fast"
│       │   │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   │   │   └── curve25519-dalek feature "precomputed-tables" (*)
│       │   │   │   └── ed25519-dalek feature "zeroize"
│       │   │   │       ├── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   │       └── curve25519-dalek feature "zeroize" (*)
│       │   │   ├── ed25519-dalek feature "rand_core"
│       │   │   │   └── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   ├── ed25519-dalek feature "serde"
│       │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│       │   │   │   └── ed25519 feature "serde"
│       │   │   │       └── ed25519 v3.0.0-rc.4 (*)
│       │   │   ├── ed25519-dalek feature "zeroize" (*)
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
│       │   │   ├── n0-error feature "default"
│       │   │   │   └── n0-error v0.1.3
│       │   │   │       ├── n0-error-macros feature "default"
│       │   │   │       │   └── n0-error-macros v0.1.3 (proc-macro)
│       │   │   │       │       ├── proc-macro2 feature "default" (*)
│       │   │   │       │       ├── quote feature "default" (*)
│       │   │   │       │       ├── syn feature "default" (*)
│       │   │   │       │       ├── syn feature "extra-traits" (*)
│       │   │   │       │       └── syn feature "full" (*)
│       │   │   │       └── spez feature "default"
│       │   │   │           └── spez v0.1.2 (proc-macro)
│       │   │   │               ├── proc-macro2 feature "default" (*)
│       │   │   │               ├── quote feature "default" (*)
│       │   │   │               ├── syn feature "default" (*)
│       │   │   │               └── syn feature "full" (*)
│       │   │   ├── url feature "default"
│       │   │   │   ├── url v2.5.8
│       │   │   │   │   ├── serde v1.0.228 (*)
│       │   │   │   │   ├── serde_derive v1.0.228 (proc-macro) (*)
│       │   │   │   │   ├── form_urlencoded feature "alloc"
│       │   │   │   │   │   ├── form_urlencoded v1.2.2
│       │   │   │   │   │   │   └── percent-encoding v2.3.2
│       │   │   │   │   │   └── percent-encoding feature "alloc"
│       │   │   │   │   │       └── percent-encoding v2.3.2
│       │   │   │   │   ├── percent-encoding feature "alloc" (*)
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
│       │   │   │   │   └── idna feature "compiled_data"
│       │   │   │   │       ├── idna v1.1.0 (*)
│       │   │   │   │       └── idna_adapter feature "compiled_data"
│       │   │   │   │           ├── idna_adapter v1.2.1 (*)
│       │   │   │   │           ├── icu_normalizer feature "compiled_data"
│       │   │   │   │           │   ├── icu_normalizer v2.1.1 (*)
│       │   │   │   │           │   └── icu_provider feature "baked"
│       │   │   │   │           │       └── icu_provider v2.1.1 (*)
│       │   │   │   │           └── icu_properties feature "compiled_data"
│       │   │   │   │               ├── icu_properties v2.1.2 (*)
│       │   │   │   │               └── icu_provider feature "baked" (*)
│       │   │   │   └── url feature "std"
│       │   │   │       ├── url v2.5.8 (*)
│       │   │   │       ├── serde feature "std" (*)
│       │   │   │       ├── form_urlencoded feature "std"
│       │   │   │       │   ├── form_urlencoded v1.2.2 (*)
│       │   │   │       │   ├── form_urlencoded feature "alloc" (*)
│       │   │   │       │   └── percent-encoding feature "std"
│       │   │   │       │       ├── percent-encoding v2.3.2
│       │   │   │       │       └── percent-encoding feature "alloc" (*)
│       │   │   │       ├── percent-encoding feature "std" (*)
│       │   │   │       └── idna feature "std"
│       │   │   │           ├── idna v1.1.0 (*)
│       │   │   │           └── idna feature "alloc" (*)
│       │   │   └── url feature "serde"
│       │   │       └── url v2.5.8 (*)
│       │   └── iroh-base feature "relay"
│       │       └── iroh-base v0.97.0 (*)
│       ├── serde feature "alloc"
│       │   ├── serde v1.0.228 (*)
│       │   └── serde_core feature "alloc"
│       │       └── serde_core v1.0.228
│       ├── serde feature "derive" (*)
│       ├── postcard feature "alloc"
│       │   ├── postcard v1.1.3
│       │   │   ├── cobs v0.3.0
│       │   │   │   └── thiserror v2.0.18
│       │   │   │       └── thiserror-impl feature "default"
│       │   │   │           └── thiserror-impl v2.0.18 (proc-macro)
│       │   │   │               ├── proc-macro2 feature "default" (*)
│       │   │   │               ├── quote feature "default" (*)
│       │   │   │               └── syn feature "default" (*)
│       │   │   ├── serde feature "derive" (*)
│       │   │   └── heapless feature "serde"
│       │   │       └── heapless v0.7.17
│       │   │           ├── serde v1.0.228 (*)
│       │   │           ├── stable_deref_trait v1.2.1
│       │   │           ├── hash32 feature "default"
│       │   │           │   └── hash32 v0.2.1
│       │   │           │       └── byteorder v1.5.0
│       │   │           └── spin feature "default"
│       │   │               ├── spin v0.9.8
│       │   │               │   └── lock_api feature "default"
│       │   │               │       ├── lock_api v0.4.14
│       │   │               │       │   └── scopeguard v1.2.0
│       │   │               │       └── lock_api feature "atomic_usize"
│       │   │               │           └── lock_api v0.4.14 (*)
│       │   │               ├── spin feature "barrier"
│       │   │               │   ├── spin v0.9.8 (*)
│       │   │               │   └── spin feature "mutex"
│       │   │               │       └── spin v0.9.8 (*)
│       │   │               ├── spin feature "lazy"
│       │   │               │   ├── spin v0.9.8 (*)
│       │   │               │   └── spin feature "once"
│       │   │               │       └── spin v0.9.8 (*)
│       │   │               ├── spin feature "lock_api"
│       │   │               │   ├── spin v0.9.8 (*)
│       │   │               │   └── spin feature "lock_api_crate"
│       │   │               │       └── spin v0.9.8 (*)
│       │   │               ├── spin feature "mutex" (*)
│       │   │               ├── spin feature "once" (*)
│       │   │               ├── spin feature "rwlock"
│       │   │               │   └── spin v0.9.8 (*)
│       │   │               └── spin feature "spin_mutex"
│       │   │                   ├── spin v0.9.8 (*)
│       │   │                   └── spin feature "mutex" (*)
│       │   │           [build-dependencies]
│       │   │           └── rustc_version feature "default" (*)
│       │   └── serde feature "alloc" (*)
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
│       └── thiserror feature "default"
│           ├── thiserror v2.0.18 (*)
│           └── thiserror feature "std"
│               └── thiserror v2.0.18 (*)
├── serde feature "alloc" (*)
├── serde feature "derive" (*)
├── thiserror feature "default" (*)
├── aspen-coordination-protocol feature "default"
│   └── aspen-coordination-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination-protocol)
│       ├── serde feature "alloc" (*)
│       └── serde feature "derive" (*)
├── aspen-forge-protocol feature "default"
│   └── aspen-forge-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-forge-protocol)
│       ├── serde feature "alloc" (*)
│       └── serde feature "derive" (*)
└── aspen-jobs-protocol feature "default"
    └── aspen-jobs-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs-protocol)
        ├── serde feature "alloc" (*)
        └── serde feature "derive" (*)
[dev-dependencies]
├── postcard feature "alloc" (*)
├── insta feature "default"
│   ├── insta v1.47.2
│   │   ├── serde feature "default" (*)
│   │   ├── console feature "std"
│   │   │   ├── console v0.16.3
│   │   │   │   └── libc feature "default"
│   │   │   │       ├── libc v0.2.183
│   │   │   │       └── libc feature "std"
│   │   │   │           └── libc v0.2.183
│   │   │   └── console feature "alloc"
│   │   │       └── console v0.16.3 (*)
│   │   ├── once_cell feature "default"
│   │   │   ├── once_cell v1.21.4
│   │   │   └── once_cell feature "std"
│   │   │       ├── once_cell v1.21.4
│   │   │       └── once_cell feature "alloc"
│   │   │           ├── once_cell v1.21.4
│   │   │           └── once_cell feature "race"
│   │   │               └── once_cell v1.21.4
│   │   ├── pest feature "default"
│   │   │   ├── pest v2.8.6
│   │   │   │   ├── ucd-trie v0.1.7
│   │   │   │   └── memchr feature "default"
│   │   │   │       ├── memchr v2.8.0
│   │   │   │       └── memchr feature "std"
│   │   │   │           ├── memchr v2.8.0
│   │   │   │           └── memchr feature "alloc"
│   │   │   │               └── memchr v2.8.0
│   │   │   ├── pest feature "memchr"
│   │   │   │   └── pest v2.8.6 (*)
│   │   │   └── pest feature "std"
│   │   │       ├── pest v2.8.6 (*)
│   │   │       └── ucd-trie feature "std"
│   │   │           └── ucd-trie v0.1.7
│   │   ├── pest_derive feature "default"
│   │   │   ├── pest_derive v2.8.6 (proc-macro)
│   │   │   │   ├── pest v2.8.6 (*)
│   │   │   │   └── pest_generator v2.8.6
│   │   │   │       ├── pest v2.8.6 (*)
│   │   │   │       ├── proc-macro2 feature "default" (*)
│   │   │   │       ├── quote feature "default" (*)
│   │   │   │       ├── syn feature "default" (*)
│   │   │   │       └── pest_meta feature "default"
│   │   │   │           └── pest_meta v2.8.6
│   │   │   │               └── pest feature "default" (*)
│   │   │   │               [build-dependencies]
│   │   │   │               └── sha2 v0.10.9
│   │   │   │                   ├── cfg-if feature "default" (*)
│   │   │   │                   ├── cpufeatures feature "default" (*)
│   │   │   │                   └── digest feature "default"
│   │   │   │                       ├── digest v0.10.7
│   │   │   │                       │   ├── block-buffer feature "default"
│   │   │   │                       │   │   └── block-buffer v0.10.4
│   │   │   │                       │   │       └── generic-array feature "default"
│   │   │   │                       │   │           └── generic-array v0.14.7
│   │   │   │                       │   │               └── typenum feature "default"
│   │   │   │                       │   │                   └── typenum v1.19.0
│   │   │   │                       │   │               [build-dependencies]
│   │   │   │                       │   │               └── version_check feature "default"
│   │   │   │                       │   │                   └── version_check v0.9.5
│   │   │   │                       │   └── crypto-common feature "default"
│   │   │   │                       │       └── crypto-common v0.1.7
│   │   │   │                       │           ├── generic-array feature "default" (*)
│   │   │   │                       │           ├── generic-array feature "more_lengths"
│   │   │   │                       │           │   └── generic-array v0.14.7 (*)
│   │   │   │                       │           └── typenum feature "default" (*)
│   │   │   │                       └── digest feature "core-api"
│   │   │   │                           ├── digest v0.10.7 (*)
│   │   │   │                           └── digest feature "block-buffer"
│   │   │   │                               └── digest v0.10.7 (*)
│   │   │   └── pest_derive feature "std"
│   │   │       ├── pest_derive v2.8.6 (proc-macro) (*)
│   │   │       ├── pest feature "std" (*)
│   │   │       └── pest_generator feature "std"
│   │   │           ├── pest_generator v2.8.6 (*)
│   │   │           └── pest feature "std" (*)
│   │   ├── regex feature "std"
│   │   │   ├── regex v1.12.3
│   │   │   │   ├── regex-syntax v0.8.10
│   │   │   │   ├── regex-automata feature "alloc"
│   │   │   │   │   └── regex-automata v0.4.14
│   │   │   │   │       └── regex-syntax v0.8.10
│   │   │   │   ├── regex-automata feature "meta"
│   │   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   │   ├── regex-automata feature "nfa-pikevm"
│   │   │   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   │   │   └── regex-automata feature "nfa-thompson"
│   │   │   │   │   │       ├── regex-automata v0.4.14 (*)
│   │   │   │   │   │       └── regex-automata feature "alloc" (*)
│   │   │   │   │   └── regex-automata feature "syntax"
│   │   │   │   │       ├── regex-automata v0.4.14 (*)
│   │   │   │   │       └── regex-automata feature "alloc" (*)
│   │   │   │   ├── regex-automata feature "nfa-pikevm" (*)
│   │   │   │   └── regex-automata feature "syntax" (*)
│   │   │   ├── regex-automata feature "std"
│   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   ├── regex-automata feature "alloc" (*)
│   │   │   │   └── regex-syntax feature "std"
│   │   │   │       └── regex-syntax v0.8.10
│   │   │   └── regex-syntax feature "std" (*)
│   │   ├── regex feature "unicode"
│   │   │   ├── regex v1.12.3 (*)
│   │   │   ├── regex feature "unicode-age"
│   │   │   │   ├── regex v1.12.3 (*)
│   │   │   │   ├── regex-automata feature "unicode-age"
│   │   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   │   └── regex-syntax feature "unicode-age"
│   │   │   │   │       └── regex-syntax v0.8.10
│   │   │   │   └── regex-syntax feature "unicode-age" (*)
│   │   │   ├── regex feature "unicode-bool"
│   │   │   │   ├── regex v1.12.3 (*)
│   │   │   │   ├── regex-automata feature "unicode-bool"
│   │   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   │   └── regex-syntax feature "unicode-bool"
│   │   │   │   │       └── regex-syntax v0.8.10
│   │   │   │   └── regex-syntax feature "unicode-bool" (*)
│   │   │   ├── regex feature "unicode-case"
│   │   │   │   ├── regex v1.12.3 (*)
│   │   │   │   ├── regex-automata feature "unicode-case"
│   │   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   │   └── regex-syntax feature "unicode-case"
│   │   │   │   │       └── regex-syntax v0.8.10
│   │   │   │   └── regex-syntax feature "unicode-case" (*)
│   │   │   ├── regex feature "unicode-gencat"
│   │   │   │   ├── regex v1.12.3 (*)
│   │   │   │   ├── regex-automata feature "unicode-gencat"
│   │   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   │   └── regex-syntax feature "unicode-gencat"
│   │   │   │   │       └── regex-syntax v0.8.10
│   │   │   │   └── regex-syntax feature "unicode-gencat" (*)
│   │   │   ├── regex feature "unicode-perl"
│   │   │   │   ├── regex v1.12.3 (*)
│   │   │   │   ├── regex-automata feature "unicode-perl"
│   │   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   │   └── regex-syntax feature "unicode-perl"
│   │   │   │   │       └── regex-syntax v0.8.10
│   │   │   │   ├── regex-automata feature "unicode-word-boundary"
│   │   │   │   │   └── regex-automata v0.4.14 (*)
│   │   │   │   └── regex-syntax feature "unicode-perl" (*)
│   │   │   ├── regex feature "unicode-script"
│   │   │   │   ├── regex v1.12.3 (*)
│   │   │   │   ├── regex-automata feature "unicode-script"
│   │   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   │   └── regex-syntax feature "unicode-script"
│   │   │   │   │       └── regex-syntax v0.8.10
│   │   │   │   └── regex-syntax feature "unicode-script" (*)
│   │   │   ├── regex feature "unicode-segment"
│   │   │   │   ├── regex v1.12.3 (*)
│   │   │   │   ├── regex-automata feature "unicode-segment"
│   │   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   │   └── regex-syntax feature "unicode-segment"
│   │   │   │   │       └── regex-syntax v0.8.10
│   │   │   │   └── regex-syntax feature "unicode-segment" (*)
│   │   │   ├── regex-automata feature "unicode"
│   │   │   │   ├── regex-automata v0.4.14 (*)
│   │   │   │   ├── regex-automata feature "unicode-age" (*)
│   │   │   │   ├── regex-automata feature "unicode-bool" (*)
│   │   │   │   ├── regex-automata feature "unicode-case" (*)
│   │   │   │   ├── regex-automata feature "unicode-gencat" (*)
│   │   │   │   ├── regex-automata feature "unicode-perl" (*)
│   │   │   │   ├── regex-automata feature "unicode-script" (*)
│   │   │   │   ├── regex-automata feature "unicode-segment" (*)
│   │   │   │   ├── regex-automata feature "unicode-word-boundary" (*)
│   │   │   │   └── regex-syntax feature "unicode"
│   │   │   │       ├── regex-syntax v0.8.10
│   │   │   │       ├── regex-syntax feature "unicode-age" (*)
│   │   │   │       ├── regex-syntax feature "unicode-bool" (*)
│   │   │   │       ├── regex-syntax feature "unicode-case" (*)
│   │   │   │       ├── regex-syntax feature "unicode-gencat" (*)
│   │   │   │       ├── regex-syntax feature "unicode-perl" (*)
│   │   │   │       ├── regex-syntax feature "unicode-script" (*)
│   │   │   │       └── regex-syntax feature "unicode-segment" (*)
│   │   │   └── regex-syntax feature "unicode" (*)
│   │   ├── similar feature "default"
│   │   │   ├── similar v2.7.0
│   │   │   └── similar feature "text"
│   │   │       └── similar v2.7.0
│   │   ├── similar feature "inline"
│   │   │   ├── similar v2.7.0
│   │   │   └── similar feature "text" (*)
│   │   └── tempfile feature "default"
│   │       ├── tempfile v3.27.0
│   │       │   ├── getrandom v0.4.2
│   │       │   │   ├── libc v0.2.183
│   │       │   │   └── cfg-if feature "default" (*)
│   │       │   ├── once_cell feature "std" (*)
│   │       │   ├── fastrand feature "default"
│   │       │   │   ├── fastrand v2.3.0
│   │       │   │   └── fastrand feature "std"
│   │       │   │       ├── fastrand v2.3.0
│   │       │   │       └── fastrand feature "alloc"
│   │       │   │           └── fastrand v2.3.0
│   │       │   ├── rustix feature "default"
│   │       │   │   ├── rustix v1.1.4
│   │       │   │   │   ├── bitflags v2.11.0
│   │       │   │   │   ├── linux-raw-sys feature "auxvec"
│   │       │   │   │   │   └── linux-raw-sys v0.12.1
│   │       │   │   │   ├── linux-raw-sys feature "elf"
│   │       │   │   │   │   └── linux-raw-sys v0.12.1
│   │       │   │   │   ├── linux-raw-sys feature "errno"
│   │       │   │   │   │   └── linux-raw-sys v0.12.1
│   │       │   │   │   ├── linux-raw-sys feature "general"
│   │       │   │   │   │   └── linux-raw-sys v0.12.1
│   │       │   │   │   ├── linux-raw-sys feature "ioctl"
│   │       │   │   │   │   └── linux-raw-sys v0.12.1
│   │       │   │   │   └── linux-raw-sys feature "no_std"
│   │       │   │   │       └── linux-raw-sys v0.12.1
│   │       │   │   └── rustix feature "std"
│   │       │   │       ├── rustix v1.1.4 (*)
│   │       │   │       ├── rustix feature "alloc"
│   │       │   │       │   └── rustix v1.1.4 (*)
│   │       │   │       └── bitflags feature "std"
│   │       │   │           └── bitflags v2.11.0
│   │       │   └── rustix feature "fs"
│   │       │       └── rustix v1.1.4 (*)
│   │       └── tempfile feature "getrandom"
│   │           └── tempfile v3.27.0 (*)
│   └── insta feature "colors"
│       ├── insta v1.47.2 (*)
│       └── insta feature "console"
│           └── insta v1.47.2 (*)
├── insta feature "filters"
│   ├── insta v1.47.2 (*)
│   └── insta feature "regex"
│       └── insta v1.47.2 (*)
├── insta feature "json"
│   ├── insta v1.47.2 (*)
│   └── insta feature "serde"
│       └── insta v1.47.2 (*)
├── insta feature "redactions"
│   ├── insta v1.47.2 (*)
│   ├── insta feature "pest"
│   │   └── insta v1.47.2 (*)
│   ├── insta feature "pest_derive"
│   │   └── insta v1.47.2 (*)
│   └── insta feature "serde" (*)
├── serde_json feature "alloc"
│   ├── serde_json v1.0.149
│   │   ├── memchr v2.8.0
│   │   ├── serde_core v1.0.228
│   │   ├── itoa feature "default"
│   │   │   └── itoa v1.0.17
│   │   └── zmij feature "default"
│   │       └── zmij v1.0.21
│   └── serde_core feature "alloc" (*)
├── tokio feature "default"
│   └── tokio v1.50.0
│       ├── pin-project-lite feature "default"
│       │   └── pin-project-lite v0.2.17
│       └── tokio-macros feature "default"
│           └── tokio-macros v2.6.1 (proc-macro)
│               ├── proc-macro2 feature "default" (*)
│               ├── quote feature "default" (*)
│               ├── syn feature "default" (*)
│               └── syn feature "full" (*)
├── tokio feature "macros"
│   ├── tokio v1.50.0 (*)
│   └── tokio feature "tokio-macros"
│       └── tokio v1.50.0 (*)
└── tokio feature "rt"
    └── tokio v1.50.0 (*)
```

## `cargo tree -p aspen-coordination-protocol -e normal`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-coordination-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination-protocol)
└── serde v1.0.228
    ├── serde_core v1.0.228
    └── serde_derive v1.0.228 (proc-macro)
        ├── proc-macro2 v1.0.106
        │   └── unicode-ident v1.0.24
        ├── quote v1.0.45
        │   └── proc-macro2 v1.0.106 (*)
        └── syn v2.0.117
            ├── proc-macro2 v1.0.106 (*)
            ├── quote v1.0.45 (*)
            └── unicode-ident v1.0.24
```

## `cargo tree -p aspen-coordination-protocol -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-coordination-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination-protocol)
├── serde feature "alloc"
│   ├── serde v1.0.228
│   │   ├── serde_core feature "result"
│   │   │   └── serde_core v1.0.228
│   │   └── serde_derive feature "default"
│   │       └── serde_derive v1.0.228 (proc-macro)
│   │           ├── proc-macro2 feature "proc-macro"
│   │           │   └── proc-macro2 v1.0.106
│   │           │       └── unicode-ident feature "default"
│   │           │           └── unicode-ident v1.0.24
│   │           ├── quote feature "proc-macro"
│   │           │   ├── quote v1.0.45
│   │           │   │   └── proc-macro2 v1.0.106 (*)
│   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "clone-impls"
│   │           │   └── syn v2.0.117
│   │           │       ├── proc-macro2 v1.0.106 (*)
│   │           │       ├── quote v1.0.45 (*)
│   │           │       └── unicode-ident feature "default" (*)
│   │           ├── syn feature "derive"
│   │           │   └── syn v2.0.117 (*)
│   │           ├── syn feature "parsing"
│   │           │   └── syn v2.0.117 (*)
│   │           ├── syn feature "printing"
│   │           │   └── syn v2.0.117 (*)
│   │           └── syn feature "proc-macro"
│   │               ├── syn v2.0.117 (*)
│   │               ├── proc-macro2 feature "proc-macro" (*)
│   │               └── quote feature "proc-macro" (*)
│   └── serde_core feature "alloc"
│       └── serde_core v1.0.228
└── serde feature "derive"
    ├── serde v1.0.228 (*)
    └── serde feature "serde_derive"
        └── serde v1.0.228 (*)
[dev-dependencies]
├── postcard feature "alloc"
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18
│   │   │       └── thiserror-impl feature "default"
│   │   │           └── thiserror-impl v2.0.18 (proc-macro)
│   │   │               ├── proc-macro2 feature "default"
│   │   │               │   ├── proc-macro2 v1.0.106 (*)
│   │   │               │   └── proc-macro2 feature "proc-macro" (*)
│   │   │               ├── quote feature "default"
│   │   │               │   ├── quote v1.0.45 (*)
│   │   │               │   └── quote feature "proc-macro" (*)
│   │   │               └── syn feature "default"
│   │   │                   ├── syn v2.0.117 (*)
│   │   │                   ├── syn feature "clone-impls" (*)
│   │   │                   ├── syn feature "derive" (*)
│   │   │                   ├── syn feature "parsing" (*)
│   │   │                   ├── syn feature "printing" (*)
│   │   │                   └── syn feature "proc-macro" (*)
│   │   └── serde feature "derive" (*)
│   └── serde feature "alloc" (*)
└── serde_json feature "alloc"
    ├── serde_json v1.0.149
    │   ├── memchr v2.8.0
    │   ├── serde_core v1.0.228
    │   ├── itoa feature "default"
    │   │   └── itoa v1.0.17
    │   └── zmij feature "default"
    │       └── zmij v1.0.21
    └── serde_core feature "alloc" (*)
```

## `cargo tree -p aspen-jobs-protocol -e normal`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-jobs-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs-protocol)
└── serde v1.0.228
    ├── serde_core v1.0.228
    └── serde_derive v1.0.228 (proc-macro)
        ├── proc-macro2 v1.0.106
        │   └── unicode-ident v1.0.24
        ├── quote v1.0.45
        │   └── proc-macro2 v1.0.106 (*)
        └── syn v2.0.117
            ├── proc-macro2 v1.0.106 (*)
            ├── quote v1.0.45 (*)
            └── unicode-ident v1.0.24
```

## `cargo tree -p aspen-jobs-protocol -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-jobs-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs-protocol)
├── serde feature "alloc"
│   ├── serde v1.0.228
│   │   ├── serde_core feature "result"
│   │   │   └── serde_core v1.0.228
│   │   └── serde_derive feature "default"
│   │       └── serde_derive v1.0.228 (proc-macro)
│   │           ├── proc-macro2 feature "proc-macro"
│   │           │   └── proc-macro2 v1.0.106
│   │           │       └── unicode-ident feature "default"
│   │           │           └── unicode-ident v1.0.24
│   │           ├── quote feature "proc-macro"
│   │           │   ├── quote v1.0.45
│   │           │   │   └── proc-macro2 v1.0.106 (*)
│   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "clone-impls"
│   │           │   └── syn v2.0.117
│   │           │       ├── proc-macro2 v1.0.106 (*)
│   │           │       ├── quote v1.0.45 (*)
│   │           │       └── unicode-ident feature "default" (*)
│   │           ├── syn feature "derive"
│   │           │   └── syn v2.0.117 (*)
│   │           ├── syn feature "parsing"
│   │           │   └── syn v2.0.117 (*)
│   │           ├── syn feature "printing"
│   │           │   └── syn v2.0.117 (*)
│   │           └── syn feature "proc-macro"
│   │               ├── syn v2.0.117 (*)
│   │               ├── proc-macro2 feature "proc-macro" (*)
│   │               └── quote feature "proc-macro" (*)
│   └── serde_core feature "alloc"
│       └── serde_core v1.0.228
└── serde feature "derive"
    ├── serde v1.0.228 (*)
    └── serde feature "serde_derive"
        └── serde v1.0.228 (*)
[dev-dependencies]
├── postcard feature "alloc"
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18
│   │   │       └── thiserror-impl feature "default"
│   │   │           └── thiserror-impl v2.0.18 (proc-macro)
│   │   │               ├── proc-macro2 feature "default"
│   │   │               │   ├── proc-macro2 v1.0.106 (*)
│   │   │               │   └── proc-macro2 feature "proc-macro" (*)
│   │   │               ├── quote feature "default"
│   │   │               │   ├── quote v1.0.45 (*)
│   │   │               │   └── quote feature "proc-macro" (*)
│   │   │               └── syn feature "default"
│   │   │                   ├── syn v2.0.117 (*)
│   │   │                   ├── syn feature "clone-impls" (*)
│   │   │                   ├── syn feature "derive" (*)
│   │   │                   ├── syn feature "parsing" (*)
│   │   │                   ├── syn feature "printing" (*)
│   │   │                   └── syn feature "proc-macro" (*)
│   │   └── serde feature "derive" (*)
│   └── serde feature "alloc" (*)
└── serde_json feature "alloc"
    ├── serde_json v1.0.149
    │   ├── memchr v2.8.0
    │   ├── serde_core v1.0.228
    │   ├── itoa feature "default"
    │   │   └── itoa v1.0.17
    │   └── zmij feature "default"
    │       └── zmij v1.0.21
    └── serde_core feature "alloc" (*)
```

## `cargo tree -p aspen-forge-protocol -e normal`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-forge-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-forge-protocol)
└── serde v1.0.228
    ├── serde_core v1.0.228
    └── serde_derive v1.0.228 (proc-macro)
        ├── proc-macro2 v1.0.106
        │   └── unicode-ident v1.0.24
        ├── quote v1.0.45
        │   └── proc-macro2 v1.0.106 (*)
        └── syn v2.0.117
            ├── proc-macro2 v1.0.106 (*)
            ├── quote v1.0.45 (*)
            └── unicode-ident v1.0.24
```

## `cargo tree -p aspen-forge-protocol -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-forge-protocol v0.1.0 (/home/brittonr/git/aspen/crates/aspen-forge-protocol)
├── serde feature "alloc"
│   ├── serde v1.0.228
│   │   ├── serde_core feature "result"
│   │   │   └── serde_core v1.0.228
│   │   └── serde_derive feature "default"
│   │       └── serde_derive v1.0.228 (proc-macro)
│   │           ├── proc-macro2 feature "proc-macro"
│   │           │   └── proc-macro2 v1.0.106
│   │           │       └── unicode-ident feature "default"
│   │           │           └── unicode-ident v1.0.24
│   │           ├── quote feature "proc-macro"
│   │           │   ├── quote v1.0.45
│   │           │   │   └── proc-macro2 v1.0.106 (*)
│   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "clone-impls"
│   │           │   └── syn v2.0.117
│   │           │       ├── proc-macro2 v1.0.106 (*)
│   │           │       ├── quote v1.0.45 (*)
│   │           │       └── unicode-ident feature "default" (*)
│   │           ├── syn feature "derive"
│   │           │   └── syn v2.0.117 (*)
│   │           ├── syn feature "parsing"
│   │           │   └── syn v2.0.117 (*)
│   │           ├── syn feature "printing"
│   │           │   └── syn v2.0.117 (*)
│   │           └── syn feature "proc-macro"
│   │               ├── syn v2.0.117 (*)
│   │               ├── proc-macro2 feature "proc-macro" (*)
│   │               └── quote feature "proc-macro" (*)
│   └── serde_core feature "alloc"
│       └── serde_core v1.0.228
└── serde feature "derive"
    ├── serde v1.0.228 (*)
    └── serde feature "serde_derive"
        └── serde v1.0.228 (*)
[dev-dependencies]
└── serde_json feature "alloc"
    ├── serde_json v1.0.149
    │   ├── memchr v2.8.0
    │   ├── serde_core v1.0.228
    │   ├── itoa feature "default"
    │   │   └── itoa v1.0.17
    │   └── zmij feature "default"
    │       └── zmij v1.0.21
    └── serde_core feature "alloc" (*)
```

## `python3 scripts/check-foundation-wire-deps.py --mode wire`

```text
PASS aspen-client-api keeps serde_json out of normal dependencies
PASS aspen-client-api no-default-features graph excludes anyhow, aspen-disk, aspen-layer, aspen-time, chrono, iroh, iroh-base, iroh-blobs, libc, n0-future, rand, redb, serde_json, tokio, tokio-util, tracing
PASS aspen-coordination-protocol keeps serde_json out of normal dependencies
PASS aspen-coordination-protocol no-default-features graph excludes anyhow, aspen-disk, aspen-layer, aspen-time, chrono, iroh, iroh-base, iroh-blobs, libc, n0-future, rand, redb, serde_json, tokio, tokio-util, tracing
PASS aspen-jobs-protocol keeps serde_json out of normal dependencies
PASS aspen-jobs-protocol no-default-features graph excludes anyhow, aspen-disk, aspen-layer, aspen-time, chrono, iroh, iroh-base, iroh-blobs, libc, n0-future, rand, redb, serde_json, tokio, tokio-util, tracing
PASS aspen-forge-protocol keeps serde_json out of normal dependencies
PASS aspen-forge-protocol no-default-features graph excludes anyhow, aspen-disk, aspen-layer, aspen-time, chrono, iroh, iroh-base, iroh-blobs, libc, n0-future, rand, redb, serde_json, tokio, tokio-util, tracing
SUMMARY ok
```

## `python3 scripts/check-foundation-wire-source-audits.py --mode wire`

```text
PASS crates/aspen-client-api/src excludes forbidden helpers outside tests
PASS crates/aspen-coordination-protocol/src excludes forbidden helpers outside tests
PASS crates/aspen-jobs-protocol/src excludes forbidden helpers outside tests
PASS crates/aspen-forge-protocol/src excludes forbidden helpers outside tests
SUMMARY ok
```

## `cargo check -p aspen-cluster`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
    Checking aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft)
    Checking aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.03s
```

## `cargo check -p aspen-client`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-client-api v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client-api)
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.32s
```

## `cargo check -p aspen-cli`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft)
    Checking aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
    Checking aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli)
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.12s
```

## `cargo check -p aspen-rpc-handlers`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-rpc-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-rpc-core)
    Checking aspen-core-essentials-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-essentials-handler)
    Checking aspen-cluster-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-handler)
    Checking aspen-rpc-handlers v0.1.0 (/home/brittonr/git/aspen/crates/aspen-rpc-handlers)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.49s
```

## `cargo check -p aspen --no-default-features --features node-runtime`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft)
    Checking aspen-rpc-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-rpc-core)
    Checking aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
    Checking aspen-cluster-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-handler)
    Checking aspen-core-essentials-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-essentials-handler)
    Checking aspen-rpc-handlers v0.1.0 (/home/brittonr/git/aspen/crates/aspen-rpc-handlers)
    Checking aspen v0.1.0 (/home/brittonr/git/aspen)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.60s
```
