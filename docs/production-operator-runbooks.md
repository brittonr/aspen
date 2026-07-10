# Production operator runbooks

These runbooks are evidence-oriented. Terminal output and logs are diagnostics; review uses the canonical Preserves receipts named by each step.

Use a realized content ref such as `blake3:<64 lowercase hex>` for every `*-ref` argument. The sample refs below are non-placeholder review fixtures; all-zero, repeated-character, `fixture`, or `placeholder` refs are negative-test material only and fail release-tier validation.

```sh
REF0=blake3:8f5174292fe31f8fc364dc8f49560b21581f2cf01e54ae3fe8820c6d90d62f65
REF1=blake3:2ded4d8475648207836b950368aa4e1037b11b9aeb6f5b939482ad4d859664f7
REF2=blake3:e6cfe6b85e63f1eb8bbaf271586411e55885b51611497587164fd2c0adf0aed3
ZERO_REF_NEGATIVE=blake3:0000000000000000000000000000000000000000000000000000000000000000
```

## Deployment profile

The reusable Nickel contract module is `docs/production-profile-contracts.ncl`; the checked-in pilot instance is `docs/production-node-profile.ncl`. Evaluate the instance and run its positive/negative fixture rail before use:

```sh
nix develop -c nickel export docs/production-node-profile.ncl > target/production-node-profile.json
nix build .#checks.$(nix eval --impure --raw --expr builtins.currentSystem).production-profile-fixtures --no-link
```

The exported profile root carries `schema_id`, `schema_version`, `source_language`, and `profile_identity` metadata. Bind the reviewed profile content ref into canonical evidence; metadata identifies the profile evidence shape only and grants no authority, source-gate trust, adapter readiness, policy, resource, retention, or transport trust. Treat profiles as one of three review tiers: `development` permits local fixtures only, `pilot` permits bounded operator-readiness evidence with caveats, and `release` requires non-placeholder source-gate, policy, Octet, Cairn, stack-provenance, production-profile, generated-export, and accepted Valence policy hash refs for the exact source candidate.

Named production thresholds live in the contract module: `max_queue_depth`, `max_receipt_bytes`, `max_store_bytes`, `max_delivery_latency_ms`, and `max_recovery_time_ms`. Changing their exported numeric values requires threshold review and fixture expectation updates. Adding adapter, redaction, live-transport, startup, or shutdown vocabulary requires updating the allowed-value contracts plus a negative typo fixture before receipts are refreshed.

Bind the reviewed profile inputs into canonical evidence:

```sh
PROFILE_REF=blake3:<reviewed-production-node-profile-json-ref>

molten test prod-soak deployment-profile \
  --profile-name pilot-node \
  --schema-id molten.prod-ops.deployment-profile.v1 \
  --schema-version 1 \
  --source-language nickel \
  --profile-identity pilot-node \
  --profile-ref "$PROFILE_REF" \
  --state-layout-ref "$REF0" \
  --required-adapter-ref "$REF0" \
  --source-gate-ref "$REF0" \
  --resource-limit-ref "$REF0" \
  --redaction-setting-ref "$REF0" \
  --live-transport-ref "$REF0" \
  --startup-expectation-ref "$REF0" \
  --shutdown-expectation-ref "$REF0" \
  --diagnostic reviewed-profile-contract \
  --out target/prod/deployment-profile.preserves
```

Canonical review artifact: `target/prod/deployment-profile.preserves`.

Profile-backed daemon init consumes checked exports or reviewed refs only; it does not evaluate Nickel at runtime. Supply all selected adapter/profile refs explicitly and retain the profile-resolution artifact for startup review:

```sh
molten node init \
  --state-root target/node \
  --node-id node:pilot \
  --profile-ref "$PROFILE_REF" \
  --actual-profile-ref "$PROFILE_REF" \
  --profile-source-kind checked-export \
  --profile-tier pilot \
  --profile-identity pilot-node \
  --profile-state-root-ref "$REF0" \
  --adapter-profile ledger="$REF0" \
  --adapter-profile registry="$REF0" \
  --adapter-profile chunks="$REF0" \
  --adapter-profile storage="$REF0" \
  --adapter-profile cache="$REF0" \
  --adapter-profile remote-dataspace="$REF0" \
  --adapter-profile services="$REF0" \
  --adapter-profile jobs="$REF0" \
  --adapter-profile coordination="$REF0" \
  --adapter-profile plugin-host="$REF0" \
  --adapter-profile catalog-mcp="$REF0" \
  --adapter-profile control="$REF0" \
  --policy-ref "$REF0" \
  --capability-ref "$REF0" \
  --resource-ref "$REF0" \
  --effect-profile-ref "$REF0" \
  --profile-resolution-out target/prod/node-profile-resolution.preserves
```

