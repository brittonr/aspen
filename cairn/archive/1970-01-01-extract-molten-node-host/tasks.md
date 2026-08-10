## Tasks

- [x] [serial] Record the published source identity and unchanged workspace, node-state, and local-store test baseline. r[molten.node_host.crate_boundary]
- [x] [depends:baseline-recorded] Add `molten-node-host` with only `molten-core`, capability filesystem dependencies, and the moved shared error, node-state, and local-store modules. r[molten.node_host.crate_boundary] r[molten.node_host.bridge_authority]
- [x] [depends:node-host-crate] Replace root definitions with explicit compatibility re-exports and keep required capability-derived bridge methods narrow. r[molten.node_host.facade_compatibility] r[molten.node_host.bridge_authority]
- [x] [depends:root-facades] Add positive new-path and old-path tests plus negative forbidden-dependency, missing-required-dependency, and malformed-manifest fixtures. r[molten.node_host.crate_boundary] r[molten.node_host.facade_compatibility]
- [x] [depends:boundary-fixtures] Update workspace, Nix, Octet or source-gate coverage, README, and node authority documentation for the new owner and bounded non-claims. r[molten.node_host.crate_boundary]
- [x] [depends:documentation-and-gates] Run focused and workspace tests, Clippy, repository source gates, Cairn validation and gates, and Nix checks. Record exact source and validation identities. r[molten.node_host.facade_compatibility]
- [x] [depends:validation-recorded] Sync the accepted node-runtime requirements and archive the completed change without touching the dirty primary worktree. r[molten.node_host.crate_boundary] r[molten.node_host.facade_compatibility] r[molten.node_host.bridge_authority]
