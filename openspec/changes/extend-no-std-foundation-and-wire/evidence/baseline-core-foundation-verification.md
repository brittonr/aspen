Evidence-ID: extend-no-std-foundation-and-wire.r1-core
Task-ID: R1
Artifact-Type: baseline-verification
Covers: core.no-std-core-baseline.compile-slice-verification-is-reviewable, architecture.modularity.acyclic-no-std-core-boundary.leaf-crate-verification-is-reviewable, architecture.modularity.feature-bundles-are-explicit-and-bounded.dependency-boundary-is-checked-deterministically

# Baseline core foundation verification

- Baseline source commit: `f1c02f9f5c34f3d3f6218a26668b36157618c9ce`
- Final rail logic source commit: `4f0c9801e`
- Baseline source captured in a standalone git clone at `/tmp/aspen-no-std-baseline-root.ETpnVI/aspen`
- Final rail logic scripts copied from current HEAD into the baseline snapshot before second-pass checks.

## wasm32 target setup

Host setup record:

```text
- rustup unavailable in this task environment; target availability proved by saved `cargo check --target wasm32-unknown-unknown` command results below.
```

## `cargo check -p aspen-traits`

```text
$ cargo check -p aspen-traits
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling serde_core v1.0.228
   Compiling thiserror v2.0.18
   Compiling serde v1.0.228
    Checking aspen-constants v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-constants)
   Compiling syn v2.0.117
   Compiling serde_derive v1.0.228
   Compiling thiserror-impl v2.0.18
   Compiling async-trait v0.1.89
    Checking aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types)
    Checking aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types)
    Checking aspen-traits v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.47s

[exit status: 0]
```

## `cargo check -p aspen-traits --no-default-features`

```text
$ cargo check -p aspen-traits --no-default-features
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s

[exit status: 0]
```

## `cargo check -p aspen-traits --no-default-features --target wasm32-unknown-unknown`

```text
$ cargo check -p aspen-traits --no-default-features --target wasm32-unknown-unknown
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling serde_core v1.0.228
   Compiling thiserror v2.0.18
   Compiling serde v1.0.228
    Checking aspen-constants v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-constants)
   Compiling syn v2.0.117
   Compiling thiserror-impl v2.0.18
   Compiling serde_derive v1.0.228
   Compiling async-trait v0.1.89
    Checking aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types)
    Checking aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types)
    Checking aspen-traits v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.19s

[exit status: 0]
```

## `cargo tree -p aspen-traits -e normal`

```text
$ cargo tree -p aspen-traits -e normal
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
aspen-traits v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits)
├── aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types)
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
├── aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-constants)
│   ├── serde v1.0.228 (*)
│   └── thiserror v2.0.18 (*)
└── async-trait v0.1.89 (proc-macro)
    ├── proc-macro2 v1.0.106 (*)
    ├── quote v1.0.45 (*)
    └── syn v2.0.117 (*)

[exit status: 0]
```

## `cargo tree -p aspen-traits -e features`

