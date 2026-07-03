## Context

The repository already uses Cairn requirement ids and has a large Rust/Nix test surface. What is missing is a first-class answer to: for this requirement, where is the positive test, where is the negative test, and which command or evidence artifact proves it? Without a machine-readable map, new normative requirements can be merged with only informal coverage.

## Decisions

### 1. Use a generated traceability manifest

**Choice:** Generate a deterministic manifest from accepted specs, active change deltas, verification markers, and configured test evidence. Each entry records the requirement id, requirement source, positive coverage, negative coverage, validation command, evidence artifact refs, and exemption status.

**Rationale:** A manifest gives humans and CI the same review object. It also avoids embedding coverage rules in prose-only docs.

### 2. Gate evidence-bearing requirements first

**Choice:** Start with requirements under the testing/evidence/release surfaces and any changed requirements in active packages, then allow explicit documented exemptions for non-executable documentation requirements.

**Rationale:** Requiring complete historical coverage in one step could block useful work. Evidence-bearing and changed requirements are the highest-risk gap.

### 3. Treat stale coverage as failure

**Choice:** The gate checks that referenced tests, commands, and requirement ids still exist. Missing test targets or deleted requirement ids fail closed unless an explicit exemption explains the gap.

**Rationale:** Traceability that silently rots is worse than no traceability because it creates false confidence.

## Risks / Trade-offs

- Some accepted requirements describe documentation or operator guidance rather than executable behavior; these need explicit exemption classes.
- Initial manifests may expose historical gaps; the gate should support a reviewed baseline while making new gaps fail closed.
- Test command refs should stay stable enough for review but not so rigid that harmless test file moves create noisy failures without diagnostics.
