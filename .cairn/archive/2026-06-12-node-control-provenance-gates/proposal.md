## Why

Node control can now loop over local `install` and `run` requests, which means side-effecting artifact changes and job execution may happen without an operator at the terminal. Before making that loop long-lived or remote-facing, those side effects need explicit provenance evidence instead of treating content hashes as trust.

## What Changes

- Add canonical provenance record and receipt artifacts for artifact refs, source refs, builder/toolchain refs, review/test/source-gate refs, and admitted trust state.
- Extend node control requests with explicit evidence refs while preserving the legacy request shape for replay.
- Require admitted reviewed provenance before node-control `install` writes registry artifacts.
- Require admitted reviewed provenance for the job artifact before node-control `run` executes a job request.
- Emit provenance gate receipts as node control subreceipts and keep missing, mismatched, sandbox-only, or malformed provenance fail-closed before side effects.
- Add a CLI fixture helper for synthetic reviewed provenance used by tests and local operator workflows.

## Impact

The local node control loop remains file-backed and deterministic, but side-effecting operations now require supply-chain evidence. This prepares the node control surface for future supervisor/remote ingress work without adding a socket or network protocol in this slice.