```text
$ cargo tree -p aspen-traits -e features
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
aspen-traits v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits)
├── aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types)
│   ├── serde feature "alloc"
│   │   ├── serde v1.0.228
│   │   │   ├── serde_core feature "result"
│   │   │   │   └── serde_core v1.0.228
│   │   │   └── serde_derive feature "default"
│   │   │       └── serde_derive v1.0.228 (proc-macro)
│   │   │           ├── proc-macro2 feature "proc-macro"
│   │   │           │   └── proc-macro2 v1.0.106
│   │   │           │       └── unicode-ident feature "default"
│   │   │           │           └── unicode-ident v1.0.24
│   │   │           ├── quote feature "proc-macro"
│   │   │           │   ├── quote v1.0.45
│   │   │           │   │   └── proc-macro2 v1.0.106 (*)
│   │   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │   │           ├── syn feature "clone-impls"
│   │   │           │   └── syn v2.0.117
│   │   │           │       ├── proc-macro2 v1.0.106 (*)
│   │   │           │       ├── quote v1.0.45 (*)
│   │   │           │       └── unicode-ident feature "default" (*)
│   │   │           ├── syn feature "derive"
│   │   │           │   └── syn v2.0.117 (*)
│   │   │           ├── syn feature "parsing"
│   │   │           │   └── syn v2.0.117 (*)
│   │   │           ├── syn feature "printing"
│   │   │           │   └── syn v2.0.117 (*)
│   │   │           └── syn feature "proc-macro"
│   │   │               ├── syn v2.0.117 (*)
│   │   │               ├── proc-macro2 feature "proc-macro" (*)
│   │   │               └── quote feature "proc-macro" (*)
│   │   └── serde_core feature "alloc"
│   │       └── serde_core v1.0.228
│   ├── serde feature "derive"
│   │   ├── serde v1.0.228 (*)
│   │   └── serde feature "serde_derive"
│   │       └── serde v1.0.228 (*)
│   └── thiserror feature "default"
│       ├── thiserror v2.0.18
│       │   └── thiserror-impl feature "default"
│       │       └── thiserror-impl v2.0.18 (proc-macro)
│       │           ├── proc-macro2 feature "default"
│       │           │   ├── proc-macro2 v1.0.106 (*)
│       │           │   └── proc-macro2 feature "proc-macro" (*)
│       │           ├── quote feature "default"
│       │           │   ├── quote v1.0.45 (*)
│       │           │   └── quote feature "proc-macro" (*)
│       │           └── syn feature "default"
│       │               ├── syn v2.0.117 (*)
│       │               ├── syn feature "clone-impls" (*)
│       │               ├── syn feature "derive" (*)
│       │               ├── syn feature "parsing" (*)
│       │               ├── syn feature "printing" (*)
│       │               └── syn feature "proc-macro" (*)
│       └── thiserror feature "std"
│           └── thiserror v2.0.18 (*)
├── aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-constants)
│   ├── serde feature "alloc" (*)
│   ├── serde feature "derive" (*)
│   └── thiserror feature "default" (*)
└── async-trait feature "default"
    └── async-trait v0.1.89 (proc-macro)
        ├── proc-macro2 feature "default" (*)
        ├── quote feature "default" (*)
        ├── syn feature "clone-impls" (*)
        ├── syn feature "full"
        │   └── syn v2.0.117 (*)
        ├── syn feature "parsing" (*)
        ├── syn feature "printing" (*)
        ├── syn feature "proc-macro" (*)
        └── syn feature "visit-mut"
            └── syn v2.0.117 (*)
[dev-dependencies]
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
└── tokio feature "rt-multi-thread"
    ├── tokio v1.50.0 (*)
    └── tokio feature "rt"
        └── tokio v1.50.0 (*)

[exit status: 0]
```

## `cargo tree -p aspen-traits -e features -i aspen-cluster-types`

```text
$ cargo tree -p aspen-traits -e features -i aspen-cluster-types
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types)
└── aspen-traits v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits)
    └── aspen-traits feature "default" (command-line)

[exit status: 0]
```

## `cargo check -p aspen-core`

```text
$ cargo check -p aspen-core
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
   Compiling shlex v1.3.0
   Compiling find-msvc-tools v0.1.9
    Checking arrayvec v0.7.6
    Checking cpufeatures v0.2.17
    Checking arrayref v0.3.9
    Checking spin v0.10.0
    Checking cfg-if v1.0.4
    Checking constant_time_eq v0.4.2
   Compiling heck v0.5.0
    Checking base64 v0.22.1
    Checking hex v0.4.3
    Checking serde v1.0.228
   Compiling cc v1.2.57
   Compiling snafu-derive v0.8.9
    Checking bincode v1.3.3
    Checking aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types)
    Checking aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types)
    Checking uhlc v0.8.2 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/uhlc)
    Checking aspen-storage-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types)
   Compiling blake3 v1.8.3
    Checking aspen-traits v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits)
    Checking snafu v0.8.9
    Checking aspen-hlc v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-hlc)
    Checking aspen-core v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.33s

[exit status: 0]
```

## `cargo check -p aspen-core --no-default-features`

```text
$ cargo check -p aspen-core --no-default-features
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
    Checking aspen-core v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.29s

[exit status: 0]
```

## `cargo check -p aspen-core-no-std-smoke`

