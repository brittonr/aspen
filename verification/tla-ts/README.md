# Consensus fast-path checker cohort

`tla-ts-0.0.10.tgz` is the **unchanged** npm publication of
`tla-ts@0.0.10` (MIT), SHA-256
`cf962f00ab297186f919e32c8f5cf28f6e78f1e92703b9a3d8f02044a487c84a`.
The upstream `LICENSE`, copyright and package/source metadata remain inside the
archive. Repository-owned adapters live beside it, never inside a patched fork.

`check.mjs SPEC CFG EXTRACTED_ROOT` imports the publication's real asynchronous
`check` API, requests one-worker TLA+ 1.7.4 semantics and final liveness checking,
and reports status, generated/distinct/queued state counts, failures and formatted
counterexample states. `parseModule` alone is not model checking.

`dive-release.json` pins the exact [Dive v0.1.0 Linux x86_64 archive](https://github.com/Chelis-Lang/dive/releases/download/v0.1.0/dive-v0.1.0-x86_64-unknown-linux-gnu.tar.gz):
tag commit `94cbbed9ae02ad3d30bbe2191bd2619a839b8795`, SHA-256
`cf0a0e2ac8fdccc2c7c73ca563ee567703a64b9ebd7f418eca6a4102263ab65d` and
version banner `dive 0.1.0 (v0.1.0-0-g94cbbed9)`. `LICENSE.dive` is the
upstream MIT notice. The original archive contains `dive`, a `tlc` shim,
`LICENSE` and bundled-library notices under `dive-v0.1.0-x86_64-unknown-linux-gnu/`.
Nix uses only the `dive` executable from that archive by absolute path and
does not install the shim or substitute for authoritative TLC.

`campaign.mjs NEW_OUTPUT EXTRACTED_NPM_ROOT DIVE_ARCHIVE DIVE_EXECUTABLE`
runs pinned TLC, TLA-ts and Dive on identical baseline and mutant inputs.
The Dive adapter invokes `check` with `--workers 1 --compat=tlc --output json`.
It maps schema 3, requires a completed exhaustive temporal baseline, and
rejects refusal, budget stop, timeout, drift or malformed/incomplete reports.
The baseline must succeed with the same generated and distinct counts as TLC
and an empty queue in all three tools. The missing-promise mutant must violate
`NoMissingPromise` in all three with more than one trace state; early-stop
counts are retained but need not agree. Each checker/config has 180 seconds;
all six sequential runs share 720 seconds.

Run `nix build --keep-failed .#checks.x86_64-linux.consensus-fastpath-model-parity`.
The existing Nix check retains actual durations, options, byte-exact stdout
and stderr, normalized summaries and Dive's native JSON reports in addition
to the model and configs, with BLAKE3 bindings in `campaign.json`. The
campaign is finite-model evidence only; Dive agreeing with TLC does not prove
the Rust implementation. See [`docs/consensus-fastpath-tla.md`](../../docs/consensus-fastpath-tla.md)
for the commands, hazard mapping and separate Quint and Veil scope.

## Updating the publication or release pin

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

For Dive, verify the exact tagged commit and asset digest, preserve the
upstream notices, review the released JSON schema and exit semantics, and
update the pin, adapter, tests and docs together. Do not use an ambient Dive
or silently skip a refusal.
