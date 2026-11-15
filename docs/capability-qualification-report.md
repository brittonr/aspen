# Aspen capability qualification report

`tools/capability-report.rs` is a read-only diagnostic join of the reviewed
[evidence matrix](../tests/evidence-matrix.ncl), the small
[capability/family mapping](../config/capability-report/capabilities.json), and
actual retained native execution artifacts. Aspen's internal crate, CLI, schema
and requirement names still use `molten`; those identifiers are unchanged.

Implementation definitions and observed execution are separate. Source coverage,
authored tests, a passing source gate, logs, JUnit, and a caller-authored success
wrapper do not prove that the selected runtime executed. The old
`molten.capability-evidence.v1` JSON wrapper is deliberately not an accepted
execution producer. The v2 input is a clean cutover, not a compatibility shim.

## Exact subject and evidence families

The input names:

- `source_ref`: exact signed producer source NAR identity, not an unverifiable Git label;
- `build_ref`: measured executing ELF content identity;
- `dependency_closure_ref`: the complete retained measured build closure;
- `feature_set`: actual compiled Cargo features (an empty list means none, not `default`);
- `execution_profile`, `adapter_refs`, `platform`, and `environment`;
- `source_root_path`: an independently provisioned source-trust root used for readback;
- capability IDs and the supplied evidence bundle, each with requirement, coverage kind,
  family, finite scenario, exact subject, artifact path and canonical artifact ref.

References are full non-placeholder BLAKE3 identities. Native receipt and report
identities use **canonical packed Preserves**, not JSON bytes, debug formatting
or log text. Artifact paths and `source_root_path` are resolved relative to the
input file; they do not enter canonical report identity.

The reviewed mapping contains durable state/recovery, effect uncertainty,
cross-process transport, cross-host transport, world replay/promotion/restore,
and control-plane failover. It names existing requirements, implementation
owners, supported profiles and required evidence families only. Commands,
targets, positive/negative definitions, caveats and exemptions are derived from
the existing matrix, not maintained in a second test registry. Missing matrix
coverage is visible and cannot be replaced by a successful unrelated artifact.

Each applicable requirement/family needs both positive and negative execution,
or the existing matrix's explicit reviewed exemption (non-placeholder review
ref, reason and note). Exemptions remain visible and do not themselves become
observations. One passing requirement cannot qualify the rest. Every supplied
artifact must validate; an extra malformed or mismatched artifact prevents a
bounded-observed result. A canonical failure takes precedence over passes and
is retained with its attribution caveat.

`simulation`, `same-process`, `distinct-process`, `nixos-vm`, and `cross-host`
are separate closed environment values. A process hosting a deterministic
simulation is still **simulation**, not distributed-process or cross-host evidence.

## Reachable native producer

The additional `harness-runtime-capability-binding` capability has an actual
native execution producer. It uses the existing harness and fixed fixtures:

- `granted-send`: [the existing two-actor suite](../examples/two-actor.preserves);
- `denied-send`: [the missing-grant suite](../tests/fixtures/capability-qualification/denied-send.preserves),
  whose native report preserves denied admission without an effect.

`molten evidence qualify` first calls the existing current-process signed-source
admission, including real signatures, executing ELF measurement, and complete
closure/cohort verification. It then executes the real existing harness,
validates the native report and replays recorded effects. The native execution
receipt embeds its exact observed subject, source validation, fixed suite and
actual native report or failure. A failed run retains the canonical failure;
it is not turned into success. Requirement/family/scenario/coverage bindings are
fixed by the producer, not selected by a caller-authored decision field.

Readback calls the existing selected-output validator once, and invokes that
exact admitted immutable `bin/molten` through Bounded Exec in read-only
`readback` mode. The admitted process remeasures itself and reports its actual
compiled features, platform, fixed profile and adapter. It also executes and
replays the two fixed **pure simulation** scenarios without creating files or
using live ports, returning their actual canonical execution identities.
Receipt metadata and caller JSON must match those observed facts even when
both are edited together; successful supplied receipt identities must match
those actually observed executions of the selected binary. Native report
validation/replay and exact fixed-suite identity are checked again. A raw
valid harness report or authored carrier without that chain remains
inconclusive, not qualified for an arbitrary build. The separate `describe`
mode used to construct inputs reads metadata without running a scenario.

The source signer, approved Nix verifier/store/kernel and reviewed native
producer remain trust assumptions. A source signature is not a signature over
a run, nor remote attestation. This is a bounded diagnostic observation under
the existing native receipt trust model, not proof against a malicious authorized
producer, universal correctness or release authority.

