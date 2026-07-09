## Tasks

- [ ] [serial] r[molten.artifacts.name_view_records] Define canonical name, alias, tag, and channel view records with scope, target refs, issuer, policy refs, evidence refs, previous-view refs, and tombstones.
- [ ] [serial] r[molten.artifacts.exact_ref_pinning] Require normative dependencies, transcript expectations, remote execution requests, migration recipes, and policy admissions to record exact artifact refs after name resolution.
- [ ] [parallel] r[molten.artifacts.name_ambiguity_denial] Add deterministic ambiguity and stale-view denial diagnostics for name resolution used in install, execution, storage, or release gates.
- [ ] [parallel] r[molten.artifacts.name_views_non_authority] Gate name view updates and resolutions so names remain discovery metadata and never grant capability, provenance, policy trust, source-gate trust, retention, or execution rights.
- [ ] [serial] r[molten.artifacts.name_view_validation] Add positive and negative fixtures for deterministic name resolution, exact ref pinning, ambiguous names, stale channels, unauthorized pointer updates, and name-only execution denial.