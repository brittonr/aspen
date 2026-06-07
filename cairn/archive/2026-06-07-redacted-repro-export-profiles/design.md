# Design: redacted repro export profiles

## Profiles

- `deny-sensitive`: current default. Any forbidden sensitive marker blocks sealed export.
- `redacted-diagnostic`: replaces selected sensitive subtrees with deterministic redaction markers and emits a transform receipt. This profile is diagnostic-only unless a policy gate explicitly names the transform as gate-preserving.
- `encrypted-private`: replaces sensitive subtrees with validated encrypted refs. Recipients need explicit reveal authority and reveal receipts to unpack private material.

## Receipts

A redaction transform receipt should be canonical Preserves:

`<redaction-transform-receipt-v1 "molten.harness.redaction-transform.v1" ...>`

It binds:

- source report ref;
- source suite ref;
- redaction policy ref;
- export profile;
- transform manifest ref;
- output bundle ref;
- loss classification (`gate-preserving`, `diagnostic-only`, or `requires-reveal`);
- checks for marker coverage, deterministic traversal order, and forbidden cleartext absence.

## Encrypted refs

`<encrypted-ref ...>` is currently forbidden. This change defines the conditions under which it can be accepted:

- algorithm and envelope schema are explicit;
- plaintext content ref is never embedded in public artifacts unless policy permits it;
- recipient/key policy is represented as Preserves evidence;
- reveal operations emit canonical reveal receipts;
- failed, partial, or stale reveal evidence cannot satisfy pass gates.

## Gate behavior

Pass gates remain conservative. A redacted bundle is accepted only when the transform receipt is present, hashes correctly, covers every sensitive marker, and the policy profile says the transform is gate-preserving. Otherwise the bundle can be unpacked only as diagnostic evidence.
