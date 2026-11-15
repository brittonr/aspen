## Implementation

- [x] [serial] Pin the separately selected Rust 1.100 nightly and exact converter source, preserving default and Octet cohorts. r[molten.sample_pgo.cohort]
- [x] [serial] Author optional Nix baseline/candidate builders and bounded capture/evaluation entrypoint; root export integration remains parent-owned. r[molten.sample_pgo.cohort] r[molten.sample_pgo.capture] r[molten.sample_pgo.measurement]
- [x] [serial] Add disjoint native harness datasets and consumer-visible negative test sources (not execution evidence). r[molten.sample_pgo.measurement] r[molten.sample_pgo.replay]
- [x] [serial] Document host prerequisites, root integration, commands, result identities, and non-claims. r[molten.sample_pgo.cohort] r[molten.sample_pgo.capture] r[molten.sample_pgo.non_claim]

## Execution and review

- [x] [serial] Validate the experimental compiler/dependency cohort and run focused consumer tests. r[molten.sample_pgo.cohort] r[molten.sample_pgo.replay]
- [x] [serial] Execute real bounded sampling, repeated held-out measurements, and four-path canonical equivalence gates on an admitted host. r[molten.sample_pgo.capture] r[molten.sample_pgo.measurement] r[molten.sample_pgo.replay]
- [x] [serial] Review raw data and record a measured keep/revise/stop/inconclusive decision; do not introduce a default profile. r[molten.sample_pgo.measurement] r[molten.sample_pgo.non_claim]

No execution task is complete from source authoring. Missing external conditions remain explicit blockers.

Parent observation (2026-11-13): the actual selected Rust 1.100-nightly baseline
built, and all 10 consumer controls passed against that Nix-store binary,
including four-path canonical artifacts/replay, tamper rejection and stale
baseline rejection. A real bounded LBR capture/conversion retained 294
application sample weights. Candidate validation originally rejected the
imported record's foreign UID. The bounded owned-copy repair now passed a real
Nix sandbox control with the exact retained raw/baseline/converter: raw hashes,
baseline build-ID/path association and regenerated profile bytes all matched.
The verifier also accounts for perf's random unidentified temporary vDSO name
without discarding malformed/conflicting real associations. The sandbox result
is `/nix/store/vkfqifq63y09s2824g3r0lbm83214l3s-aspen-owned-profile-validation-control`.

Fresh-checkout baseline dry-run requires three local crate builds; the shared
store pool had 2,625,503,232 bytes free. The integration owner did not admit a
large two-variant/profile-consuming build with that reserve. Exact storage and
dry-run evidence is `/dev/shm/aspen-sample-pgo-feasibility-20261113.json`; this is
a resource-admission blocker, not an observed ENOSPC or a measured decision.
The retained profile is only a repair control for its original cohort and must
not be reused for changed source/binaries. At that resource-admission checkpoint,
neither repeated measurements nor a measured decision had completed.

After the repair, parent executed
`python3 -m unittest discover -s tools/sample-pgo -p test_experiment.py -k test_associations_`:
all four new association parser controls passed. These cover the unstable
unidentified vDSO boundary, preserved exact/non-vDSO associations, and fail-closed
malformed/conflicting-row handling. No unchanged consumer controls were rerun.

Retained small diagnostics: [verifier repair result](evidence/2026-11-13/result.json),
original binding/association, matching raw/profile hashes, build-time UID proof,
validation command/log records, original candidate failure, and storage/dry-run
feasibility observation. Raw/profile bytes remain at their existing exact
run/store locations, not duplicated into this change. Candidate replay and
paired measurements were unexecuted at that earlier checkpoint.

## Fresh measured observation (2026-11-14)

After explicit user admission of the recovered shared reserve, the real entrypoint completed a fresh immutable-source baseline, bounded LBR capture/conversion, independently validated candidate, canonical comparison and held-out measurement. The source was `/nix/store/ys9rpf8f3lv00fqcjyx4ncn13ypxqxnh-1gp0ya502nxnjpazbycqw13lb7hmfvm5-source`; no earlier profile was reused. The new profile retained 478 application sample weights. Success, denied, exhausted and error artifacts matched; only success/denied were replayable reports and both replayed successfully.

Three warmups preceded 30 alternating-order pairs on every path. All 240 measured baseline/candidate invocations had matching paired exit classes and artifact BLAKE3 identities, with no measured timeout. The predeclared decision is **revise**, not adoption: paired mean wall improvement was -46.22% with an approximate 95% Student-t interval of [-58.06%, -34.39%]; maximum paired peak-RSS growth was 8.14% and binary growth was 12.51%. These violate the existing keep rule. No maintained/default compiler or profile was enabled, and the policy/budgets were not changed to obtain a different decision.

Repository-owned result, raw pairs, source/profile/binary/dataset bindings, equivalence, realization records and raw-data review are retained under `evidence/2026-11-14/`. Full raw perf/profile, canonical per-invocation artifacts, logs and host facts remain at `/home/brittonr/git/OnixResearch/aspen/target/aspen-sample-pgo-fresh-20261114`; its result links retain the baseline/candidate Nix outputs. This completes the bounded experiment's execution/review criteria, not release qualification or permission to enable the profile. Final repository gates remain separate.

