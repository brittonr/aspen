# Full Aspen Hardening Audit Report

Generated: `2026-05-05T13:04:07Z`

## Summary

- Total findings recorded: 10
- Open high-risk findings: 0
- Open findings: 0
- High/medium findings were handled as focused direct remediation commits; no child OpenSpec remains required before archive.
- Residual supply-chain risk is explicitly scoped as non-claim, not a high-risk blocker for this umbrella.

## Findings

| ID | Severity | Status | Finding | Evidence | Follow-up |
| --- | --- | --- | --- | --- | --- |
| F-001 | medium | remediated | CI request status classification drift | evidence/authorization-matrix.md<br>evidence/authorization-matrix.json | 0019561ff |
| F-002 | high | remediated | Generic KV access over reserved internal prefixes | evidence/reserved-prefix-audit.md<br>evidence/reserved-prefix-audit.json | c6d659225 |
| F-003 | high | remediated | Domain APIs using generic internal-prefix capabilities | evidence/domain-capability-audit.md<br>evidence/domain-capability-audit.json | 6ddc9433a, 682cb782a, f17166360, 15e02c84d, 86a236dc4, bb3b323cb, 67e014c9a, 2838b8c22, 4aa1b12e5 |
| F-004 | high | guarded | Public/no-auth classification drift risk | evidence/negative-drift-tests.md<br>evidence/negative-drift-tests.json | 63fb46887 |
| F-005 | high | remediated | Token and ticket debug output exposed reconstructable material | evidence/token-ticket-redaction.md<br>evidence/token-ticket-redaction.json | a550b5a47 |
| F-006 | high | remediated | Persistent generated secrets and hook-ticket debug surfaces needed redaction/owner-only permissions | evidence/token-ticket-persistence-permissions.md<br>evidence/token-ticket-persistence-permissions.json | 7c39843e5 |
| F-007 | high | remediated | Forge/dogfood operator output could reveal Aspen ticket URL credentials | evidence/cli-log-output-redaction.md<br>evidence/cli-log-output-redaction.json<br>evidence/dogfood-receipt-redaction.md<br>evidence/dogfood-receipt-redaction.json | 8fd8417f9, 5a0dc1cda |
| F-008 | medium | remediated | CI shell executor working-directory symlink escape | evidence/execution-sandbox-command-boundaries.md<br>evidence/execution-sandbox-command-boundaries.json | a385ff78b |
| F-009 | high | remediated | Authenticated Raft outbound ALPN lacked matching production inbound router registration | evidence/transport-boundary-audit.md<br>evidence/transport-boundary-audit.json | 5f2eacd30 |
| F-010 | low | accepted_residual_risk | Supply-chain residual review scope remains manual beyond pin/hash invariants | evidence/supply-chain-boundary-audit.md<br>evidence/supply-chain-boundary-audit.json | No high-risk child change opened; RustSec/cargo-deny, vendored patch review, and release signing/attestation remain explicit non-claims for this audit. |

## Remediation ownership

All direct remediations are owned by Aspen maintainers and are landed on `main`. Source handles and evidence handles are listed per finding in `audit-report.json`.

## Archive decision

The report records no open high-risk findings. The umbrella can be archived after the final Phase 6 gate set passes and tasks are marked complete.
