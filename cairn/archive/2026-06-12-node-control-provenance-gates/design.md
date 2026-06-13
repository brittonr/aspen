## Design

Node control provenance gates are a local fail-closed preflight that runs after authority/resource checks and before any operation-specific side effect.

### Provenance artifacts

`provenance-record-v1` is canonical Preserves evidence with:

- `artifact`: the exact content/addressed artifact ref covered by the record.
- `trust-state`: one of `unknown`, `source-known`, `builder-attested`, `reviewed`, `reproducible-verified`, `sandbox-only`, `policy-trusted`, or `denied`.
- `source`, `dependency-closure`, `toolchain`, and `builder` refs.
- `review`, `tests`, `source-gates`, and `policy` refs.

`provenance-receipt-v1` records the local admission decision for an operation/profile/artifact tuple. Receipts include checks that make the trust boundary explicit: content hash identity is not itself trust, artifact refs are bound, trust state was evaluated, and the receipt is canonical.

### Node-control request evidence

`node-control-request-v1` gains an `evidence` sequence. The parser still accepts the earlier 9-field request shape and treats it as empty evidence for deterministic replay of existing receipts.

### Install gate

`install` evaluates provenance for the payload ref. Only reviewed, reproducible-verified, or policy-trusted provenance passes the `node-control` profile. Missing evidence, malformed records, wrong artifact bindings, and sandbox-only trust state deny before `artifacts::install_artifact` is called.

### Run gate

`run` parses the job execution request and evaluates provenance for the contained `job_ref` before reading the admission receipt or calling job execution loopback. Denial emits a provenance subreceipt and prevents job side effects.

### CLI fixture

`molten node provenance-fixture --artifact-ref ... --out ...` writes a synthetic reviewed provenance record. It is for deterministic local tests and operator workflows; production provenance will be replaced by real builder/review/source-gate records in later slices.
