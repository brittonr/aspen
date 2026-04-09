## Context

The session audit found real Tiger Style drift in Aspen: oversized handlers, zero-assert hotspots, recursive merge logic, and public or verified APIs that still expose `usize`. The problem was not the findings themselves. The problem was how they were captured.

The output was split across chat and off-repo agent files, so reviewers could not inspect the report from the repo. A note added to `.agent/napkin.md` described a scanner bug, but there was no tracked reproducer showing the failing shape or the intended parser rule. Because no production source files changed, the work could not reasonably be described as remediation.

Aspen also already has an archived `2026-03-23-tiger-style-remediation` change. That work focused on removing `unwrap`, `expect`, and `panic` from production paths. The current change should not reopen that broad campaign. It should make this audit reviewable, reproducible, and actionable, then land a small first refactor slice.

## Goals / Non-Goals

**Goals**

- Commit a durable Tiger Style audit report inside the repo.
- Provide a repo-local scanner command that other contributors can rerun.
- Add a regression fixture for the declaration-vs-body parsing bug discovered during the audit.
- Turn the current hotspot list into phased, file-specific remediation tasks.
- Land a first bounded remediation slice in production code so the change contains actual Tiger Style improvements.

**Non-Goals**

- Full repo-wide Tiger Style compliance in one change.
- Replacing every public `usize` use across all crates.
- General-purpose Rust parsing infrastructure for every style rule.
- Repeating the older unwrap/panic purge.

## Decisions

### 1. Keep the durable audit artifact inside the change directory

**Choice:** Store a human-readable `audit-report.md` and supporting command transcripts under this change directory.

**Rationale:** The review complaint was about durability and reviewability. A tracked report in the change directory solves that immediately, and the artifact remains available after archive.

**Alternative:** Keep the report only in chat or under `~/.pi/`. Rejected because reviewers cannot inspect or diff those artifacts from the repo.

### 2. Make the scanner reproducible before relying on its counts

**Choice:** Add a repo-local audit harness that can emit machine-readable output and can be exercised against committed fixtures.

**Rationale:** The original ad-hoc scan was good enough for exploration, but not good enough to support a durable engineering note or future gating. Reviewers need to rerun the same scan and see the same hotspot inventory.

**Alternative:** Preserve the inline one-off command only in the audit report. Rejected because it keeps the logic unowned and makes regression testing awkward.

**Implementation:**

- Add a repo-local Tiger Style scanner command.
- Emit at least a machine-readable inventory of hotspots and a human-readable summary.
- Version the scanner behavior through committed fixtures or targeted tests.

### 3. Capture the parser bug as a fixture, not just a note

**Choice:** Add a regression case containing trait method declarations followed by later impl bodies, and assert that declaration-only signatures ending in `;` are ignored when counting function bodies.

**Rationale:** The napkin rule is only useful if a future contributor can see the failing shape and the intended parser boundary.

**Alternative:** Leave the rule in `.agent/napkin.md` without repo evidence. Rejected because it recreates the same review problem.

### 4. Separate audit-only output from remediation output

**Choice:** The audit report must declare whether the current state is audit-only, remediation in progress, or remediation complete, and tasks must stay open until source changes and verification artifacts exist.

**Rationale:** The done review flagged a real ambiguity. A repo can have a good audit without having completed refactors. The change needs language and task structure that keeps those states separate.

**Alternative:** Treat any audit summary as proof of remediation. Rejected because it weakens review discipline.

### 5. Remediate by bounded hotspot slices

**Choice:** Start with a small number of high-value hotspots rather than a repo-wide rewrite.

**Initial slice:**

1. `crates/aspen-ci-handler/src/handler/pipeline.rs:handle_trigger_pipeline`
2. `crates/aspen-auth/src/capability.rs:Capability::authorizes`
3. `crates/aspen-auth/src/capability.rs:Capability::contains`
4. A small verified/public `usize` cleanup set in:
   - `crates/aspen-ci-core/src/verified/pipeline.rs`
   - `crates/aspen-forge/src/verified/ref_validation.rs`
   - `crates/aspen-cluster/src/gossip/discovery/helpers.rs`

**Rationale:** These functions were prominent in the audit, are small enough to attack without destabilizing the whole repo, and cover the main Tiger Style gaps: function length, assertion density, pure-helper extraction, and explicit-size APIs.

**Alternative:** Start with `crates/aspen-forge-handler/src/executor.rs:execute` or `crates/aspen-core-essentials-handler/src/coordination.rs:handle`. Rejected for the first slice because they are larger blast-radius handlers and better suited to a second phase after the audit harness and report exist.

## Risks / Trade-offs

**Scanner complexity drift** -> A quick scanner can grow into an unmaintainable partial Rust parser.
**Mitigation:** Keep the contract small: count bodies, ignore declaration-only signatures, measure assertions, detect a narrow set of Tiger Style smells, and cover each heuristic with fixtures.

**Refactor churn in hot handlers** -> Breaking large handlers into smaller helpers can ripple through tests and call sites.
**Mitigation:** Add characterization coverage first, extract pure helpers before changing orchestration, and keep each remediation task scoped to one hotspot.

**False precision in audit numbers** -> The scanner may still report directional counts rather than perfect truth for all Rust syntax.
**Mitigation:** Report methodology and known limitations in `audit-report.md`, and use the tool for prioritization rather than as a hard gate until the fixture set grows.

## Verification Plan

- Commit `audit-report.md` with the current hotspot inventory and scan method.
- Save the scanner command transcript and fixture transcript under `evidence/`.
- Add a regression test or fixture proving declaration-only trait methods are ignored.
- For each refactor task, save targeted test/build output under `evidence/` before checking the task.
- Run `scripts/openspec-preflight.sh durable-tigerstyle-audit` before claiming the change is complete.