```text
$ cargo check -p aspen-core-no-std-smoke
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
    Checking aspen-core-no-std-smoke v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core-no-std-smoke)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s

[exit status: 0]
```

## `cargo tree -p aspen-core --no-default-features -e normal --depth 1`

```text
$ cargo tree -p aspen-core --no-default-features -e normal --depth 1
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
aspen-core v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core)
├── aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types)
├── aspen-constants v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-constants)
├── aspen-hlc v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-hlc)
├── aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types)
├── aspen-storage-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types)
├── aspen-traits v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits)
├── async-trait v0.1.89 (proc-macro)
├── base64 v0.22.1
├── bincode v1.3.3
├── hex v0.4.3
├── serde v1.0.228
├── snafu v0.8.9
└── thiserror v2.0.18

[exit status: 0]
```

## `cargo tree -p aspen-core --no-default-features -e normal`

```text
$ cargo tree -p aspen-core --no-default-features -e normal
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
aspen-core v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core)
├── aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types)
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
├── aspen-constants v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-constants)
├── aspen-hlc v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-hlc)
│   ├── blake3 v1.8.3
│   │   ├── arrayref v0.3.9
│   │   ├── arrayvec v0.7.6
│   │   ├── cfg-if v1.0.4
│   │   ├── constant_time_eq v0.4.2
│   │   └── cpufeatures v0.2.17
│   ├── serde v1.0.228 (*)
│   └── uhlc v0.8.2 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/uhlc)
│       ├── serde v1.0.228 (*)
│       └── spin v0.10.0
├── aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-constants)
│   ├── serde v1.0.228 (*)
│   └── thiserror v2.0.18 (*)
├── aspen-storage-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types)
│   ├── bincode v1.3.3
│   │   └── serde v1.0.228 (*)
│   ├── redb v2.6.3
│   │   └── libc v0.2.183
│   └── serde v1.0.228 (*)
├── aspen-traits v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits)
│   ├── aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types) (*)
│   ├── aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types) (*)
│   └── async-trait v0.1.89 (proc-macro)
│       ├── proc-macro2 v1.0.106 (*)
│       ├── quote v1.0.45 (*)
│       └── syn v2.0.117 (*)
├── async-trait v0.1.89 (proc-macro) (*)
├── base64 v0.22.1
├── bincode v1.3.3 (*)
├── hex v0.4.3
├── serde v1.0.228 (*)
├── snafu v0.8.9
│   └── snafu-derive v0.8.9 (proc-macro)
│       ├── heck v0.5.0
│       ├── proc-macro2 v1.0.106 (*)
│       ├── quote v1.0.45 (*)
│       └── syn v2.0.117 (*)
└── thiserror v2.0.18 (*)

[exit status: 0]
```

## `cargo tree -p aspen-core --no-default-features -e features`

