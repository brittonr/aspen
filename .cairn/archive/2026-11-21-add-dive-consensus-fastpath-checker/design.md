## Context

The TLC/TLA-ts campaign at issue #5 established finite-model parity for the baseline and negative control. Issue #62 records a separate upstream release trial; it does not establish that the integrated check has run.

## Decisions

### Decision: Pin one release, with no substitute

**Choice:** Fetch the GitHub `v0.1.0` x86_64 Linux release by its exact SHA-256. Use only its `dive` executable, not its `tlc` shim. Record the full tag commit and verify the whole `--version` banner before checking either config. Retain the original MIT notice and archive provenance.

**Rationale:** Nix validates the fixed-output archive. The direct executable path prevents PATH substitution and the extra version check catches drift. Reject floating Nix or Cargo providers and substituting Dive for TLC or TLA-ts.

### Decision: Admit only the completed expected report families

Run the baseline then the mutant, both ordered TLC, TLA-ts, Dive. Keep the existing per-run and total deadline. The runner binds process status, arguments, byte-exact streams and elapsed time to the observation. A pure schema-3 reader admits only a completed temporal baseline or a safety invariant counterexample; the parity gate compares counts to authoritative TLC only for the exhaustive baseline. Preserve the native report in addition to the normalized result; bind retained files with BLAKE3.

### Verification

Use synthetic success, refusal, budget stop, incomplete capture, version drift, count mismatch and wrong-property inputs. The parent must run the focused parity derivation and inspect its actual retained evidence; synthetic fixtures and previous trials cannot close that gate.

## Risks / Trade-offs

- A TLC-profile temporal check is a final check; TLC's lasso choice is not promised and this mutant is a safety violation. Six sequential runs share the existing 720 second campaign budget and may fail on slower hosts. Do not loosen limits or change the model. Evidence applies only to the finite design model and does not prove the Rust implementation.
