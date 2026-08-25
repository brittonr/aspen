# Align Octet metadata hashing

## Why

Molten recomputes Octet configuration identity before source-gate admission. Pinned Octet serializes metadata hash inputs with lexically ordered JSON object keys. Molten uses `serde_json` with insertion-order preservation enabled through workspace feature unification. Every genuine Octet run therefore appears stale and cannot satisfy configuration-current evidence.

## What Changes

- Match pinned Octet revision `fc38f59330b626961d166febfdf1a5aa6575460f` canonical metadata field order.
- Replace feature-sensitive JSON map construction with typed, ordered serialization payloads.
- Add positive canonical-order and negative changed-input tests.
- Regenerate strict gate evidence against the pinned Octet tool.

## Impact

This change affects Octet metadata freshness evaluation only. It does not suppress findings, make warning-only output pass, or weaken any strict source-gate check.
