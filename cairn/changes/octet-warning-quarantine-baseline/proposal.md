## Why

Molten cannot honestly flip to strict Octet fail-close while the current workspace has thousands of warning findings. At the same time, leaving the gate advisory invites warning growth and makes future strictness harder. The safe transition is a quarantine baseline: capture the current warning set as explicit debt, block all new or escalated findings immediately, deny critical classes without review, and force the baseline to shrink until strict fail-close can replace it.

The baseline must be an evidence artifact, not a hidden suppression file. It should bind exact finding keys, source fingerprints, Octet config/profile hashes, review owners, expiry, and burn-down targets. Any drift not explained by removal or review must fail closed.

## What Changes

- Add canonical `octet-warning-baseline-v1` and `octet-baseline-receipt-v1` records for temporary quarantine of existing Octet/TigerStyle findings.
- Define stable finding keys over lint id, crate, source path, line span or fingerprint, message/category, config hash, and source/object fingerprint where available.
- Require no-new-findings semantics: new, moved, escalated, malformed, or unkeyed findings deny in quarantine profile.
- Require baseline shrinkage and expiry: every baseline refresh must reduce uncovered warning count or attach review receipts explaining why a finding remains.
- Prohibit quarantine coverage for critical classes unless a review receipt binds the exact finding, source fingerprint, risk rationale, and replacement plan.
- Make the baseline visible in catalog/ledger and downstream receipts so operators see that the gate is in quarantine rather than strict pass.

## Impact

This creates a migration path from the current `warning-only` Octet state to strict fail-close without pretending warnings are clean. It lets CI become fail-closed for regressions immediately, while making the remaining warning debt explicit, expiring, reviewable, and measurable.
