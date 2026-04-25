# V4: Downstream Consumer Evidence

## Fixture

Path: `openspec/changes/archive/2026-04-25-extract-coordination-crate/fixtures/downstream-consumer`

Dependencies are limited to:

```toml
[dependencies]
aspen-coordination = { path = "../../../../../crates/aspen-coordination" }
aspen-kv-types = { path = "../../../../../crates/aspen-kv-types" }
aspen-traits = { path = "../../../../../crates/aspen-traits" }
```

## cargo check

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
```

## cargo metadata package names

```
ahash
anyhow
aspen-cluster-types
aspen-constants
aspen-coordination
aspen-coordination-downstream-consumer
aspen-kv-types
aspen-time
aspen-traits
async-trait
cfg-if
getrandom
heck
itoa
libc
memchr
metrics
once_cell
pin-project-lite
portable-atomic
ppv-lite86
proc-macro2
quote
r-efi
rand
rand_chacha
rand_core
serde
serde_core
serde_derive
serde_json
snafu
snafu-derive
syn
thiserror
thiserror-impl
tokio
tokio-macros
tracing
tracing-attributes
tracing-core
unicode-ident
version_check
wasip2
wit-bindgen
zerocopy
zerocopy-derive
zmij
```

## Root aspen, handler, and binary dependency check

```
OK: aspen not present
OK: aspen-cli not present
OK: aspen-tui not present
OK: aspen-dogfood not present
OK: aspen-rpc-handlers not present
OK: aspen-core-essentials-handler not present
OK: aspen-blob-handler not present
OK: aspen-ci-handler not present
OK: aspen-cluster-handler not present
OK: aspen-docs-handler not present
OK: aspen-forge-handler not present
OK: aspen-job-handler not present
OK: aspen-nix-handler not present
OK: aspen-secrets-handler not present
```
