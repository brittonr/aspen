Evidence-ID: extend-no-std-foundation-and-wire.v2
Task-ID: V2
Artifact-Type: verification
Covers: core.no-std-core-baseline.bare-dependency-uses-alloc-only-default, core.no-std-core-baseline.alloc-only-build-succeeds, core.no-std-core-baseline.bare-default-downstream-consumer-remains-supported, core.no-std-core-baseline.representative-std-consumers-remain-supported, architecture.modularity.feature-bundles-are-explicit-and-bounded.std-compatibility-is-an-explicit-opt-in, core.no-std-core-baseline.compile-slice-verification-is-reviewable, architecture.modularity.acyclic-no-std-core-boundary.pure-consumers-avoid-runtime-shells, architecture.modularity.acyclic-no-std-core-boundary.leaf-crates-default-to-alloc-safe-builds, architecture.modularity.acyclic-no-std-core-boundary.leaf-crates-reject-forbidden-runtime-helpers, architecture.modularity.acyclic-no-std-core-boundary.leaf-crate-verification-is-reviewable, architecture.modularity.feature-bundles-are-explicit-and-bounded.alloc-only-core-excludes-runtime-shells, architecture.modularity.feature-bundles-are-explicit-and-bounded.dependency-boundary-is-checked-deterministically, architecture.modularity.feature-bundles-are-explicit-and-bounded.feature-topology-verification-is-reviewable

# Core foundation verification

## `cargo check -p aspen-core`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
```

## `cargo check -p aspen-core --no-default-features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
```

## `cargo check -p aspen-core --no-default-features --features sql`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.19s
```

## `cargo check -p aspen-core-no-std-smoke`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s
```

## `cargo check -p aspen-core-shell`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.40s
```

## `cargo check -p aspen-core-shell --features layer`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
```

## `cargo check -p aspen-core-shell --features global-discovery`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.26s
```

## `cargo check -p aspen-core-shell --features sql`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.23s
```

## `cargo check -p aspen-storage-types`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
```

## `cargo check -p aspen-storage-types --no-default-features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s
```

## `cargo check -p aspen-storage-types --no-default-features --target wasm32-unknown-unknown`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling unicode-ident v1.0.24
   Compiling serde_core v1.0.228
   Compiling serde v1.0.228
   Compiling syn v2.0.117
   Compiling serde_derive v1.0.228
    Checking bincode v1.3.3
    Checking aspen-storage-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-storage-types)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.69s
```

## `cargo check -p aspen-traits`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
```

## `cargo check -p aspen-traits --no-default-features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
```

## `cargo check -p aspen-traits --no-default-features --target wasm32-unknown-unknown`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling serde_core v1.0.228
   Compiling serde v1.0.228
   Compiling thiserror v2.0.18
    Checking aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
   Compiling syn v2.0.117
   Compiling thiserror-impl v2.0.18
   Compiling serde_derive v1.0.228
   Compiling async-trait v0.1.89
    Checking aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
    Checking aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
    Checking aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.28s
```

