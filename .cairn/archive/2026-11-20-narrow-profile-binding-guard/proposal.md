## Why

Every audit run since the first refresh reports one `profile-binding` finding:
"resolved root features differ from the reviewed empty default profile". The
cause is not drift: `Cargo.toml` declares a self dev-dependency,
`molten = { path = ".", features = ["test-fixtures"] }`, so `cargo metadata`
always resolves the `molten` root node with exactly `["test-fixtures"]`.

The guard was written for a tree without that dev-dependency, so it can never
pass again — a permanent false positive that hides a real feature change if one
ever appears.

## What Changes

- Narrow the guard: the root feature set is reviewed when it is exactly
  `{"test-fixtures"}`, and still denies any other set — a new default, an
  optional-feature union, or an added dev feature.
- Add the matching controls: `["test-fixtures"]` passes, `["test-fixtures",
  "spacewasm-pilot"]` denies, and the existing `["spacewasm-pilot"]` and
  binding-level `["world-snapshot-vm-cohort"]` denials are unchanged.

Non-goals: no change to the audited closure, the binding fields, the scanner, or
any audit policy. This removes a false positive; it does not make the audit
clean.

## Impact

- **Files**: `tools/dependency-audit.rs`, `config/dependency-audit/controls.rs`.
- **Testing**: `cargo -Zscript tools/dependency-audit.rs --self-test`, the pinned
  audit re-run at one RustSec revision, and the repository gates.
