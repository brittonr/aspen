# Change: production-profile-named-units

## Why

The production profile currently stores resource-limit literals directly in the concrete profile. Reviewers must infer whether a value is bytes, milliseconds, queue entries, or a deliberate production threshold. That makes later changes harder to audit and increases the chance of unit mistakes.

## What

- Introduce named Nickel unit constants and named production threshold constants for bytes, milliseconds, queue depth, receipt size, store size, delivery latency, and recovery time.
- Rewrite the concrete profile to reference names rather than raw numeric literals.
- Keep exported JSON values stable for the current reviewed profile.

## Impact

Profile review becomes clearer without changing runtime inputs. Diffs explain which reviewed threshold changed instead of showing only an unexplained number.