## `cargo tree -p aspen-storage-types -e normal`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-storage-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-storage-types)
├── bincode v1.3.3
│   └── serde v1.0.228
│       ├── serde_core v1.0.228
│       └── serde_derive v1.0.228 (proc-macro)
│           ├── proc-macro2 v1.0.106
│           │   └── unicode-ident v1.0.24
│           ├── quote v1.0.45
│           │   └── proc-macro2 v1.0.106 (*)
│           └── syn v2.0.117
│               ├── proc-macro2 v1.0.106 (*)
│               ├── quote v1.0.45 (*)
│               └── unicode-ident v1.0.24
└── serde v1.0.228 (*)
```

## `cargo tree -p aspen-storage-types -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-storage-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-storage-types)
├── bincode v1.3.3
│   └── serde feature "default"
│       ├── serde v1.0.228
│       │   ├── serde_core feature "result"
│       │   │   └── serde_core v1.0.228
│       │   └── serde_derive feature "default"
│       │       └── serde_derive v1.0.228 (proc-macro)
│       │           ├── proc-macro2 feature "proc-macro"
│       │           │   └── proc-macro2 v1.0.106
│       │           │       └── unicode-ident feature "default"
│       │           │           └── unicode-ident v1.0.24
│       │           ├── quote feature "proc-macro"
│       │           │   ├── quote v1.0.45
│       │           │   │   └── proc-macro2 v1.0.106 (*)
│       │           │   └── proc-macro2 feature "proc-macro" (*)
│       │           ├── syn feature "clone-impls"
│       │           │   └── syn v2.0.117
│       │           │       ├── proc-macro2 v1.0.106 (*)
│       │           │       ├── quote v1.0.45 (*)
│       │           │       └── unicode-ident feature "default" (*)
│       │           ├── syn feature "derive"
│       │           │   └── syn v2.0.117 (*)
│       │           ├── syn feature "parsing"
│       │           │   └── syn v2.0.117 (*)
│       │           ├── syn feature "printing"
│       │           │   └── syn v2.0.117 (*)
│       │           └── syn feature "proc-macro"
│       │               ├── syn v2.0.117 (*)
│       │               ├── proc-macro2 feature "proc-macro" (*)
│       │               └── quote feature "proc-macro" (*)
│       └── serde feature "std"
│           ├── serde v1.0.228 (*)
│           └── serde_core feature "std"
│               └── serde_core v1.0.228
├── serde feature "alloc"
│   ├── serde v1.0.228 (*)
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
└── postcard feature "default"
    ├── postcard v1.1.3 (*)
    └── postcard feature "heapless-cas"
        ├── postcard v1.1.3 (*)
        ├── postcard feature "heapless"
        │   └── postcard v1.1.3 (*)
        └── heapless feature "cas"
            ├── heapless v0.7.17 (*)
            └── heapless feature "atomic-polyfill"
                └── heapless v0.7.17 (*)
```

## `cargo tree -p aspen-traits -e normal`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
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
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
│   ├── serde v1.0.228 (*)
│   └── thiserror v2.0.18 (*)
└── async-trait v0.1.89 (proc-macro)
    ├── proc-macro2 v1.0.106 (*)
    ├── quote v1.0.45 (*)
    └── syn v2.0.117 (*)
```

## `cargo tree -p aspen-traits -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
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
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
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
```

## `cargo tree -p aspen-traits -e features -i aspen-cluster-types`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
└── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
    └── aspen-traits feature "default" (command-line)
```

## `python3 scripts/check-foundation-wire-deps.py --mode leaf`

```text
PASS aspen-storage-types keeps redb out of normal dependencies
PASS aspen-traits keeps aspen-cluster-types on alloc-safe default-features = false
PASS aspen-traits keeps aspen-kv-types on alloc-safe default-features = false
PASS aspen-storage-types no-default-features graph excludes iroh, iroh-base, libc, redb
PASS aspen-traits no-default-features graph excludes iroh, iroh-base, libc, redb
SUMMARY ok
```

## `python3 scripts/check-foundation-wire-source-audits.py --mode leaf`

```text
PASS crates/aspen-storage-types/src excludes forbidden helpers outside tests
PASS crates/aspen-traits/src excludes forbidden helpers outside tests
SUMMARY ok
```

