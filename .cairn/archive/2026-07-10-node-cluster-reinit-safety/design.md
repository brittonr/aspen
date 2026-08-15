## Context

`molten node init` writes durable node config and identity receipts. `molten cluster init` plans per-node state roots and then calls node init for each planned node. Before this change, a repeated init could proceed far enough to overwrite or mix with existing lifecycle files, and the cluster wrapper did not provide explicit reset semantics.

## Decisions

### Lifecycle state is classified before node init

**Choice:** Add a small daemon lifecycle classifier over in-memory booleans for config, identity receipt, startup receipt, shutdown receipt, and active control lock presence. Node init scans those files in the shell, passes the booleans to the pure classifier, and only permits `Empty` roots.

**Rationale:** The pure classifier is testable without filesystem setup, while the shell still owns state-root IO and error reporting.

### Cluster init denies collisions unless reset is explicit

**Choice:** Non-force cluster init rejects an existing cluster manifest and rejects any planned node root whose daemon lifecycle state is not `Empty`. `--force` removes only the planned node root directories and then writes the new manifest after successful node initialization.

**Rationale:** Existing manifests and lifecycle files are durable operator evidence. Resetting them should require an explicit flag and should not remove unrelated directories outside the planned nodes.

### Ambient cluster roots are invalid

**Choice:** Cluster planning rejects empty, current-directory, and parent-directory state-root syntax before deriving node paths.

**Rationale:** Cluster init can create or remove multiple node roots. Rejecting ambient roots avoids accidental mutation of the checkout or its parent.

## Functional core / shell split

- Pure core: `node_lifecycle_state` classifies `NodeLifecycleFiles`; cluster planning validates node names, duplicate node ids, and ambient state-root syntax from in-memory inputs.
- Shell: daemon init scans lifecycle paths and writes receipts; cluster init checks manifest existence, removes planned node root directories only when `--force` is set, calls node init, and writes the manifest.

## Risks / Trade-offs

- Reinitializing a stopped local fixture now requires an explicit reset rather than silently overwriting state.
- `--force` intentionally removes only planned node roots, not every sibling directory under the cluster root, to keep reset scope narrow.
- Lifecycle classifications are operational safety checks only; downstream gates still evaluate canonical receipts and authority evidence.
