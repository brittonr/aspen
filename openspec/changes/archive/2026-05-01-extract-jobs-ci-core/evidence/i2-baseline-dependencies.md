# I2 Baseline dependency graphs

## aspen-jobs default

```text
$ cargo tree -p aspen-jobs -e normal --depth 2
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-jobs v0.1.0 (/home/brittonr/git/aspen/crates/aspen-jobs)
├── anyhow v1.0.102
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   ├── serde v1.0.228
│   └── thiserror v2.0.18
├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
├── aspen-coordination v0.1.0 (/home/brittonr/git/aspen/crates/aspen-coordination)
│   ├── anyhow v1.0.102
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
│   ├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types)
│   ├── aspen-time v0.1.0 (/home/brittonr/git/aspen/crates/aspen-time)
│   ├── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits)
│   ├── async-trait v0.1.89 (proc-macro)
│   ├── metrics v0.24.3
│   ├── rand v0.9.2
│   ├── serde v1.0.228 (*)
│   ├── serde_json v1.0.149
│   ├── snafu v0.8.9
│   ├── thiserror v2.0.18 (*)
│   ├── tokio v1.50.0
│   └── tracing v0.1.44
├── aspen-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-core)
│   ├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types) (*)
│   ├── aspen-constants v0.1.0 (/home/brittonr/git/aspen/crates/aspen-constants)
│   ├── aspen-hlc v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hlc)
│   ├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types) (*)
│   ├── aspen-storage-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-storage-types)
│   ├── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits) (*)
│   ├── async-trait v0.1.89 (proc-macro) (*)
│   ├── base64 v0.22.1
│   ├── bincode v1.3.3
│   ├── hex v0.4.3
│   ├── serde v1.0.228 (*)
│   ├── snafu v0.8.9 (*)
│   └── thiserror v2.0.18 (*)
├── aspen-hlc v0.1.0 (/home/brittonr/git/aspen/crates/aspen-hlc) (*)
├── aspen-kv-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-kv-types) (*)
├── aspen-time v0.1.0 (/home/brittonr/git/aspen/crates/aspen-time)
├── aspen-traits v0.1.0 (/home/brittonr/git/aspen/crates/aspen-traits) (*)
├── async-trait v0.1.89 (proc-macro) (*)
├── base64 v0.22.1
├── bincode v1.3.3 (*)
├── blake3 v1.8.3
│   ├── arrayref v0.3.9
│   ├── arrayvec v0.7.6
│   ├── cfg-if v1.0.4
│   ├── constant_time_eq v0.4.2
│   └── cpufeatures v0.2.17
├── bytes v1.11.1
├── chrono v0.4.44
│   ├── iana-time-zone v0.1.65
│   ├── num-traits v0.2.19
│   └── serde v1.0.228 (*)
├── cron v0.13.0
│   ├── chrono v0.4.44 (*)
│   ├── nom v7.1.3
│   └── once_cell v1.21.4
├── flate2 v1.1.9
│   ├── crc32fast v1.5.0
│   └── miniz_oxide v0.8.9
├── getrandom v0.2.17
│   ├── cfg-if v1.0.4
│   └── libc v0.2.183
├── glob v0.3.3
├── hex v0.4.3
├── iroh v0.97.0
│   ├── backon v1.6.0
│   ├── bytes v1.11.1
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1
│   ├── ed25519-dalek v3.0.0-pre.1
│   ├── futures-util v0.3.32
│   ├── hickory-resolver v0.25.2
│   ├── http v1.4.0
│   ├── ipnet v2.12.0
│   ├── iroh-base v0.97.0
│   ├── iroh-metrics v0.38.3
│   ├── iroh-relay v0.97.0
│   ├── n0-error v0.1.3
│   ├── n0-future v0.3.2
│   ├── n0-watcher v0.6.1
│   ├── netwatch v0.15.0
│   ├── noq v0.17.0
│   ├── noq-proto v0.16.0
│   ├── noq-udp v0.9.0
│   ├── papaya v0.2.3
│   ├── pin-project v1.1.11
│   ├── pkarr v5.0.2
│   ├── pkcs8 v0.11.0-rc.11
│   ├── portable-atomic v1.13.1
│   ├── portmapper v0.15.0
│   ├── rand v0.9.2 (*)
│   ├── reqwest v0.12.28
│   ├── rustc-hash v2.1.1
│   ├── rustls v0.23.37
│   ├── rustls-pki-types v1.14.0
│   ├── rustls-webpki v0.103.9
│   ├── serde v1.0.228 (*)
│   ├── smallvec v1.15.1
│   ├── strum v0.28.0
│   ├── sync_wrapper v1.0.2
│   ├── tokio v1.50.0 (*)
│   ├── tokio-stream v0.1.18
│   ├── tokio-util v0.7.18
│   ├── tracing v0.1.44 (*)
│   ├── url v2.5.8
│   └── webpki-roots v1.0.6
├── libc v0.2.183
├── rand v0.9.2 (*)
├── redb v2.6.3
│   └── libc v0.2.183
├── serde v1.0.228 (*)
├── serde_json v1.0.149 (*)
├── snafu v0.8.9 (*)
├── tempfile v3.27.0
│   ├── fastrand v2.3.0
│   ├── getrandom v0.4.2
│   ├── once_cell v1.21.4 (*)
│   └── rustix v1.1.4
├── thiserror v2.0.18 (*)
├── tokio v1.50.0 (*)
├── tokio-util v0.7.18 (*)
├── tracing v0.1.44 (*)
├── uuid v1.22.0
│   ├── getrandom v0.4.2 (*)
│   └── serde_core v1.0.228
└── zstd v0.13.3
    └── zstd-safe v7.2.4
exit_code=0
```

## aspen-ci-core default

```text
$ cargo tree -p aspen-ci-core -e normal --depth 2
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-ci-core v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ci-core)
├── chrono v0.4.44
│   ├── iana-time-zone v0.1.65
│   └── num-traits v0.2.19
├── schemars v0.8.22
│   ├── dyn-clone v1.0.20
│   ├── schemars_derive v0.8.22 (proc-macro)
│   ├── serde v1.0.228
│   └── serde_json v1.0.149
├── serde v1.0.228 (*)
├── serde_json v1.0.149 (*)
├── snafu v0.8.9
│   └── snafu-derive v0.8.9 (proc-macro)
└── uuid v1.22.0
    ├── getrandom v0.4.2
    └── serde_core v1.0.228
exit_code=0
```

## aspen-jobs-protocol default

```text
$ cargo tree -p aspen-jobs-protocol -e normal --depth 2
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
exit_code=0
```

## Forbidden from reusable `aspen-jobs-core` defaults

The new core default surface must exclude root `aspen`, handler crates, `aspen-jobs` runtime shell, job worker crates, CI executor crates, concrete transport (`iroh`, `iroh-base`, `irpc`), Redb/storage backends, process/Nix/SNIX/VM execution, node/bootstrap crates, and handler/client runtime graphs.

Allowed first-slice dependencies are expected to be limited to serialization/error/helper crates plus small Aspen reusable contracts such as `aspen-jobs-protocol` or `aspen-ci-core` only when the dependency direction is justified.
