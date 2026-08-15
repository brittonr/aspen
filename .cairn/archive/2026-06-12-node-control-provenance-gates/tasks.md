## Phase 1: Canonical evidence

- [x] [serial] r[molten.node_control_provenance.spec.canonical_evidence] Add canonical provenance record/receipt values and ledger classification.
- [x] [serial] r[molten.node_control_provenance.spec.canonical_evidence] Extend node control requests with explicit evidence refs and legacy parsing.

## Phase 2: Side-effect gates

- [x] [serial] r[molten.node_control_provenance.spec.install_gate] Gate node-control install on admitted provenance for the payload ref before registry writes.
- [x] [serial] r[molten.node_control_provenance.spec.run_gate] Gate node-control run on admitted provenance for the job ref before job execution.
- [x] [serial] r[molten.node_control_provenance.spec.trust_state] Emit provenance gate receipts as control subreceipts and show/list them through node tooling.

## Phase 3: CLI, coverage, and validation

- [x] [parallel] r[molten.node_control_provenance.spec.canonical_evidence] Add CLI/test synthetic reviewed provenance fixtures.
- [x] [parallel] r[molten.node_control_provenance.spec.trust_state] Cover missing provenance denial, queued loop denial, reviewed pass, tampered provenance denial, and sandbox-only profile behavior.
- [x] [serial] r[molten.node_control_provenance.spec.install_gate] Run full Molten validation gates.
- [x] [serial] r[molten.node_control_provenance.spec.canonical_evidence] Run Cairn strict validation with checked-out Cairn policy.
