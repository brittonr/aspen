# Proposal checkpoint

## Current result

Structural validation is now unblocked. Task 10448 produced PASS receipts with no issues for proposal, design, and tasks on draft `bbb081328ff9bb4c086c8e841ce170ca02416e02`.
The tasks receipt records zero done and seven open tasks, four requirement blocks, and seven scenarios.
This is structural validation, not substantive design approval, an independent review, implementation acceptance, or startup authority.

Task 10446 tested three existing Cairn binaries with the unchanged project policy. The `xfkv...` and `5yk...` binaries returned valid repository reports; `j543...` retained the same policy-parse failure as the earlier `62a...` binary.
The selected document validator is `/nix/store/xfkvckw26s74x7yjf2lyb3l2ihbwyfsc-cairn-0.1.0/bin/cairn`, BLAKE3 `d6be1f637e19227bbb17a9974c0a45ab74a2ce183e66c9b697736ba4e84b8aa8`.
The gate receipts bind policy hash `d504b9473640f58c37425e4c101172bc46fba4e5489dbaecb3a8f5c633722752`.
No policy downgrade, compiler build, tool acquisition, or review-agent invocation ran.

An isolated copy passed the tasks gate. Removing its first concurrency marker and replacing its first requirement ID produced four explicit issues, top-level `valid: false`, and `verdict: FAIL`.
The CLI nevertheless exited 0. Task 10449 failed because its harness wrongly expected exit 1; that failed assumption and full receipt are retained.
Read the receipt decision, not process success alone. This combined mutation tests detection of those two defects, not comprehensive validator correctness.

## Conditional marker route

Task 10455 passed six reduced compiler controls in `evidence/verify-marker-route.sh`:

- Ordinary stable and nightly compilation: exit 0, no diagnostics.
- Active driver-injected marker cfg: exit 0, no diagnostics.
- Disabled marker under the active driver: exit 101 with the expected exhaustive-enum diagnostic.
- Added variant under stable compilation: exit 1 with E0004.
- Added variant under active Octet: exit 101 with E0004.

Each probe used an empty environment plus explicit inputs; none supplied RUSTC_BOOTSTRAP.
Only fixture metadata was compiled. Source/tool digests matched before and after; no ICE markers appeared.
The retained March-21 compiler, repaired driver, and exact-marker lint library were reused unchanged.

The earlier Rust-minimal 1.97.1 path was missing, so task 10453 stopped in preflight without executing probes.
An explicitly selected existing Rust 1.97.1 compiler was used for the stable fixture only:
`/nix/store/780x05ayqxz76pml1gi5jkar1pg0vk0g-rustc-1.97.1/bin/rustc`, BLAKE3 `ec1adcc9d4bb8c32050e01353a438da0d891a5734df463cc54ab1fd0ef12f6bf`.
It reports commit `8bab26f4f68e0e26f0bb7960be334d5b520ea452`, built from a source tarball.
This is not binary equivalence to the missing compiler or a replacement for the May-26 runtime cohort.

These are reduced compiler-route controls, not actual node-host mutation tests, Cargo manifest integration, full package compatibility, or runtime behavior proof.
Production annotations and features remain unchanged; all seven implementation/acceptance tasks remain open.
Private current evidence is `~/.local/state/onix/molten-node-vm/cairn-compat-1/`; earlier failed attempts remain below.

## Historical blocked checkpoint

At the original checkpoint no proposal/design approval, successful Cairn validation, tasks-gate result, or runtime acceptance was claimed.
Production source and startup authority are unchanged. The last full source gate remains run 15: 30 node-host errors, zero warnings.

## Attempted structural checks

- Task 10440: the previously retained `/nix/store/khx3cabpmd3zr9j0zkmfcs08jh2c5kk6-cairn-0.1.0/bin/cairn` was not executable; the command stopped before validation.
- Task 10441: an explicitly selected existing replacement did not support `--version`; no validation ran.
- Task 10442: the replacement attempted `validate --root .` but could not parse the existing project policy:

```text
error: failed to parse policy cairn-policy/generated/cairn-policy.json: policy has invalid field lifecycle_store_policy.project_config.required_fields
```

The tasks gate did not run because validation failed first.
The policy was not edited, downgraded, replaced, or bypassed to accommodate the validator.
No tool acquisition or build ran. No alternate validation policy was used.

Replacement identity, for document-check diagnostics only:

- `/nix/store/62a78ih8fnmil0nqvrqzdnrhkvl9ciif-cairn-0.1.0/bin/cairn`
- BLAKE3 `30f3a719b55dac84fe8069d05e1d14752d621b24d102d3ae1dffe9dce789b426`

Private full logs: `~/.local/state/onix/molten-node-vm/closed-domain-proposal/logs/`.
Observed task IDs can be reused after Pueue cleanup; use this directory and the saved commands to distinguish these attempts from earlier tasks.

## Next boundary

Use the selected compatible Cairn tool and inspect receipt decisions on subsequent structural checks.
Review the closed-domain contract before production edits. The conditional analysis-time route now has reduced compiler evidence; actual package integration and domain mutation controls are still required.
The proposed markers must document reviewed domain intent, not serve as a shortcut around the source gate.
Startup tasks 4–5 and all build/binding/VM/replay requirements remain open.
