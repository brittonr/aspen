# Design: Nix dogfood release evidence binding

## Canonical evidence

`molten dogfood nix-release-export --output-path PATH --out FILE` reads the Nix dogfood output files and emits `nix-dogfood-release-evidence-v1` with:

- the output path string and a domain-separated path ref;
- dogfood report ref;
- release-gate receipt ref;
- human summary text ref;
- after-nextest marker ref and nextest check path;
- preserved file refs for report, release gate, summary, and nextest marker;
- checks that classify the artifact as evidence-only.

## Verification

`molten dogfood nix-release-verify --output-path PATH --evidence FILE --receipt-out FILE` recomputes all refs from the output path and emits `nix-dogfood-release-verify-receipt-v1`. Mismatched report, release-gate, summary, marker, path, or file refs produce a deny receipt with diagnostics.

## Nix check integration

The `checks.x86_64-linux.dogfood-local-node` derivation writes report/gate/summary/after-nextest files, exports canonical Nix dogfood evidence against `$out`, verifies it, and preserves both receipts in the check output.

## Trust boundary

The evidence binds release review artifacts only. It does not grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to skip subsystem-specific gates.