```text
$ cargo tree -p aspen-core --no-default-features -e features
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
aspen-core v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core)
├── aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types)
│   ├── serde feature "alloc"
│   │   ├── serde v1.0.228
│   │   │   ├── serde_core feature "result"
│   │   │   │   └── serde_core v1.0.228
│   │   │   └── serde_derive feature "default"
│   │   │       └── serde_derive v1.0.228 (proc-macro)
│   │   │           ├── proc-macro2 feature "proc-macro"
│   │   │           │   └── proc-macro2 v1.0.106
│   │   │           │       └── unicode-ident feature "default"
│   │   │           │           └── unicode-ident v1.0.24
│   │   │           ├── quote feature "proc-macro"
│   │   │           │   ├── quote v1.0.45
│   │   │           │   │   └── proc-macro2 v1.0.106 (*)
│   │   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │   │           ├── syn feature "clone-impls"
│   │   │           │   └── syn v2.0.117
│   │   │           │       ├── proc-macro2 v1.0.106 (*)
│   │   │           │       ├── quote v1.0.45 (*)
│   │   │           │       └── unicode-ident feature "default" (*)
│   │   │           ├── syn feature "derive"
│   │   │           │   └── syn v2.0.117 (*)
│   │   │           ├── syn feature "parsing"
│   │   │           │   └── syn v2.0.117 (*)
│   │   │           ├── syn feature "printing"
│   │   │           │   └── syn v2.0.117 (*)
│   │   │           └── syn feature "proc-macro"
│   │   │               ├── syn v2.0.117 (*)
│   │   │               ├── proc-macro2 feature "proc-macro" (*)
│   │   │               └── quote feature "proc-macro" (*)
│   │   └── serde_core feature "alloc"
│   │       └── serde_core v1.0.228
│   ├── serde feature "derive"
│   │   ├── serde v1.0.228 (*)
│   │   └── serde feature "serde_derive"
│   │       └── serde v1.0.228 (*)
│   └── thiserror feature "default"
│       ├── thiserror v2.0.18
│       │   └── thiserror-impl feature "default"
│       │       └── thiserror-impl v2.0.18 (proc-macro)
│       │           ├── proc-macro2 feature "default"
│       │           │   ├── proc-macro2 v1.0.106 (*)
│       │           │   └── proc-macro2 feature "proc-macro" (*)
│       │           ├── quote feature "default"
│       │           │   ├── quote v1.0.45 (*)
│       │           │   └── quote feature "proc-macro" (*)
│       │           └── syn feature "default"
│       │               ├── syn v2.0.117 (*)
│       │               ├── syn feature "clone-impls" (*)
│       │               ├── syn feature "derive" (*)
│       │               ├── syn feature "parsing" (*)
│       │               ├── syn feature "printing" (*)
│       │               └── syn feature "proc-macro" (*)
│       └── thiserror feature "std"
│           └── thiserror v2.0.18 (*)
├── aspen-constants v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-constants)
├── aspen-hlc v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-hlc)
│   ├── blake3 v1.8.3
│   │   ├── arrayvec v0.7.6
│   │   ├── constant_time_eq v0.4.2
│   │   ├── arrayref feature "default"
│   │   │   └── arrayref v0.3.9
│   │   ├── cfg-if feature "default"
│   │   │   └── cfg-if v1.0.4
│   │   └── cpufeatures feature "default"
│   │       └── cpufeatures v0.2.17
│   │   [build-dependencies]
│   │   └── cc feature "default"
│   │       └── cc v1.2.57
│   │           ├── find-msvc-tools feature "default"
│   │           │   └── find-msvc-tools v0.1.9
│   │           └── shlex feature "default"
│   │               ├── shlex v1.3.0
│   │               └── shlex feature "std"
│   │                   └── shlex v1.3.0
│   ├── uhlc v0.8.2 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/uhlc)
│   │   ├── serde feature "alloc" (*)
│   │   ├── serde feature "derive" (*)
│   │   ├── spin feature "mutex"
│   │   │   └── spin v0.10.0
│   │   └── spin feature "spin_mutex"
│   │       ├── spin v0.10.0
│   │       └── spin feature "mutex" (*)
│   ├── serde feature "alloc" (*)
│   └── serde feature "derive" (*)
├── aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-constants)
│   ├── serde feature "alloc" (*)
│   ├── serde feature "derive" (*)
│   └── thiserror feature "default" (*)
├── aspen-storage-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types)
│   ├── bincode v1.3.3
│   │   └── serde feature "default"
│   │       ├── serde v1.0.228 (*)
│   │       └── serde feature "std"
│   │           ├── serde v1.0.228 (*)
│   │           └── serde_core feature "std"
│   │               └── serde_core v1.0.228
│   ├── serde feature "alloc" (*)
│   ├── serde feature "derive" (*)
│   └── redb feature "default"
│       └── redb v2.6.3
│           └── libc feature "default"
│               ├── libc v0.2.183
│               └── libc feature "std"
│                   └── libc v0.2.183
├── aspen-traits v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits)
│   ├── aspen-cluster-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-cluster-types) (*)
│   ├── aspen-kv-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-kv-types) (*)
│   └── async-trait feature "default"
│       └── async-trait v0.1.89 (proc-macro)
│           ├── proc-macro2 feature "default" (*)
│           ├── quote feature "default" (*)
│           ├── syn feature "clone-impls" (*)
│           ├── syn feature "full"
│           │   └── syn v2.0.117 (*)
│           ├── syn feature "parsing" (*)
│           ├── syn feature "printing" (*)
│           ├── syn feature "proc-macro" (*)
│           └── syn feature "visit-mut"
│               └── syn v2.0.117 (*)
├── bincode v1.3.3 (*)
├── serde feature "alloc" (*)
├── serde feature "derive" (*)
├── thiserror feature "default" (*)
├── async-trait feature "default" (*)
├── base64 feature "alloc"
│   └── base64 v0.22.1
├── hex feature "alloc"
│   └── hex v0.4.3
└── snafu feature "rust_1_65"
    ├── snafu v0.8.9
    │   └── snafu-derive feature "default"
    │       └── snafu-derive v0.8.9 (proc-macro)
    │           ├── heck v0.5.0
    │           ├── proc-macro2 feature "default" (*)
    │           ├── quote feature "default" (*)
    │           ├── syn feature "default" (*)
    │           └── syn feature "full" (*)
    └── snafu feature "rust_1_61"
        ├── snafu v0.8.9 (*)
        └── snafu-derive feature "rust_1_61"
            └── snafu-derive v0.8.9 (proc-macro) (*)
[dev-dependencies]
├── insta feature "default"
│   ├── insta v1.47.2
│   │   ├── console feature "std"
│   │   │   ├── console v0.16.3
│   │   │   │   └── libc feature "default" (*)
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
├── postcard feature "alloc"
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18 (*)
│   │   ├── serde feature "derive" (*)
│   │   └── heapless feature "serde"
│   │       └── heapless v0.7.17
│   │           ├── serde v1.0.228 (*)
│   │           ├── stable_deref_trait v1.2.1
│   │           ├── hash32 feature "default"
│   │           │   └── hash32 v0.2.1
│   │           │       └── byteorder v1.5.0
│   │           └── spin feature "default"
│   │               ├── spin v0.9.8
│   │               │   └── lock_api feature "default"
│   │               │       ├── lock_api v0.4.14
│   │               │       │   └── scopeguard v1.2.0
│   │               │       └── lock_api feature "atomic_usize"
│   │               │           └── lock_api v0.4.14 (*)
│   │               ├── spin feature "barrier"
│   │               │   ├── spin v0.9.8 (*)
│   │               │   └── spin feature "mutex"
│   │               │       └── spin v0.9.8 (*)
│   │               ├── spin feature "lazy"
│   │               │   ├── spin v0.9.8 (*)
│   │               │   └── spin feature "once"
│   │               │       └── spin v0.9.8 (*)
│   │               ├── spin feature "lock_api"
│   │               │   ├── spin v0.9.8 (*)
│   │               │   └── spin feature "lock_api_crate"
│   │               │       └── spin v0.9.8 (*)
│   │               ├── spin feature "mutex" (*)
│   │               ├── spin feature "once" (*)
│   │               ├── spin feature "rwlock"
│   │               │   └── spin v0.9.8 (*)
│   │               └── spin feature "spin_mutex"
│   │                   ├── spin v0.9.8 (*)
│   │                   └── spin feature "mutex" (*)
│   │           [build-dependencies]
│   │           └── rustc_version feature "default"
│   │               └── rustc_version v0.4.1
│   │                   └── semver feature "default"
│   │                       ├── semver v1.0.27
│   │                       └── semver feature "std"
│   │                           └── semver v1.0.27
│   └── serde feature "alloc" (*)
├── postcard feature "default"
│   ├── postcard v1.1.3 (*)
│   └── postcard feature "heapless-cas"
│       ├── postcard v1.1.3 (*)
│       ├── postcard feature "heapless"
│       │   └── postcard v1.1.3 (*)
│       └── heapless feature "cas"
│           ├── heapless v0.7.17 (*)
│           └── heapless feature "atomic-polyfill"
│               └── heapless v0.7.17 (*)
├── proptest feature "default"
│   ├── proptest v1.10.0
│   │   ├── num-traits v0.2.19
│   │   │   [build-dependencies]
│   │   │   └── autocfg feature "default"
│   │   │       └── autocfg v1.5.0
│   │   ├── rand_chacha v0.9.0
│   │   │   ├── rand_core feature "default"
│   │   │   │   └── rand_core v0.9.5
│   │   │   │       └── getrandom feature "default"
│   │   │   │           └── getrandom v0.3.4
│   │   │   │               ├── libc v0.2.183
│   │   │   │               └── cfg-if feature "default" (*)
│   │   │   └── ppv-lite86 feature "simd"
│   │   │       └── ppv-lite86 v0.2.21
│   │   │           ├── zerocopy feature "default"
│   │   │           │   └── zerocopy v0.8.42
│   │   │           └── zerocopy feature "simd"
│   │   │               └── zerocopy v0.8.42
│   │   ├── rusty-fork v0.3.1
│   │   │   ├── tempfile feature "default" (*)
│   │   │   ├── fnv feature "default"
│   │   │   │   ├── fnv v1.0.7
│   │   │   │   └── fnv feature "std"
│   │   │   │       └── fnv v1.0.7
│   │   │   ├── quick-error feature "default"
│   │   │   │   └── quick-error v1.2.3
│   │   │   └── wait-timeout feature "default"
│   │   │       └── wait-timeout v0.2.1
│   │   │           └── libc feature "default" (*)
│   │   ├── tempfile feature "default" (*)
│   │   ├── bitflags feature "default"
│   │   │   └── bitflags v2.11.0
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
│   │   ├── rand feature "alloc"
│   │   │   └── rand v0.9.2
│   │   │       └── rand_core v0.9.5 (*)
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
│   │   │   ├── proptest feature "regex-syntax"
│   │   │   │   └── proptest v1.10.0 (*)
│   │   │   ├── num-traits feature "std"
│   │   │   │   └── num-traits v0.2.19 (*)
│   │   │   ├── rand feature "os_rng"
│   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   └── rand_core feature "os_rng"
│   │   │   │       └── rand_core v0.9.5 (*)
│   │   │   └── rand feature "std"
│   │   │       ├── rand v0.9.2 (*)
│   │   │       ├── rand feature "alloc" (*)
│   │   │       └── rand_core feature "std"
│   │   │           ├── rand_core v0.9.5 (*)
│   │   │           └── getrandom feature "std"
│   │   │               └── getrandom v0.3.4 (*)
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
├── serde_json feature "default"
│   ├── serde_json v1.0.149
│   │   ├── memchr v2.8.0
│   │   ├── serde_core v1.0.228
│   │   ├── itoa feature "default"
│   │   │   └── itoa v1.0.17
│   │   └── zmij feature "default"
│   │       └── zmij v1.0.21
│   └── serde_json feature "std"
│       ├── serde_json v1.0.149 (*)
│       ├── serde_core feature "std" (*)
│       └── memchr feature "std"
│           ├── memchr v2.8.0
│           └── memchr feature "alloc"
│               └── memchr v2.8.0
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
├── tokio feature "rt-multi-thread"
│   ├── tokio v1.50.0 (*)
│   └── tokio feature "rt"
│       └── tokio v1.50.0 (*)
└── tokio feature "sync"
    └── tokio v1.50.0 (*)

[exit status: 0]
```

