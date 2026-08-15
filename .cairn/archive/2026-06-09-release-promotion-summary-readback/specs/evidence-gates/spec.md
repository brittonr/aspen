## ADDED Requirements

### Requirement: Release promotion summary record
r[molten.evidence.release_promotion.summary_record] Molten MUST emit a canonical `release-promotion-summary-v1` artifact that summarizes promotion receipt, signed promotion envelope, selected keyring key, source evidence, Octet evidence, Cairn evidence, diagnostics, and evidence-only checks.

#### Scenario: Summary binds promotion readback refs
- GIVEN a realized dogfood release output with promotion evidence
- WHEN release promotion summary generation runs
- THEN the summary binds the promotion receipt ref, signed envelope ref, signed subject ref, signed key ref, source ref, Octet ref, and Cairn ref

### Requirement: Release promotion summary CLI
r[molten.evidence.release_promotion.summary_cli] Molten MUST expose `molten dogfood release-promotion-summary` to write promotion summaries from a realized dogfood output and signed receipt keyring.

#### Scenario: CLI writes summary artifact
- GIVEN a dogfood release output and signed receipt keyring
- WHEN the summary CLI runs
- THEN it writes `release-promotion-summary-v1` to the requested output path
- AND prints a status line containing the summary decision and refs

### Requirement: Signed promotion summary readback
r[molten.evidence.release_promotion.summary_signed_readback] Release promotion summaries MUST verify the signed promotion envelope through the selected keyring key and require the signed subject ref to match the promotion receipt ref.

#### Scenario: Signed promotion readback passes
- GIVEN a passing promotion receipt and signed promotion envelope over that receipt
- AND a current unrevoked signed receipt key
- WHEN summary generation runs
- THEN the summary decision is `pass`
- AND it binds the signed envelope and key refs

### Requirement: Promotion summary denies stale or missing readback
r[molten.evidence.release_promotion.summary_deny_readback] Release promotion summaries MUST emit deny evidence when the promotion receipt is missing, not passing, bound to another output path, unsigned, signed for another subject, or signed by a missing/stale/revoked key.

#### Scenario: Missing signed promotion denies summary
- GIVEN a passing promotion receipt
- AND no signed promotion envelope in the dogfood output
- WHEN summary generation runs
- THEN the summary decision is `deny`
- AND diagnostics identify signed promotion verification failure

### Requirement: Promotion summary remains evidence only
r[molten.evidence.release_promotion.summary_evidence_only] Release promotion summaries MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, release publication authority, or permission to bypass subsystem gates.

#### Scenario: Summary does not replace subsystem gates
- GIVEN a passing release promotion summary
- WHEN a subsystem or release publisher requires its own authority or gate evidence
- THEN the summary MUST NOT be sufficient authority
- AND the subsystem MUST still require its own gate evidence