No-profile `molten node init` remains available for local fixtures, but the stored profile resolution carries the `local-fixture-config` caveat and cannot satisfy release profile evidence.

## Init, run, status, stop

Follow the normal node path and bind the profile/source-gate receipts in the operator review bundle:

```sh
molten node init --state-root target/node --node-id node:pilot
molten node run --state-root target/node
molten node status --state-root target/node --health-out target/node.health.preserves
molten node stop --state-root target/node --shutdown-out target/node.shutdown.preserves
molten test prod-soak runbook-check \
  --runbook-name node-lifecycle \
  --operation init-run-status-stop \
  --canonical-artifact-ref "$REF0" \
  --denial-fixture-ref "$REF1" \
  --auxiliary-log-ref "$REF2" \
  --out target/prod/runbook.node-lifecycle.preserves
```

Review `startup-receipt.preserves`, `node.health.preserves`, `node.shutdown.preserves`, and `runbook.node-lifecycle.preserves`; do not treat stdout as pass evidence.

## Evidence export and source-gate refresh

```sh
molten receipts export "$REF0" --ledger target/node/ledger --out target/prod/exported-receipt.preserves
cargo octet check --artifact-dir target/octet
cargo run -- test octet gate --artifacts target/octet --profile strict-ci \
  --receipt-out target/octet/gate-receipt.preserves
molten test prod-soak runbook-check \
  --runbook-name evidence-and-source-gate \
  --operation export-refresh \
  --canonical-artifact-ref "$REF0" \
  --denial-fixture-ref "$REF1" \
  --out target/prod/runbook.evidence-source.preserves
```

The Octet gate is current production evidence only when it is generated for the same source candidate and policy under review.

## Backup and restore drill

Backups must bind ledger, Redb, chunks, identity metadata, retention pins, and source-gate refs. A passing drill also binds tamper-denial evidence.

```sh
molten test prod-soak backup-restore-drill \
  --drill-name pilot-backup-restore \
  --ledger-ref "$REF0" \
  --redb-ref "$REF0" \
  --chunk-ref "$REF0" \
  --identity-ref "$REF0" \
  --retention-pin-ref "$REF0" \
  --source-gate-ref "$REF0" \
  --restore-verification-ref "$REF0" \
  --tamper-denial-ref "$REF1" \
  --out target/prod/backup-restore.preserves
```

Denial example:

```sh
molten test prod-soak backup-restore-drill \
  --decision deny \
  --drill-name tampered-backup \
  --diagnostic missing-ledger-member \
  --out target/prod/backup-restore.deny.preserves
```

## Upgrade and rollback drill

```sh
molten test prod-soak upgrade-rollback-drill \
  --plan-name pilot-upgrade \
  --migration-ref "$REF0" \
  --smoke-ref "$REF0" \
  --rollback-eligibility-ref "$REF0" \
  --irreversible-exclusion-ref "$REF1" \
  --post-rollback-ref "$REF0" \
  --out target/prod/upgrade-rollback.preserves
```

If an irreversible migration or destructive retention operation lacks exclusion evidence, emit a deny receipt instead of claiming rollback safety.

## Observability and SLO snapshot

```sh
molten test prod-soak observability-slo \
  --snapshot-name pilot-snapshot \
  --adapter-health-ref "$REF0" \
  --queue-depth 8 \
  --max-queue-depth 64 \
  --control-loop-ref "$REF0" \
  --resource-pressure-ref "$REF0" \
  --retention-drift-ref "$REF0" \
  --source-gate-freshness-ref "$REF0" \
  --live-transport-ref "$REF0" \
  --import-export-failure-ref "$REF1" \
  --out target/prod/observability.preserves
```

For over-limit resource pressure, use `--decision degraded` so the canonical receipt reports degraded status without minting pass evidence.

## Security readiness

Threat model, drills, redaction audit, supply-chain review, boundary negatives, incident response, and the rollup report are all canonical receipts:

```sh
molten test prod-soak threat-model \
  --model-name pilot-threat-model \
  --threat "leaked live ticket" \
  --mapped-gate-ref "$REF0" \
  --drill-ref "$REF0" \
  --negative-suite-ref "$REF0" \
  --unresolved-risk-ref "$REF1" \
  --pilot-consequence-ref "$REF2" \
  --out target/prod/security.threat-model.preserves

molten test prod-soak security-drill \
  --drill-kind stale-ticket-denial \
  --scenario stale-ticket \
  --pass-evidence-ref "$REF0" \
  --denial-ref "$REF1" \
  --cleanup-ref "$REF2" \
  --out target/prod/security.drill.preserves

molten test prod-soak redaction-audit \
  --audit-name export-redaction \
  --surface-ref "$REF0" \
  --redaction-ref "$REF1" \
  --reveal-gate-ref "$REF2" \
  --plaintext-denial-ref "$REF1" \
  --out target/prod/security.redaction.preserves

molten test prod-soak supply-chain-review \
  --review-name pilot-supply-chain \
  --release-ref "$REF0" \
  --source-gate-ref "$REF0" \
  --provenance-ref "$REF0" \
  --build-verify-ref "$REF0" \
  --signed-keyring-ref "$REF0" \
  --sensitive-artifact-ref "$REF0" \
  --mismatch-denial-ref "$REF1" \
  --out target/prod/security.supply-chain.preserves

molten test prod-soak boundary-negative-suite \
  --suite-name boundary-negative \
  --preserves-parser-ref "$REF0" \
  --receipt-validator-ref "$REF0" \
  --source-gate-ref "$REF0" \
  --repro-bundle-ref "$REF0" \
  --node-ingress-ref "$REF0" \
  --provenance-ref "$REF0" \
  --plugin-hostcall-ref "$REF0" \
  --malformed-denial-ref "$REF1" \
  --out target/prod/security.boundary.preserves

molten test prod-soak incident-response-drill \
  --incident-kind leaked-ticket \
  --scenario leaked-ticket \
  --detection-ref "$REF0" \
  --containment-ref "$REF1" \
  --recovery-ref "$REF2" \
  --next-step-ref "$REF0" \
  --out target/prod/security.incident.preserves

molten test prod-soak security-readiness-report \
  --report-name pilot-security \
  --threat-model-ref "$REF0" \
  --supply-chain-ref "$REF0" \
  --drill-ref "$REF0" \
  --redaction-audit-ref "$REF0" \
  --boundary-suite-ref "$REF0" \
  --incident-response-ref "$REF0" \
  --unresolved-risk-ref "$REF1" \
  --pilot-recommendation limited-internal-pilot \
  --out target/prod/security.readiness.preserves
```

A report with unresolved risk refs must not recommend `broad-production`.

## Release-candidate and pilot decision

```sh
molten test prod-soak pilot-decision \
  --scope limited-internal-pilot \
  --allowed-workload stateless-internal-jobs \
  --denied-workload customer-critical-destructive-retention \
  --rollback-trigger stale-source-gate \
  --stop-condition failed-dogfood-replay \
  --operator-review-ref "$REF0" \
  --caveat octet-configuration-clean-until-source-remediated-zero \
  --out target/prod/pilot-decision.preserves

PILOT_REF=$(molten test prod-soak show target/prod/pilot-decision.preserves 2>/dev/null | awk '{print $3}' | cut -d= -f2)

molten test prod-soak release-candidate-gate \
  --candidate aspen-molten-pilot \
  --source-ref "$REF0" \
  --rust-validation-ref "$REF0" \
  --nextest-ref "$REF0" \
  --nix-check-ref "$REF0" \
  --cairn-validation-ref "$REF0" \
  --octet-ref "$REF0" \
  --dogfood-ref "$REF0" \
  --bundle-verify-ref "$REF0" \
  --promotion-ref "$REF0" \
  --export-verify-ref "$REF0" \
  --source-gate-status configuration-clean-caveat \
  --source-gate-caveat octet-configuration-clean-until-source-remediated-zero \
  --pilot-decision-ref "$PILOT_REF" \
  --out target/prod/release-candidate.preserves
```

If Octet source-remediated-zero evidence is unavailable, the candidate receipt can only support the named constrained pilot scope and must carry the caveat.

## Emergency stop

```sh
molten node stop --state-root target/node --shutdown-out target/node.shutdown.preserves
molten test prod-soak incident-response-drill \
  --incident-kind emergency-stop \
  --scenario operator-emergency-stop \
  --detection-ref "$REF0" \
  --containment-ref "$REF1" \
  --recovery-ref "$REF2" \
  --next-step-ref "$REF0" \
  --out target/prod/emergency-stop.preserves
```

Emergency receipts are evidence only. They do not grant authority, policy, provenance, retention, source-gate, or destructive-operation trust.
