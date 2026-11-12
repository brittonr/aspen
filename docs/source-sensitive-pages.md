# Source-sensitive documentation pages

This is a bounded review aid, not a second specification registry. A page in
this table makes behavioral claims tied to named source. When a change touches
one of the listed source paths, review the page in the same change, or record in
the change/evidence why the page still matches. Link checks and source review do
not prove behavioral claims; the checked-in implementation and tests remain the
authority.

| Page | Primary source to re-read | Verification |
| --- | --- | --- |
| [Prolly semantic-state map](prolly-semantic-state-map.md) | `crates/molten-core/src/prolly_map/operations.rs`, `crates/molten-core/src/prolly_map/tree/build.rs`, `crates/molten-core/src/prolly_map/diff.rs` | `cargo test -p molten-core --lib prolly` |
| [History-independent semantic maps](technical/world-state/history-independent-semantic-maps.md) | `crates/molten-core/src/prolly_map/`, `src/prolly_map/service.rs` | Focused Prolly tests plus the retained benchmark profile |
| [Fabric transport sessions](fabric-transport-session-runtime.md) | `crates/molten-core/src/fabric_transport/`, `src/fabric_transport/cross_process/` | Focused transport tests and the declared loopback profile |
| [Node state filesystem authority](node-state-filesystem-authority.md) | `src/node_state/`, `crates/molten-node-host/src/` | Focused node-state tests |
| [Content-store adapter](content-store-adapter.md) | `crates/molten-core/src/content_store_adapter/`, `src/content_store_adapter/` | Focused content-store tests |
| [NixOS VM executable faults](nixos-vm-executable-faults.md) | `src/nixos/vm/`, `nix/vm-signed-source-check.nix` | `cargo test -p molten --lib vm_fault` plus the declared VM rail |
| [Component RPC pilot](component-rpc-pilot.md) | `src/componentrpc/`, `config/component-rpc/`, `tests/componentrpc.rs` | `cargo test --features component-rpc --test componentrpc` |
| [SpaceWasm core pilot](spacewasm-core-pilot.md) | `src/spacewasm/`, `config/spacewasm/` | `cargo test --features spacewasm-pilot -p molten --lib` |
| [Reproducible dependencies](reproducible-dependencies.md) | `Cargo.toml`, `Cargo.lock`, `flake.nix`, `flake.lock`, `config/release-dependencies/profile.ncl`, `crate-hashes.json`, both unit2nix plans | `nix build .#checks.x86_64-linux.release-dependency-profile` and `scripts/git-source-hashes.sh --check` |
| [README first workflow](README.md) | `src/cli/ops/`, `src/system_extension/` | Execute the documented command in a fresh output directory |

Related pages outside this table may still make source claims. Use the source
links in the page itself and the governing [documentation map](README.md) when a
change crosses a boundary this table does not name.
