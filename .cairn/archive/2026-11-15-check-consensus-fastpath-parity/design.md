# Design

The authoritative finite instance remains three replicas, two commands and three views. A global original-path commit sequence models the base ordering assumption; per-replica applied projections can lag but must be prefixes. Application and reply sequences reject duplicates. Conservative distinct-command conflict classification is intentional. Fast-commit witnesses retain same-view acknowledgements, proposer promises and predecessor classification at the admission boundary so properties do not inspect unrelated later state.

Recovery separately begins a base-view change, agrees the preserved set, commits that set and an explicit marker, then resumes the matching acceleration view. Interrupted recovery retains accepted identities and recomputes the preserved set. Safety does not assume fair scheduling. Spec imposes weak fairness on the recovery progress action for RecoveryEventuallyResumes; it does not claim progress through permanent partitions.

The repository-owned Node adapter invokes the unchanged npm publication's exported check API with the same source/config supplied to TLC. The campaign requires complete successful searches to have identical generated and distinct state counts; the mutant requires the same named invariant violation with a nonempty trace. Early-stop mutant state counts are retained and compared separately because checker successor order can differ. No parser result is a semantic verdict. Raw outputs and elapsed times are retained without manufacturing observations.

The promise mutant is a config constant that removes only the proposer-promise guard. The admission witness still records the missing promise, enabling a short NoMissingPromise counterexample. This is a model mutant, not the separately requested Rust mutant.

Quint trace-to-Rust replay and optional Veil are distinct acceptance lanes. Their exact pins, APIs and implementation prerequisites must be documented; neither replaces required TLA+ parity. Root flake/dependency integration is owned by the parent. Tests/model campaigns/gates are deferred until integration.

The required Quint lane mirrors the existing pure Rust recovery/admission/ledger API rather than replacing those functions with a second implementation. A nested State projection compares phase, base/acceleration/last-normal views, accepted/recovery sets, marker presence, actual fast-admission result and application/reply outcomes after every step. All four RecoveryAction variants have named Quint actions, including denied transitions. Test composition supplies canonical operation fixtures and ordered convergence; it does not prove a base consensus engine's ordering implementation. Six profile properties name the bounded composition assumptions and outcomes explicitly.

Pin Quint 0.33.1 at source revision 3eaa85fc01820ea25bd7cb7baee0bb8d5c02c8b0 and quint-connect 0.1.2 at 4f018f54fc7dd4cef341d10111427bab59d3b307. A repository-owned patch removes the actual Rust mixed-view acknowledgement guard in an isolated source copy. The same replay test must then fail on the deterministic init/mixed-view-attempt fixture; no production mutation or test-local replacement core is installed. Generated ITF and replay output are execution evidence only after the parent runs the lane. Quint compilation to TLA+ depends on Apalache and has not established generated-source parity; the hand-written TLA+ remains the parity authority.

## Lifecycle owner

The delta modifies the existing `consensus` capability, which already owns the four selected `molten.consensus.fast_path_model` requirement IDs. Preserve their accepted obligations and scenarios, and leave the separate external-reference-conformance requirement untouched. `consensus-model-parity` is not a second capability owning copies of these requirements, and relabelling them as `ADDED` is not a valid native merge repair.