### Execute and retain the real fixture

Build the existing source-bound output when authorized, then have its independent
source authority sign/select it and provision the source root as documented in
[node source admission](node-state-filesystem-authority.md). This tool never
creates a trust key, signs an output, synthesizes source admission or enables a
production profile. The selected output must contain the new CLI implementation.
An old signed output without it is unavailable, not a fallback success.

```sh
nix build .#molten-startup-source-binding --no-update-lock-file
# Independent source authority signs/selects that exact reviewed output and
# provisions SOURCE_ROOT/source-gate-trust.json; do not self-authorize a key.

nickel export --format json tests/evidence-matrix.ncl > /tmp/evidence-matrix.json
python3 scripts/qualify-capability-report.py \
  --source-root "$SOURCE_ROOT" \
  --matrix /tmp/evidence-matrix.json \
  --out target/capability-qualification-run
```

The fixture requires a new output directory, retains both actual receipts,
constructs the full input with all seven views, asserts the bounded simulation
row, rejects promotion of that evidence into the six unexecuted product rows,
and compares repeated canonical output bytes. Its parsed stdout refs are only
declarations: the consumer independently recomputes identity and validates the
native artifacts. The source verification may take minutes; it never skips the
measured closure to accelerate a diagnostic report.

To obtain exact subject JSON without running a scenario:

```sh
"$SIGNED_OUTPUT/bin/molten" evidence qualify \
  --source-root "$SOURCE_ROOT" --scenario describe --out -
```

### Report an existing input

```sh
cargo -q -Zscript tools/capability-report.rs \
  --matrix /tmp/evidence-matrix.json \
  --input target/capability-qualification-run/input.json \
  --json-out target/capability-qualification-run/report.json \
  --canonical-out target/capability-qualification-run/report.bin \
  --markdown-out target/capability-qualification-run/report.md
```

`--mapping` can select another reviewed mapping file. Unknown fields, duplicate
IDs, unlisted requirements/families and malformed exact subjects deny input.
Canonical output is the existing canonical Preserves projection of the ordered
machine report; JSON and Markdown are review views. Ordering and identity have
no timestamps, process IDs, path-derived subjects or input-order dependence.

## Execution statuses and producer limits

| Status | Meaning |
| --- | --- |
| `bounded-observed` | A positive exact-subject native observation, every required positive/negative obligation or reviewed exemption, and no conflicting failure or invalid supplied evidence. The finite scenarios and native refs remain attached. |
| `failed` | At least one validated canonical failure; a pass cannot override it. Raw unbound failure attribution is explicitly caller-declared. |
| `inconclusive` | Evidence exists but is partial, mismatched, unsupported as an artifact, malformed, unavailable, or not exact-subject-bound. |
| `not-recorded` | The selected mapping supports this profile/environment but no execution artifact was supplied; missing definitions/families remain listed. |
| `unsupported` | The selected profile/environment is outside the reviewed capability mapping. Evidence and diagnostics are still retained. |

Only the fixed harness simulation producer is admitted for observed qualification
by this increment. The six requested product-capability views remain visible,
with missing/unsupported scopes; this does not claim that their implementations
are absent or complete. Their native producers must supply their own actual
runtime artifacts, exact subject binding and native validators before those
families can become observed. Generic verification-run wrappers, source gates,
performance experiments, model receipts and the harness simulation cannot fill
that gap. In particular, VM startup failure, unavailable world adapters, hosted
actor integration and cross-host ownership evidence keep their original bounded
scope. A missing external checker cohort remains unavailable, never invented.

## Regression controls (not recorded live qualification)

```sh
cargo test --lib capability_qualification
cargo -q -Zscript tools/capability-report.rs --self-test
```

The controls exercise actual native harness artifacts; positive/negative
binding, feature/profile/platform/source/build/closure/adapter tampering (even
joint receipt/input rewrites), simulation-to-live substitution, stale source
gates, wrong fixtures, raw unbound successes, JSON wrappers, missing artifacts,
placeholder refs, canonical failures beside passes, partial requirements,
missing negative families, and canonical ordering. Explicit cfg(test) source
tokens test validator laws only and cannot enter the production admission API.
The standalone controls retain a standard temporary-directory guard because
repository cfg(test) workspace helpers are unavailable to a Cargo script.

No checks or live producer executions are asserted by this documentation edit.
The report grants no operation authority, release admission, source-trust bypass,
production-readiness badge or permission to reinterpret native receipt semantics.
