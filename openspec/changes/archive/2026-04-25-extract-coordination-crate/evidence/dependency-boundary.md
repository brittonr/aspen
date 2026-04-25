# V1: Dependency Boundary Evidence

## cargo tree -p aspen-coordination --edges normal

```
aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination)
├── anyhow v1.0.102
├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
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
├── aspen-time v0.1.0 (/home/brittonr/git/aspen/crates/aspen-time)
├── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
│   ├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   │   ├── serde v1.0.228 (*)
│   │   └── thiserror v2.0.18 (*)
│   ├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types) (*)
│   └── async-trait v0.1.89 (proc-macro)
│       ├── proc-macro2 v1.0.106 (*)
│       ├── quote v1.0.45 (*)
│       └── syn v2.0.117 (*)
├── async-trait v0.1.89 (proc-macro) (*)
├── metrics v0.24.3
│   └── ahash v0.8.12
│       ├── cfg-if v1.0.4
│       ├── once_cell v1.21.4
│       └── zerocopy v0.8.42
├── rand v0.9.2
│   ├── rand_chacha v0.9.0
│   │   ├── ppv-lite86 v0.2.21
│   │   │   └── zerocopy v0.8.42
│   │   └── rand_core v0.9.5
│   │       └── getrandom v0.3.4
│   │           ├── cfg-if v1.0.4
│   │           └── libc v0.2.183
│   └── rand_core v0.9.5 (*)
├── serde v1.0.228 (*)
├── serde_json v1.0.149
│   ├── itoa v1.0.17
│   ├── memchr v2.8.0
│   ├── serde_core v1.0.228
│   └── zmij v1.0.21
├── snafu v0.8.9
│   └── snafu-derive v0.8.9 (proc-macro)
│       ├── heck v0.5.0
│       ├── proc-macro2 v1.0.106 (*)
│       ├── quote v1.0.45 (*)
│       └── syn v2.0.117 (*)
├── thiserror v2.0.18 (*)
├── tokio v1.50.0
│   ├── pin-project-lite v0.2.17
│   └── tokio-macros v2.6.1 (proc-macro)
│       ├── proc-macro2 v1.0.106 (*)
│       ├── quote v1.0.45 (*)
│       └── syn v2.0.117 (*)
└── tracing v0.1.44
    ├── pin-project-lite v0.2.17
    ├── tracing-attributes v0.1.31 (proc-macro)
    │   ├── proc-macro2 v1.0.106 (*)
    │   ├── quote v1.0.45 (*)
    │   └── syn v2.0.117 (*)
    └── tracing-core v0.1.36
        └── once_cell v1.21.4
```

## Forbidden dependency grep (normal dependencies only)

```
OK: aspen-core not found in normal dependency tree
OK: aspen-core-shell not found in normal dependency tree
OK: aspen not found in normal dependency tree
OK: aspen-cli not found in normal dependency tree
OK: aspen-tui not found in normal dependency tree
OK: aspen-forge-web not found in normal dependency tree
OK: aspen-nix-cache-gateway not found in normal dependency tree
OK: aspen-snix-bridge not found in normal dependency tree
OK: aspen-dogfood not found in normal dependency tree
OK: aspen-rpc-handlers not found in normal dependency tree
OK: aspen-core-essentials-handler not found in normal dependency tree
OK: aspen-blob-handler not found in normal dependency tree
OK: aspen-ci-handler not found in normal dependency tree
OK: aspen-cluster-handler not found in normal dependency tree
OK: aspen-docs-handler not found in normal dependency tree
OK: aspen-forge-handler not found in normal dependency tree
OK: aspen-job-handler not found in normal dependency tree
OK: aspen-nix-handler not found in normal dependency tree
OK: aspen-secrets-handler not found in normal dependency tree
OK: aspen-transport not found in normal dependency tree
OK: aspen-node not found in normal dependency tree
OK: aspen-cluster not found in normal dependency tree
OK: aspen-raft not found in normal dependency tree
OK: aspen-trust not found in normal dependency tree
OK: aspen-secrets not found in normal dependency tree
OK: aspen-sql not found in normal dependency tree
OK: iroh not found in normal dependency tree
OK: iroh-base not found in normal dependency tree
OK: irpc not found in normal dependency tree
```

## cargo tree -p aspen-coordination --no-default-features --edges normal

```
aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination)
├── anyhow v1.0.102
├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
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
├── aspen-time v0.1.0 (/home/brittonr/git/aspen/crates/aspen-time)
├── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
│   ├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   │   ├── serde v1.0.228 (*)
│   │   └── thiserror v2.0.18 (*)
│   ├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types) (*)
│   └── async-trait v0.1.89 (proc-macro)
│       ├── proc-macro2 v1.0.106 (*)
│       ├── quote v1.0.45 (*)
│       └── syn v2.0.117 (*)
├── async-trait v0.1.89 (proc-macro) (*)
├── metrics v0.24.3
│   └── ahash v0.8.12
│       ├── cfg-if v1.0.4
│       ├── once_cell v1.21.4
│       └── zerocopy v0.8.42
├── rand v0.9.2
│   ├── rand_chacha v0.9.0
│   │   ├── ppv-lite86 v0.2.21
│   │   │   └── zerocopy v0.8.42
│   │   └── rand_core v0.9.5
│   │       └── getrandom v0.3.4
│   │           ├── cfg-if v1.0.4
│   │           └── libc v0.2.183
│   └── rand_core v0.9.5 (*)
├── serde v1.0.228 (*)
├── serde_json v1.0.149
│   ├── itoa v1.0.17
│   ├── memchr v2.8.0
│   ├── serde_core v1.0.228
│   └── zmij v1.0.21
├── snafu v0.8.9
│   └── snafu-derive v0.8.9 (proc-macro)
│       ├── heck v0.5.0
│       ├── proc-macro2 v1.0.106 (*)
│       ├── quote v1.0.45 (*)
│       └── syn v2.0.117 (*)
├── thiserror v2.0.18 (*)
├── tokio v1.50.0
│   ├── pin-project-lite v0.2.17
│   └── tokio-macros v2.6.1 (proc-macro)
│       ├── proc-macro2 v1.0.106 (*)
│       ├── quote v1.0.45 (*)
│       └── syn v2.0.117 (*)
└── tracing v0.1.44
    ├── pin-project-lite v0.2.17
    ├── tracing-attributes v0.1.31 (proc-macro)
    │   ├── proc-macro2 v1.0.106 (*)
    │   ├── quote v1.0.45 (*)
    │   └── syn v2.0.117 (*)
    └── tracing-core v0.1.36
        └── once_cell v1.21.4
```

## Note on dev-dependencies

aspen-testing is a dev-dependency and transitively pulls aspen-core-shell -> aspen-core.
The checked boundary is the normal dependency graph for default/no-default features.