## `python3 scripts/check-aspen-core-feature-claims.py --default-features /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/core-default-features.txt --smoke-manifest /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/smoke-manifest.txt --smoke-source /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/smoke-source.txt --cluster-features /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/cluster-core-features.txt --cli-features /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/cli-core-features.txt --output /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/feature-claims.json`

```text
$ python3 scripts/check-aspen-core-feature-claims.py --default-features /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/core-default-features.txt --smoke-manifest /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/smoke-manifest.txt --smoke-source /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/smoke-source.txt --cluster-features /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/cluster-core-features.txt --cli-features /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/cli-core-features.txt --output /tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/feature-claims.json

[exit status: 0]
```

## Feature claims JSON

```text
{
  "failures": [],
  "ok": true,
  "results": {
    "cli_features": {
      "missing_markers": [],
      "ok": true,
      "path": "/tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/cli-core-features.txt"
    },
    "cluster_features": {
      "missing_markers": [],
      "ok": true,
      "path": "/tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/cluster-core-features.txt"
    },
    "core_manifest": {
      "messages": [],
      "ok": true,
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core/Cargo.toml"
    },
    "default_features": {
      "offending_lines": [],
      "ok": true,
      "path": "/tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/core-default-features.txt"
    },
    "shell_manifest": {
      "messages": [],
      "ok": true,
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core-shell/Cargo.toml"
    },
    "smoke_manifest": {
      "messages": [],
      "ok": true,
      "path": "/tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/smoke-manifest.txt"
    },
    "smoke_source": {
      "messages": [],
      "ok": true,
      "path": "/tmp/aspen-no-std-baseline-run.MvIgm5/feature-claims/smoke-source.txt"
    }
  }
}

```

