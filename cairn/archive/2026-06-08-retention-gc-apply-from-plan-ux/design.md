# Design: Retention GC Apply From Plan UX

## Receipt shape

`retention-gc-apply-v1` is a canonical Preserves artifact with:

- decision and mode (`apply`),
- subsystem, action, object, class, and requester,
- original `plan-ref`,
- recomputed `plan-ref`,
- optional retention receipt ref,
- optional tombstone ref,
- admitted evidence refs from normal destructive admission,
- diagnostics,
- checks for plan binding, recomputation before mutation, unchanged plan, passing plan decision, normal admission, tombstone binding, and evidence-only boundaries.

The apply receipt is deletion-safety evidence only. It does not grant authority or substitute for local policy, authority, supporting-evidence, reference-index, remote-GC, or imported remote-clearance evidence.

## Apply algorithm

1. Read the stored plan by `--plan-ref`.
2. Parse the embedded candidate and destructive evidence summary.
3. Recompute a fresh dry-run plan from the embedded candidate/evidence and current local retention store.
4. Run normal destructive retention admission from the embedded evidence.
5. Deny without writing retention receipts or tombstones when:
   - the original plan decision is not `pass`,
   - the recomputed plan ref differs from the original plan ref,
   - the recomputed plan decision is not `pass`, or
   - normal destructive admission denies.
6. Only after all gates pass, call the normal retention evaluation path so it emits the authoritative retention receipt and tombstone metadata.
7. Store the apply receipt under the retention store.

Recomputing may store the fresh plan artifact, but that artifact remains dry-run evidence and is not destructive authority.

## CLI

`molten test retention gc-apply-plan --root <root> --plan-ref <ref> [--receipt-out <path>]`

The command prints or writes the apply receipt and summarizes the original/recomputed plan refs, decision, retention receipt ref, tombstone ref, and diagnostic count.

## Safety properties

- Plan drift is detected by canonical ref mismatch before mutation.
- Re-apply after a successful tombstone sees a changed reference index and denies.
- Adding pins or changing clearance/admission state after planning denies before mutation.
- Denied applies still produce auditable apply receipts.
