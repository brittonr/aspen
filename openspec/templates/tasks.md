# Tasks Template

Use `tasks.md` to decompose an OpenSpec change into bounded, dependency-aware work.

## 1. Implementation

Implementation tasks should be small enough to complete and verify with a focused evidence set.
Prefer one coherent requirement/scenario family per task.

Guidance:
- Keep implementation task `[covers=...]` fan-out to three requirement/scenario IDs or fewer.
- Split tasks that cover multiple independent deliverables, unrelated files, or more than three IDs.
- Put foundation/prerequisite work before dependent implementation tasks.
- If a task must mention a later dependency, add an explicit `Prerequisite:` note explaining why the order is still safe.
- Do not hide implementation and verification in the same checkbox; add a separate verification task.

Examples:

- [ ] I1 Implement bounded parser fixture support. [covers=domain.parser.happy]
- [ ] I2 Split oversized storage/auth delivery into separate storage and auth tasks. [covers=domain.storage.write,domain.auth.ticket]
- [ ] I3 Implement runtime adapter after foundation types are stable. Prerequisite: I1 defines the shared type boundary. [covers=domain.adapter.runtime]

## 2. Verification

Verification tasks may cover broader requirement sets only when they are explicitly labeled as integration proof and name expected evidence.

Guidance:
- Broad verification tasks must say `integration proof` or `integration verification`.
- Broad verification tasks must name the integration boundary and expected evidence artifact/transcript/report.
- Otherwise, split verification tasks by requirement family.

Examples:

- [ ] V1 Add focused fixtures for parser happy/error paths. [covers=domain.parser.happy,domain.parser.error]
- [ ] V2 Run integration proof across parser/storage/auth boundary and save evidence transcript. [covers=domain.parser.happy,domain.storage.write,domain.auth.ticket,domain.integration.e2e]