## `cargo tree -p aspen-core --no-default-features -e normal --depth 1`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
├── aspen-hlc v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hlc)
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
├── aspen-storage-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-storage-types)
├── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
├── async-trait v0.1.89 (proc-macro)
├── base64 v0.22.1
├── bincode v1.3.3
├── hex v0.4.3
├── serde v1.0.228
├── snafu v0.8.9
└── thiserror v2.0.18
```

## `cargo tree -p aspen-core --no-default-features -e normal`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
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
├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
├── aspen-hlc v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hlc)
│   ├── blake3 v1.8.3
│   │   ├── arrayref v0.3.9
│   │   ├── arrayvec v0.7.6
│   │   ├── cfg-if v1.0.4
│   │   ├── constant_time_eq v0.4.2
│   │   └── cpufeatures v0.2.17
│   ├── serde v1.0.228 (*)
│   └── uhlc v0.8.2 (/home/brittonr/git/aspen/vendor/uhlc)
│       ├── serde v1.0.228 (*)
│       └── spin v0.10.0
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
│   ├── serde v1.0.228 (*)
│   └── thiserror v2.0.18 (*)
├── aspen-storage-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-storage-types)
│   ├── bincode v1.3.3
│   │   └── serde v1.0.228 (*)
│   └── serde v1.0.228 (*)
├── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
│   ├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types) (*)
│   ├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types) (*)
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
```

## `cargo tree -p aspen-core --no-default-features -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
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
├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
├── aspen-hlc v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hlc)
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
│   ├── uhlc v0.8.2 (/home/brittonr/git/aspen/vendor/uhlc)
│   │   ├── serde feature "alloc" (*)
│   │   ├── serde feature "derive" (*)
│   │   ├── spin feature "mutex"
│   │   │   └── spin v0.10.0
│   │   └── spin feature "spin_mutex"
│   │       ├── spin v0.10.0
│   │       └── spin feature "mutex" (*)
│   ├── serde feature "alloc" (*)
│   └── serde feature "derive" (*)
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
│   ├── serde feature "alloc" (*)
│   ├── serde feature "derive" (*)
│   └── thiserror feature "default" (*)
├── aspen-storage-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-storage-types)
│   ├── bincode v1.3.3
│   │   └── serde feature "default"
│   │       ├── serde v1.0.228 (*)
│   │       └── serde feature "std"
│   │           ├── serde v1.0.228 (*)
│   │           └── serde_core feature "std"
│   │               └── serde_core v1.0.228
│   ├── serde feature "alloc" (*)
│   └── serde feature "derive" (*)
├── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
│   ├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types) (*)
│   ├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types) (*)
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
```

## `cargo check -p aspen-cluster`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.29s
```

## `cargo check -p aspen-client`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.36s
```

## `cargo check -p aspen-rpc-handlers`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.29s
```

## `cargo check -p aspen --no-default-features --features node-runtime`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
```

## `cargo tree -p aspen-core -e features`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
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
├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
├── aspen-hlc v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hlc)
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
│   ├── uhlc v0.8.2 (/home/brittonr/git/aspen/vendor/uhlc)
│   │   ├── serde feature "alloc" (*)
│   │   ├── serde feature "derive" (*)
│   │   ├── spin feature "mutex"
│   │   │   └── spin v0.10.0
│   │   └── spin feature "spin_mutex"
│   │       ├── spin v0.10.0
│   │       └── spin feature "mutex" (*)
│   ├── serde feature "alloc" (*)
│   └── serde feature "derive" (*)
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
│   ├── serde feature "alloc" (*)
│   ├── serde feature "derive" (*)
│   └── thiserror feature "default" (*)
├── aspen-storage-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-storage-types)
│   ├── bincode v1.3.3
│   │   └── serde feature "default"
│   │       ├── serde v1.0.228 (*)
│   │       └── serde feature "std"
│   │           ├── serde v1.0.228 (*)
│   │           └── serde_core feature "std"
│   │               └── serde_core v1.0.228
│   ├── serde feature "alloc" (*)
│   └── serde feature "derive" (*)
├── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
│   ├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types) (*)
│   ├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types) (*)
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
```

