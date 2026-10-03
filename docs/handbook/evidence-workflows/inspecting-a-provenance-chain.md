# Inspecting a provenance chain

Mode: How-to

## Goal and prerequisites

Determine whether a supplied provenance record and, when relevant, build-verification receipt support one exact artifact under one operation/profile. This is an inspection procedure, not permission to install or execute the artifact. Consult the [Handbook](../README.md) for adjacent operations and the [evidence/authority companion](../../technical/foundations/evidence-and-authority-separation.md) for the theoretical boundary.

You need the actual subject's canonical content ref, the supplied Preserves record files, the requested operation, and a reviewed profile. Obtain these from the evidence producer, not from illustrative README hashes. Have an existing `molten` binary available and choose new output filenames in an isolated review directory. All commands below are source-checked, not executed for this batch; no runtime verification is claimed.

## 1. Decide what “chain” means here

Supply-chain provenance connects an artifact to source, dependency closure, toolchain, builder, review, test, policy, and optional build-record refs. It is not automatically a scoped hash chain. `src/evidence/chain.rs` owns a separate continuity structure keyed by scope, id, and epoch. A provenance record can be stored or referenced by such a chain, but neither relationship should be inferred from filenames.

Start a review note with the artifact ref and requested operation/profile. If those are unavailable, stop: selecting a record because it happens to parse cannot answer the subject-specific admission question.

## 2. Read the supplied record before evaluating

Use the provenance-specific reader, not the operator receipt reader. The latter supports a bounded set of dogfood/operator receipt kinds rather than arbitrary provenance records.

The following block requires you to set `PROVENANCE_FILE`, `ARTIFACT_REF`, `OPERATION`, `PROFILE`, and `EVALUATION_OUT` from actual review inputs. The output path must be fresh; the guard prevents an existing file from being overwritten. Command spelling comes from the inspected [root declaration](../../../src/main/root/parts/command/p000/body.rs), [provenance declaration](../../../src/cli/workflow/provenance/command.rs), and [handler](../../../src/cli/workflow/provenance/ops.rs), reached through [main aliases](../../../src/main.rs). Source-checked; not executed.

```sh
: "${PROVENANCE_FILE:?Set the supplied provenance file}"
: "${ARTIFACT_REF:?Set the actual canonical subject ref}"
: "${OPERATION:?Set the requested operation}"
: "${PROFILE:?Set the reviewed profile}"
: "${EVALUATION_OUT:?Set a fresh evaluation output path}"
test -f "$PROVENANCE_FILE" && test ! -e "$EVALUATION_OUT" &&
  molten test provenance show "$PROVENANCE_FILE" &&
  molten test provenance evaluate --operation "$OPERATION" \
    --profile "$PROFILE" --artifact-ref "$ARTIFACT_REF" \
    --provenance "$PROVENANCE_FILE" --receipt-out "$EVALUATION_OUT" &&
  molten test provenance show "$EVALUATION_OUT"
```

This first evaluation deliberately supplies no build verification. It can expose that missing prerequisite instead of hiding it. `show` renders a summary; retain the Preserves inputs and evaluation receipt as the review evidence. Inspect `decision`, diagnostics, and the matched record ref, not merely the shell exit status: the handlers return successfully after writing an ordinary deny evaluation.

## 3. Apply the operation/profile decision

The accepted profiles are `node-control` and `local-test`. Ordinary node-control evaluation accepts reviewed, reproducible-verified, or policy-trusted states. Ordinary local-test evaluation also accepts sandbox-only. The explicit sensitive operation names are `install-policy-artifact`, `install-migration-recipe`, `install-production-executable`, and `remote-sync-execute`; those use the stronger threshold.

Do not change the operation spelling or downgrade to local-test to manufacture a pass. The inspected threshold function recognizes those exact sensitive strings, so a review must match the consuming subsystem's actual operation. Parsing a valid trust-state string does not prove the asserted review or policy took place.

## 4. Follow the build binding when required

For a `reproducible-verified` record, inspect three equalities: the evaluation subject equals both expected and actual artifact refs in a passing build-verification receipt; that receipt's build-record ref appears in the provenance record's `build-records`; and the supplied build-record value hashes to that ref. The binding helper checks the first two relationships against parsed receipts. Resolving and reviewing the build record remains part of your evidence collection.

If you have the actual verification file, use another fresh evaluation output and add the declared `--build-verification` option to the evaluation invocation above. Preserve the earlier deny receipt rather than overwriting it. `verify-build` itself compares an expected ref with a caller-supplied actual ref; it does not launch Nix, rebuild a package, or independently discover the produced artifact.

There is a source-review nuance: `policy-trusted` satisfies the strong threshold, while the extra build-binding branch runs specifically for `reproducible-verified`. The threshold metadata flag must not be read as proof that every accepted state traverses that branch.

## 5. Resolve a concrete mismatch without changing the claim

The checked-in test `reviewed_provenance_passes_node_control_and_wrong_artifact_denies` evaluates a synthetic reviewed record against its own subject, then against another canonical artifact ref. The second evaluation denies even though the record remains well formed. In a real handoff, obtain evidence for the requested subject or correct an independently established selection mistake; do not edit the record's artifact field to match the request.

Finish with the exact subject, operation/profile, selected record, receipt decision, unresolved referenced objects, and remaining subsystem gates. A provenance pass is evidence for that evaluation only, not authority, live execution, or release readiness.

## Sources

- [Handbook](../README.md)
- [Evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [README provenance diagnostics](../../../README.md#supply-chain-provenance-diagnostics)
- [Provenance matching and evaluation](../../../src/provenance/parts/mod/p001/body.rs)
- [Build bindings and thresholds](../../../src/provenance/parts/mod/p002/body.rs)
- [Wrong-subject and profile tests](../../../src/provenance/parts/mod/tests/m000/p000/body.rs)
- [CLI declaration](../../../src/cli/workflow/provenance/command.rs)