## `python3 scripts/check-aspen-core-no-std-boundary.py --manifest-path /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core/Cargo.toml --allowlist /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/scripts/aspen-core-no-std-transitives.txt --output /tmp/aspen-no-std-baseline-run.MvIgm5/boundary/deps-transitive.json --diff-output /tmp/aspen-no-std-baseline-run.MvIgm5/boundary/deps-allowlist-diff.txt`

```text
$ python3 scripts/check-aspen-core-no-std-boundary.py --manifest-path /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core/Cargo.toml --allowlist /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/scripts/aspen-core-no-std-transitives.txt --output /tmp/aspen-no-std-baseline-run.MvIgm5/boundary/deps-transitive.json --diff-output /tmp/aspen-no-std-baseline-run.MvIgm5/boundary/deps-allowlist-diff.txt
boundary check failed
- transitive allowlist mismatch
- denylisted packages resolved

[exit status: 1]
```

## Boundary JSON

```text
{
  "allowlist_path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/scripts/aspen-core-no-std-transitives.txt",
  "denylist_hits": [
    "libc@0.2.183",
    "redb@2.6.3"
  ],
  "direct": {
    "expected": [
      "aspen-cluster-types",
      "aspen-constants",
      "aspen-hlc",
      "aspen-kv-types",
      "aspen-storage-types",
      "aspen-traits",
      "async-trait",
      "base64",
      "bincode",
      "hex",
      "serde",
      "snafu",
      "thiserror"
    ],
    "missing": [],
    "resolved": [
      "aspen-cluster-types",
      "aspen-constants",
      "aspen-hlc",
      "aspen-kv-types",
      "aspen-storage-types",
      "aspen-traits",
      "async-trait",
      "base64",
      "bincode",
      "hex",
      "serde",
      "snafu",
      "thiserror"
    ],
    "unexpected": []
  },
  "failures": [
    "transitive allowlist mismatch",
    "denylisted packages resolved"
  ],
  "manifest_path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core/Cargo.toml",
  "manifest_rules": [
    {
      "messages": [],
      "ok": true,
      "package": "aspen-cluster-types"
    },
    {
      "messages": [],
      "ok": true,
      "package": "aspen-constants"
    },
    {
      "messages": [],
      "ok": true,
      "package": "aspen-hlc"
    },
    {
      "messages": [],
      "ok": true,
      "package": "aspen-kv-types"
    },
    {
      "messages": [],
      "ok": true,
      "package": "aspen-storage-types"
    },
    {
      "messages": [],
      "ok": true,
      "package": "aspen-traits"
    },
    {
      "messages": [],
      "ok": true,
      "package": "base64"
    },
    {
      "messages": [],
      "ok": true,
      "package": "bincode"
    },
    {
      "messages": [],
      "ok": true,
      "package": "hex"
    },
    {
      "messages": [],
      "ok": true,
      "package": "serde"
    },
    {
      "messages": [],
      "ok": true,
      "package": "snafu"
    }
  ],
  "ok": false,
  "review_notes": [
    {
      "messages": [],
      "ok": true,
      "package": "arrayref@0.3.9",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-arrayref.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "arrayvec@0.7.6",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-arrayvec.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "blake3@1.8.3",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-blake3.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "cfg-if@1.0.4",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-cfg-if.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "constant_time_eq@0.4.2",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-constant_time_eq.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "cpufeatures@0.2.17",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-cpufeatures.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "heck@0.5.0",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-heck.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "proc-macro2@1.0.106",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-proc-macro2.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "quote@1.0.45",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-quote.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "serde_core@1.0.228",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-serde_core.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "serde_derive@1.0.228",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-serde_derive.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "snafu-derive@0.8.9",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-snafu-derive.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "spin@0.10.0",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-spin.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "syn@2.0.117",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-syn.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "thiserror-impl@2.0.18",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-thiserror-impl.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "uhlc@0.8.2",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-uhlc.md"
    },
    {
      "messages": [],
      "ok": true,
      "package": "unicode-ident@1.0.24",
      "path": "/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive-review-unicode-ident.md"
    }
  ],
  "transitives": {
    "introduced_by": {
      "arrayref@0.3.9": "aspen-hlc@0.1.0",
      "arrayvec@0.7.6": "aspen-hlc@0.1.0",
      "blake3@1.8.3": "aspen-hlc@0.1.0",
      "cfg-if@1.0.4": "aspen-hlc@0.1.0",
      "constant_time_eq@0.4.2": "aspen-hlc@0.1.0",
      "cpufeatures@0.2.17": "aspen-hlc@0.1.0",
      "heck@0.5.0": "snafu@0.8.9",
      "libc@0.2.183": "aspen-storage-types@0.1.0",
      "proc-macro2@1.0.106": "aspen-cluster-types@0.1.0",
      "quote@1.0.45": "aspen-cluster-types@0.1.0",
      "redb@2.6.3": "aspen-storage-types@0.1.0",
      "serde_core@1.0.228": "aspen-cluster-types@0.1.0",
      "serde_derive@1.0.228": "aspen-cluster-types@0.1.0",
      "snafu-derive@0.8.9": "snafu@0.8.9",
      "spin@0.10.0": "aspen-hlc@0.1.0",
      "syn@2.0.117": "aspen-cluster-types@0.1.0",
      "thiserror-impl@2.0.18": "aspen-cluster-types@0.1.0",
      "uhlc@0.8.2": "aspen-hlc@0.1.0",
      "unicode-ident@1.0.24": "aspen-cluster-types@0.1.0"
    },
    "missing_from_graph": [],
    "resolved": [
      "arrayref@0.3.9",
      "arrayvec@0.7.6",
      "blake3@1.8.3",
      "cfg-if@1.0.4",
      "constant_time_eq@0.4.2",
      "cpufeatures@0.2.17",
      "heck@0.5.0",
      "libc@0.2.183",
      "proc-macro2@1.0.106",
      "quote@1.0.45",
      "redb@2.6.3",
      "serde_core@1.0.228",
      "serde_derive@1.0.228",
      "snafu-derive@0.8.9",
      "spin@0.10.0",
      "syn@2.0.117",
      "thiserror-impl@2.0.18",
      "uhlc@0.8.2",
      "unicode-ident@1.0.24"
    ],
    "unexpected": [
      "libc@0.2.183",
      "redb@2.6.3"
    ]
  }
}

```

