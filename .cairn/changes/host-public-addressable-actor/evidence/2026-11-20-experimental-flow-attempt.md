# Experimental addressable-actor flow attempt (2026-11-20)

## Scope

Authoring and one actual execution attempt under an explicitly experimental
signed source selection, experimental Ed25519 seeds, and an experimental
operator policy. This record is not a positive/negative native qualification
and does not check the change task.

## Authoring outcome

`molten-service-host-cohort create --profile counter` completed for a fresh
cohort and emitted:

```
<experimental-addressable-counter-cohort-authored-v1
  "blake3:f3b9b2acb44c4893daffeb45875b02bb41b9ebfe3e94a38489a92083d0a5ef2b"
  "blake3:25e8325f25f268e7115c66fb61400da432ff03c1b3ee466ede4fcc54677ed1af"
  "blake3:3088e2fa05c3bce66cd1e44afb821f90480099ef9cf0509a5d953abae066fbeb"
  "blake3:fbe89164a407cd85926f880f2eecf7fb2646e9aabea2234a4d530311ed1fd5e0"
  "blake3:ada6ab879a2fce8bd0749ea059082d1410839f025071af7eec0530eb9f521227"
  "blake3:1e6371c95168f35576dc656ec25342e1d4c1189927bb1a5bd284996329e81500"
  "blake3:c073119bdde38f2ded14f71de65a916c92bd1a3f6d4995e4d684937421cd2c7c"
  "blake3:8aad2f390ab215b3c739f9ae575656b89e3b9593d5f0326b94212831c8a03ace"
  "blake3:77ee028aff26b74329e2233590251227ecff499513a572291a763686aef97fa1"
  "authorship-only-execution-evidence-missing-no-live-pass-or-production-admission">
```

The authored cohort contains `counter-flow.py`, `dialogue.py`, the exact actor
configuration/placement selection, expected checkpoints and the admission
bundle. Authorship is not execution.

## Execution attempt

`counter-flow.py` was invoked with the same source trust, signed output,
issuer/holder seeds and policy, with a 900-second per-command timeout:

```
python3 "$COHORT/counter-flow.py" \
  --source-trust "$TRUST" --signed-output "$SIGNED_OUTPUT" \
  --issuer-key "$ISSUER_SEED" --holder-key "$HOLDER_SEED" \
  --policy "$POLICY" --timeout 900
```

It failed during the first bootstrap command,
`molten addressable-actor native --input counter-command-001.json` with
`{"kind":"install","group":"molten.counter.delivery"}`. The host process entered
its bounded retry wait and did not emit a signing request within 900 seconds;
the driver killed it, and the subsequent `READY` write raised
`BrokenPipeError`. No `counter-execution.json` was written.

The host was not observed to reject the operation; it did not reach the signing
boundary. The exact cause is therefore unresolved, not a policy denial and not a
successful installation.

## Non-claims

- The signing key is a locally generated experimental authority, not an
  independently authorized production operator key.
- No positive init/install/request/result/checkpoint/sleep/exit/restore/wake
  qualification, negative cohort, revocation, survival or ordering result is
  claimed.
- The 900-second command timeout may be insufficient under the observed host
  load, but a retry must use a fresh cohort and preserve the exact failure.

## Longer-budget retry (2026-11-20)

A fresh `counter3` cohort was authored with the same signed output, trust,
seeds and policy. The flow was rerun with `--timeout 3600`. The first
`native install` command completed after roughly fourteen minutes, including
the signed-source closure verification. The second `native install` command
began, and its independent `authorize` invocation was still verifying the full
signed closure after ten minutes; the retry was stopped before any terminal
result.

Finding: every `molten addressable-actor` invocation and every independent
authorizer invocation re-verifies the complete signed output and its closure.
The authored flow needs roughly fifteen such invocations. Under the observed
host load this exceeds a bounded session budget even though individual
commands complete. A future execution needs either substantially lower host
load or an admitted session-scoped verification/reuse design that preserves
the same exact source selection. This is a cost/latency blocker, not an
observed policy denial.

No `counter-execution.json` or negative-cohort result was written.

## Closure-cache fix (2026-11-20)

`node::source_binding::closure_cache` now retains one successful full closure
measurement per cohort, keyed by the exact output path, selected public key,
binding bytes, executing-binary measurement, `build-closure`, and all six graph
inputs. Signature authentication, executable measurement, retained-input
identity checks, gate validation and candidate admission still run on every
invocation; only the repeated NAR rehash is skipped on an exact hit. Missing,
malformed, or mismatched entries re-run the full measurement. The cache is
documented in `docs/node-state-filesystem-authority.md` and covered by focused
tests.

The signed output must be rebuilt and re-signed before the actor flow can
benefit, so no retry result is claimed here.