## `cargo tree -p aspen-cluster -e features -i aspen-core`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
└── aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
    ├── aspen-core-shell feature "default"
    │   ├── aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
    │   │   └── aspen-auth feature "default"
    │   │       ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
    │   │       │   └── aspen-cluster feature "default" (command-line)
    │   │       ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft)
    │   │       │   └── aspen-raft feature "default"
    │   │       │       └── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    │   │       └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
    │   │           └── aspen-transport feature "default"
    │   │               ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    │   │               ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    │   │               └── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network)
    │   │                   └── aspen-raft-network feature "default"
    │   │                       └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    │   ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    │   ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    │   ├── aspen-testing v0.1.0 (/home/brittonr/git/aspen/crates/aspen-testing)
    │   │   └── aspen-testing feature "default"
    │   │       [dev-dependencies]
    │   │       └── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    │   └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
    └── aspen-core-shell feature "layer"
        └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
└── aspen-core feature "default"
    ├── aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination)
    │   └── aspen-coordination feature "default"
    │       └── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    ├── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network) (*)
    ├── aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
    │   └── aspen-raft-types feature "default"
    │       ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    │       ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    │       ├── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network) (*)
    │       └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
    ├── aspen-redb-storage v0.1.0 (/home/brittonr/git/aspen/crates/aspen-redb-storage)
    │   └── aspen-redb-storage feature "default"
    │       └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    └── aspen-sharding v0.1.0 (/home/brittonr/git/aspen/crates/aspen-sharding)
        └── aspen-sharding feature "default"
            ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
            ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
            ├── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network) (*)
            └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
```

## `cargo tree -p aspen-cli -e features -i aspen-core`

```text
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
└── aspen-core-shell v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core-shell)
    ├── aspen-core-shell feature "default"
    │   ├── aspen-auth v0.1.0 (/home/brittonr/git/aspen/crates/aspen-auth)
    │   │   └── aspen-auth feature "default"
    │   │       ├── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli)
    │   │       │   └── aspen-cli feature "default" (command-line)
    │   │       ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    │   │       │   └── aspen-client feature "default"
    │   │       │       └── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
    │   │       ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
    │   │       │   └── aspen-cluster feature "default"
    │   │       │       └── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
    │   │       ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft)
    │   │       │   └── aspen-raft feature "default"
    │   │       │       └── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    │   │       └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport)
    │   │           └── aspen-transport feature "default"
    │   │               ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client) (*)
    │   │               ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    │   │               ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    │   │               └── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network)
    │   │                   └── aspen-raft-network feature "default"
    │   │                       └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    │   ├── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
    │   ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    │   ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    │   └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
    └── aspen-core-shell feature "layer"
        ├── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
        └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
└── aspen-core feature "default"
    ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client) (*)
    ├── aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination)
    │   └── aspen-coordination feature "default"
    │       └── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    ├── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network) (*)
    ├── aspen-raft-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-types)
    │   └── aspen-raft-types feature "default"
    │       ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
    │       ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    │       ├── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network) (*)
    │       └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
    ├── aspen-redb-storage v0.1.0 (/home/brittonr/git/aspen/crates/aspen-redb-storage)
    │   └── aspen-redb-storage feature "default"
    │       └── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
    └── aspen-sharding v0.1.0 (/home/brittonr/git/aspen/crates/aspen-sharding)
        └── aspen-sharding feature "default"
            ├── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
            ├── aspen-raft v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft) (*)
            ├── aspen-raft-network v0.1.0 (/home/brittonr/git/aspen/crates/aspen-raft-network) (*)
            └── aspen-transport v0.1.0 (/home/brittonr/git/aspen/crates/aspen-transport) (*)
```

## `python3 scripts/check-aspen-core-feature-claims.py --default-features openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/core-default-features.txt --smoke-manifest openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/smoke-manifest.txt --smoke-source openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/smoke-source.txt --cluster-features openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/cluster-core-features.txt --cli-features openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/cli-core-features.txt --output openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/feature-claims.json`

```text
```

## `python3 scripts/check-aspen-core-no-std-boundary.py --manifest-path crates/aspen-core/Cargo.toml --allowlist scripts/aspen-core-no-std-transitives.txt --output openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/deps-transitive.json --diff-output openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/deps-allowlist-diff.txt`

```text
```
