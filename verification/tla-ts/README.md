# TLA-ts semantic checker cohort

`tla-ts-0.0.10.tgz` is the **unchanged** npm publication of
`tla-ts@0.0.10` (MIT), SHA-256
`cf962f00ab297186f919e32c8f5cf28f6e78f1e92703b9a3d8f02044a487c84a`.
The upstream `LICENSE`, copyright and package/source metadata remain inside the
archive. Repository-owned adapters live beside it, never inside a patched fork.

`check.mjs SPEC CFG EXTRACTED_ROOT` imports the publication's real asynchronous
`check` API, requests one-worker TLA+ 1.7.4 semantics and final liveness checking,
and reports status, generated/distinct/queued state counts, failures and formatted
counterexample states. `parseModule` alone is not model checking.

`campaign.mjs NEW_OUTPUT EXTRACTED_ROOT` runs the authoritative TLA+ source and
identical baseline/mutant configs through pinned TLC and TLA-ts. It verifies the
archive pin, rejects incomplete/unexpected results, compares exhaustive counts,
and retains raw outputs and actual durations. A checker/config has 180 seconds;
the four-run campaign has 720 seconds. The deliberate promise mutant must
violate `NoMissingPromise` in both tools; early-stop counts remain separately
reported prefix observations, not exhaustive counts.

The Nix rail is defined in `nix/consensus-fastpath-model.nix`. Parent root wiring
must replace the former TLC-only/parse-only checks with the combined semantic
rail. See `docs/consensus-fastpath-tla.md` for complete hazard mapping,
commands, independent Quint/Veil status and explicit unexecuted-evidence status.

## Updating the publication pin

Use the repository dependency-import/pin review process: retrieve the exact npm
publication (for example `npm pack tla-ts@VERSION --ignore-scripts` in a dedicated
import directory), verify the registry integrity/provenance and archive hash,
review the published API and transitive tool cohort, and retain all upstream
license/notice files. Replace the publication as a whole, record its exact
version/hash and corresponding upstream revision when supplied, and update the
repository-owned pin assertions, license inventory and docs together. Never
hand-edit tar members or claim an unavailable upstream revision. Re-execute both
baseline and mutant semantic campaigns; a parse or an upstream benchmark is not
Aspen's verdict, state-count or speed evidence.
