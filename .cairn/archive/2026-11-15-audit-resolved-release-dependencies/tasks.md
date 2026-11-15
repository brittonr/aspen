## Implementation

- [x] [serial] Pin maintained cargo-audit and provide bounded offline default-profile capture. r[molten.dependency_audit.resolved]
- [x] [serial] Bind scanner, database, closure, source/build/lock/target/features and prerequisite evidence; deny stale inputs. r[molten.dependency_audit.resolved]
- [x] [serial] Evaluate source/SPDX/notice observations with dependency paths and exact expiring advisory exceptions. r[molten.dependency_audit.resolved] r[molten.dependency_audit.controls]
- [x] [serial] Exercise affected/source/unavailable-license/expired/stale-binding controls; unchanged real-database controls passed and retained under evidence/2026-11-13/controls/ (not rerun during capture). r[molten.dependency_audit.controls]
- [x] [serial] Capture the actual frozen default-profile workspace with exact release-evidence sources and normalize the same snapshot twice more; 741 packages/204 findings deny, all prerequisite statuses zero, report/summary byte-identical. Retain complete bounded evidence and comparisons under evidence/2026-11-13/; this binds the pre-documentation snapshot only. r[molten.dependency_audit.resolved] r[molten.dependency_audit.controls]
- [x] [serial] Parent completed final Nix rails and strict lifecycle validation before review/sync/archive and integration; the actual deny observation does not establish release readiness or remediate findings. r[molten.dependency_audit.resolved] r[molten.dependency_audit.controls]