## Boundary diff

```text
## Direct missing
- none

## Direct unexpected
- none

## Transitives unexpected
- libc@0.2.183
- redb@2.6.3

## Allowlist entries not in graph
- none

## Denylist hits
- libc@0.2.183
- redb@2.6.3

## Invalid review notes
- none

## Manifest rule failures
- none

```

## `python3 scripts/check-foundation-wire-deps.py --mode leaf`

```text
$ python3 scripts/check-foundation-wire-deps.py --mode leaf
FAIL aspen-storage-types still lists redb as a normal dependency
PASS aspen-traits keeps aspen-cluster-types on alloc-safe default-features = false
PASS aspen-traits keeps aspen-kv-types on alloc-safe default-features = false
FAIL aspen-storage-types no-default-features graph leaked libc, redb
PASS aspen-traits no-default-features graph excludes iroh, iroh-base, libc, redb
SUMMARY failed

[exit status: 1]
```

## `python3 scripts/check-foundation-wire-source-audits.py --mode leaf`

```text
$ python3 scripts/check-foundation-wire-source-audits.py --mode leaf
FAIL /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types/src/lib.rs still contains `redb::TableDefinition` outside test-only code
FAIL /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types/src/lib.rs still contains `TableDefinition::new` outside test-only code
PASS crates/aspen-storage-types/src excludes forbidden helpers outside tests
FAIL /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-traits/src/lib.rs still contains `std::sync::Arc` outside test-only code
PASS crates/aspen-traits/src excludes forbidden helpers outside tests
SUMMARY failed

[exit status: 1]
```

