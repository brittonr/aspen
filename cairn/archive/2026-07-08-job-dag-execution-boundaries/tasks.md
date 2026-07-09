## Tasks

- [x] [serial] r[molten.job_dag.modularity.boundaries] Inventoried job DAG planning, admission, scheduling, worker, blob IO, coordination, receipts, and CLI shell ownership in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.job_dag.modularity.pure_plans] Extracted `plan_job_execution` as a pure execution-planning boundary returning structured effects without storage, transport, or executor side effects.
- [x] [serial] r[molten.job_dag.modularity.execution_trust] Preserved execution admission so blob presence, queue delivery, or lease acquisition cannot grant execution trust by itself.
- [x] [parallel] r[molten.job_dag.modularity.tests] Added positive admitted-plan tests and negative tests for missing provenance/admission, cycles, stale leases, missing manifests, and unsupported executors.
- [x] [serial] r[molten.job_dag.modularity.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
