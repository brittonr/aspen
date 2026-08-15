## Context

The dogfood Nix check currently writes report, release gate, summary, nextest marker, Nix evidence, and Nix verification receipt files. Each artifact is canonical, but a release reviewer must know which files constitute the complete review set.

## Goals

- Bundle the dogfood release evidence set into a canonical Preserves artifact.
- Verify the bundle by recomputing every member ref from the output path.
- Emit deny receipts, not log-only failures, for stale, missing, or tampered members.
- Preserve the evidence-only trust boundary.

## Non-Goals

- Do not sign release bundles in this slice.
- Do not make release bundles replace subsystem authority, provenance, policy, resource, source, retention, or destructive-operation gates.
- Do not introduce production-cluster release readiness.

## Bundle shape

`release-evidence-bundle-v1` contains:

- schema id,
- output path and output path ref,
- member file refs for dogfood report, release gate, summary, nextest marker, Nix dogfood evidence, and Nix verify receipt,
- dogfood report and release gate refs,
- Nix evidence and Nix verify refs,
- nextest marker ref and realized nextest check path,
- checks for report pass, release gate pass, Nix verify pass, bundle member binding, nextest dependency binding, evidence-only boundary, and no text oracle.

## Verification

`release-evidence-bundle-verify-receipt-v1` parses a bundle, observes the given output path, recomputes all member refs, checks dogfood report/release gate/Nix evidence/Nix verify consistency, and emits:

- `pass` when all refs match and the Nix verify receipt passes,
- `deny` with diagnostics for stale refs, missing members, tampered members, or output observation failures.

The verifier is allowed to write a deny receipt even when output observation fails, so CI and operators get canonical failure evidence rather than only stderr logs.

## CLI

```text
molten dogfood release-bundle-export --output-path OUT --out release-evidence-bundle.preserves
molten dogfood release-bundle-verify --output-path OUT --bundle release-evidence-bundle.preserves --receipt-out release-evidence-bundle-verify.preserves
```

## Nix integration

The `dogfood-local-node` check writes:

- `release-evidence-bundle.preserves`,
- `release-evidence-bundle-verify.preserves`,
- `release-evidence-bundle-verify.txt`.

The derivation fails unless the bundle verifier status line contains `decision=pass`.
