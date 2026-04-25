# Review remediation: task scope correction

Same-family review found tasks 1.1, 1.4, and 1.6 were checked before their full runtime/discovery scope existed.

Corrections:

- Task 1.1 is unchecked until Forge capability discovery returns node-specific routing identifiers for active backends instead of empty `backend_routes`.
- Task 1.4 is unchecked until JJ session admission or handler code enforces transport-version compatibility before object exchange.
- Task 1.6 is unchecked until plugin registration/activation lifecycle rejects protocol identifier collisions, not just manifest helper validation.

Task 1.3 remains checked because repository metadata now defaults missing backend metadata to Git-only behavior and current Forge repo responses populate Git-only backends.
