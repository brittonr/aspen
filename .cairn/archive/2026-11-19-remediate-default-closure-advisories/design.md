## Context

The audit's 203 findings split into 67 advisories (38 vulnerability, 17 unsound,
12 unmaintained), 5 license-policy, 5 source-policy, 40 unavailable-license,
85 unavailable-notices and one profile-binding finding at `b6971ceab`. Of the
advisories, 13 crates have a patched version that a registry update can reach
without changing any pinned git revision.

## Decisions

### 1. Refresh only what the current semver pins allow

**Choice:** `cargo update -p <crate> --precise <patched>` for each registry crate
whose patched range intersects the existing requirement, then regenerate both
unit2nix plans.

**Rationale:** These are register-only moves: no git pin, `flake.lock` input,
`crate-hashes.json` entry or source hash changes, so the documented pin
procedure's git-source steps do not apply. Anything that needs a semver-breaking
transitive move is left to its owning upstream rather than forced with a
`[patch]` that would silently diverge from the stack.

### 2. Record the remaining four advisories instead of excepting them

**Choice:** No `[[advisory-exception]]` entry is added. The evidence names each
remaining advisory, the chain that carries it and the upstream move it needs.

**Rationale:** The audit policy requires an exception to carry owner, review and
a 90-day expiry. These four are not accepted risk; they are blocked upstream
work, and an exception would convert a known mismatch into a silent one.

### 3. Keep the pilot's Wasmtime cohort out of this change

**Choice:** The `wasmtime`/`wasmtime-wasi` 33.0.0 advisories are noted as outside
the audited closure because the component-RPC feature is opt-in.

**Rationale:** Moving that cohort changes the pinned wRPC/WIT contract the
archived pilot was qualified against; it needs its own change and re-qualification.

## Risks / Trade-offs

- A refreshed transitive can still change behaviour; `cargo check` plus the
  existing test rail is the only evidence here, and the lock diff is limited to
  14 crates.
- The audit remains deny: unavailable-license and unavailable-notices dominate the
  count and are package-evidence work, not dependency work.
