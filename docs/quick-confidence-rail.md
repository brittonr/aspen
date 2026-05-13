# Aspen quick confidence rail

The quick confidence rail is a bounded local preflight for operators who need broader confidence than one focused test without paying for full dogfood or gated runtime-host proofs.

Run it with:

```bash
scripts/test-harness.sh quick-confidence
```

For planning or docs checks without executing the rail:

```bash
scripts/test-harness.sh quick-confidence --dry-run
```

The rail writes a structured JSON summary to `target/quick-confidence/summary.json` by default. Use `--summary PATH` to choose a different output path, and `--json` to print the summary as JSON.

## Included checks

The initial bounded command set is intentionally composed from existing local checks:

- `scripts/test-harness.sh check` — verifies generated test-harness inventory freshness and manifest validity.
- `scripts/test-harness.sh runtime-host-acceptance-bundle` — checks runtime-host acceptance-bundle documentation, inventory, marker, and non-proof consistency without executing gated runtime hosts.
- `scripts/test-harness.sh public-api-boundary` — checks that the reusable `aspen-testing` default public API does not accidentally pull VM, patchbay, madsim, runtime-app, forge, CI, jobs, or Raft adapters into the default dependency graph.
- `scripts/test-harness.sh verus-trusted-boundaries` — checks that the residual `#[verifier(external_body)]` inventory still matches `docs/verus-trusted-boundaries.md` and has not drifted beyond the reviewed crypto/encoding/tuple assumptions.
- `cargo test --test operator_receipts_docs -- --nocapture` — keeps operator receipt safety documentation discoverable and consistent.
- `cargo test --test runtime_host_readiness_docs -- --nocapture` — keeps runtime-host readiness documentation and guardrails discoverable and consistent.
- `openspec validate --all --strict --json` — validates active and canonical OpenSpec state.
- `git diff --check` — catches whitespace and EOF issues before commit.

The summary reports every included check with its name, command, status, exit status, elapsed time, and a diagnostic pointer for failures. A failing check does not erase earlier successful check results from the summary.

## Explicit non-proof boundary

This rail skips expensive or environment-sensitive proofs. A passing quick confidence rail does **not** prove any of the following:

- full dogfood/self-hosting acceptance (`nix run .#dogfood-local -- full`);
- KVM/NixOS VM runtime-host execution;
- Uhyve/Hermit runtime-host execution;
- Hyperlight runtime-host execution;
- full `nix flake check` or ignored/network nextest profiles.

Use the full dogfood receipt flow for fresh self-hosting acceptance, and use the gated runtime-host proof commands documented in `docs/runtime-host-readiness.md` when promoting runtime-host rows.
