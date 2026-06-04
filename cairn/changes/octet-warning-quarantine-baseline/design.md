## Context

The current full Octet run produces a warning-only status with thousands of findings. A full fail-closed strict gate should ultimately reject that, but Molten needs an immediate path that prevents further drift while warning debt is burned down.

A quarantine baseline is acceptable only as transitional evidence. It is not a suppression mechanism, not proof of safety, and not valid for strict release/admission profiles after its expiry.

## Baseline record

```preserves
<octet-warning-baseline-v1
  "molten.octet.warning-baseline.v1"
  <scope "workspace">
  <created-at "2026-05-31T00:00:00Z">
  <expires-at "...">
  <octet-config-hash "b3:...">
  <octet-profile-hash "b3:...">
  <toolchain "nightly-2026-03-21-x86_64-unknown-linux-gnu">
  <source-snapshot-ref "b3:...">
  <finding-keys [...]>
  <critical-finding-keys [...]>
  <allowed-profiles ["quarantine-ci"]>
  <burn-down <total n> <target-next n> <deadline "...">>
  <review-refs [...]>
  <checks [...]>>
```

A baseline receipt records comparisons between a new Octet run and the baseline:

```preserves
<octet-baseline-receipt-v1
  "molten.octet.baseline-receipt.v1"
  <decision "pass"|"deny">
  <baseline-ref "b3:...">
  <run-status-ref "b3:...">
  <new-findings [...]>
  <removed-findings [...]>
  <unchanged-findings [...]>
  <critical-unreviewed [...]>
  <expired? #f>
  <diagnostics [...]>
  <checks [...]>>
```

## Finding keys

Finding keys should be stable enough to catch regressions without hiding moved code:

- lint id and normalized lint family;
- crate/test target;
- normalized source path under workspace when possible;
- source span or nearest Valence/function-object fingerprint;
- rendered message category;
- critical surface marker/profile when available;
- Octet version/config/profile hash.

If a finding cannot be keyed, the baseline comparison must deny rather than silently ignore it.

## Quarantine profile semantics

The `quarantine-ci` profile may pass only when:

1. every current finding matches a baseline key or an attached review receipt;
2. no new finding appears;
3. no finding escalates severity or criticality;
4. no unreviewed critical class appears;
5. the baseline has not expired;
6. the warning count is less than or equal to the allowed burn-down target;
7. artifacts and comparison receipts are canonical and ledger-visible.

Strict profiles ignore quarantine pass unless explicitly configured for a temporary transition. Release, remote admission, node startup, and upgrade gates should require strict profile once the burn-down is done.

## Burn-down policy

The baseline must shrink monotonically by sprint or milestone. A refresh that keeps or increases warning count requires explicit review receipts and should deny release profiles. Current high-value buckets to burn down first:

- critical denial classes: `no_panic`, `no_unwrap`, `ambient_clock`, `unbounded_loop`, authority typing, secret rendering, harness backdoor;
- resource shape: unbounded collection growth and long loops on runtime/harness/job surfaces;
- Tiger Style structure: function length, too many parameters, oversized shell files;
- evidence caveats: missing object corpus/fingerprint linkage for critical paths.

## Non-goals

- Do not create permanent suppressions.
- Do not allow a quarantine baseline to stand in for strict source-gate pass evidence after expiry.
- Do not hide finding counts from operators or downstream receipts.
