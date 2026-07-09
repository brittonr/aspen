## Why

CLI harness tests already exercise many valuable happy and fail-closed paths. Some assertions still use terminal output as the primary signal. That is acceptable for user-experience smoke checks, but Molten's evidence model requires canonical Preserves artifacts and receipts to be normative.

Receipt-first CLI tests make regressions harder to hide behind unchanged strings and make CLI behavior align with the same evidence boundaries used by gates and release review.

## What Changes

- Audit CLI harness tests and classify stdout/stderr checks as diagnostic-only unless paired with parsed canonical artifacts.
- Prefer parsing report, failure, gate, repro, redaction, dogfood, and release receipts for normative assertions.
- Add negative CLI cases that prove malformed, diagnostic-only, stale, or missing evidence fails closed before pass evidence is emitted.
- Add shared CLI test helpers that make receipt parsing easier than string matching.

## Impact

The CLI suite becomes a direct test of the evidence contract, not just command UX. This reduces false confidence from brittle or overbroad text assertions.
