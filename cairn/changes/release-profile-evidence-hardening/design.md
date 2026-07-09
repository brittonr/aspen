## Context

`docs/production-node-profile.ncl` currently validates shape and invariants well, but it is still a pilot fixture with placeholder source-gate refs. `cairn-policy/default.ncl` likewise keeps stack provenance optional with a placeholder accepted policy hash. Those defaults are useful while local evidence rails are being built, but release-review configuration should make fixture status impossible to confuse with release readiness.

## Decisions

### Deployment profile tier is explicit

**Choice:** Add a profile tier value or equivalent release-profile wrapper with `development`, `pilot`, and `release` semantics. Existing local fixtures remain pilot/development unless promoted through explicit release-profile validation.

**Rationale:** The current profile is named `pilot-node`; preserving that status avoids retroactively making placeholder refs look release-grade.

### Release tiers reject fixture refs

**Choice:** Release-scoped profile export and release promotion validation reject all-zero refs, obvious repeated-character dummy refs, and declared fixture placeholders unless an explicit diagnostic-only fixture profile is selected.

**Rationale:** Placeholder refs are valid as negative/fixture material, but not as release readiness evidence.

### Stack provenance is evidence-only but mandatory for release tier

**Choice:** Stack provenance remains non-authoritative, but release-tier evidence requires a current stack-provenance input whose accepted Valence policy hashes are reviewed non-placeholder BLAKE3 digests.

**Rationale:** Requiring the evidence improves review completeness without granting runtime authority or upstream verifier soundness.

### Freshness is checked separately from shape

**Choice:** Keep Nickel authoring contracts responsible for shape, vocabularies, and obvious placeholder denial. Freshness checks compare supplied canonical refs, generated JSON, and receipts in the release shell.

**Rationale:** Nickel should not become live runtime authority; release shells own filesystem reads and receipt comparison.

## Validation strategy

- Add positive development/pilot profile fixtures and a release fixture with reviewed non-placeholder refs.
- Add negative release fixtures for zero refs, dummy repeated refs, optional stack provenance, stale generated JSON, and missing Octet/Cairn/source evidence.
- Run contract export drift gate, focused release/profile tests, and Cairn validation/gates.

## Non-claims

Passing release profile validation does not prove deployed runtime correctness, source-code correctness, authority delegation, transport liveness, retention safety, or release eligibility by itself. It only proves release-scoped config has complete, non-placeholder evidence references and keeps evidence-only boundaries visible.
