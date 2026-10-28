# Immutable supported-world intake — 2026-10-28

## Outcome and ownership

Molten now pins published Mantle `e1827a8cc1cacf2810aa2314dc8971177832a8f0`.
The release dependency profile binds both the existing tree-verifier archive and
`.cairn/archive/2026-10-28-select-octet-component-profiles`, including exact
reviewed evidence digests. Cargo, Nix, source hashes and both generated build
plans select that revision. No sibling override is a product dependency.

This resolves the earlier import-free supported-world artifact-policy blocker.
It does **not** complete shared-runtime adoption. The complete local runtime,
test-only shared adapter, admitted Molten profile/WIT, effect-world activation
restriction, actor and extension composition roots remain unchanged.

The durable capability is reusable closed producer profile selection plus
product-owned verification of an unchanged supported-world materialization.
Mantle maintainers own profile selection, bounded tree copying and producer
receipts. Molten component-runtime maintainers own exact profile admission,
private snapshot lifetime and complete report agreement. Current consumers are
Molten's component intake and its pending actor/extension adoption; installation
still requires the separate end-to-end gates below.

## Exact immutable supported-world observation

The [actual producer execution](2026-10-28-supported-world-intake/producer-execution.json),
[request](2026-10-28-supported-world-intake/producer-request.json) and
[producer consumer report](2026-10-28-supported-world-intake/producer-consumer-report.json)
retain the genuine selected-profile publication. The selected closed Octet
profile is `molten-component-baseline`, identity
`24dc3346b817ee2645b000922a612a1988f63914b49688a8b72ebb758316c846`.

The [freshly linked immutable Molten public-API invocation](2026-10-28-supported-world-intake/immutable-intake.json)
accepted unchanged bundle
`blake3:8b622bd5b9364333f6b144715d8a7154ac31171c0783b690acec928fc999e429`
under unchanged supported Molten profile
`blake3:605a53191900ec2c8016cacc1e17c2338e9cda5423115dd5bb993e5082975910`.
It required complete Structural+Bytes agreement and retained exactly **3,700
component bytes** and **423 WIT bytes**. The WIT measurement matched the admitted
profile, and the component bytes matched the original producer artifact.
The successful private staging child was removed before return.

Adding an empty directory to the original source tree then denied a second
intake, removed that failed private lease and left the previously retained
component unchanged. The observer removed its own added directory. This is
source-identity and lifetime evidence, not atomic mutable-source capture.

[Exact linked package subjects](2026-10-28-supported-world-intake/linked-library-subjects.json)
and the [driver build](2026-10-28-supported-world-intake/driver-build.json) bind the
public API to the immutable Git-pinned provider, not a development override.
The [inert driver source](2026-10-28-supported-world-intake/intake-driver-source.txt)
records its assertions. It deliberately does not invoke the runtime or install
an actor or extension.

## Distinct effect-world blocker

The exact product contract still requires `execution_admitted=false` for
`molten.wasm.component.effects.v1`. The existing inactive host adapter does not
authorize changing that contract or installing a production linker.

A separate real generated-WIT `actor-effects` guest was compiled with the pinned
producer compiler and normalized with the actual producer `wasm-tools strip`.
The [binding invocation](2026-10-28-supported-world-intake/effect-binding-generation.json),
[inert guest source](2026-10-28-supported-world-intake/effect-probe-source.txt),
[original static observation](2026-10-28-supported-world-intake/effect-world-before-normalization.json)
and [normalized static observation](2026-10-28-supported-world-intake/effect-world-denial.json)
are retained. This probe is not a materialized production bundle or runtime
invocation.

The normalized artifact identity is
`53ce05c7a3b0234c36ca4dc27fc9c501bcf70563296dcd6c95f633c350ca2742`.
The unchanged selected `molten-component-baseline` checker exited **1**, status
`failed`, with **four disallowed imports**, **one duplicate custom section**
(`name`) and **one resource limit** (three tables exceed the one-table bound).
The import findings include the actual `effect-request` functions, a function
table and a numeric adapter import. Metadata normalization did not remove these
remaining blockers. No import allowance, table ceiling, WIT or execution
profile was widened to make this probe pass.

The missing prerequisite is a reviewed immutable producer/checker/product
cohort for the effect world, including its exact allowed surface and resource
mapping, followed by operated deadline, real actor/extension, effect and receipt
parity. Keep the change blocked and its eight remaining tasks unchecked.
Import-free intake cannot substitute for this cohort or authorize partial
cutover. No duplicate runtime mechanism is removed before acceptance.

## Repeatability and retained inputs

The [unchanged publication snapshot](2026-10-28-supported-world-intake/unchanged-producer-snapshot.zip)
has BLAKE3
`313db1ee6056694401847ddafa281fd2f40f124e7b63d196372718d1e7dbab5f`.
It preserves the original absolute publication locators (stored without the
leading slash), original directory structure and bytes. It also retains the
normalized effect probe under `effect-probe/normalized.wasm`. Replay must
restore declared locators in caller-owned roots, not edit the bundle or treat
the ZIP as a file-valued source-closure replacement. Successful observations
prove no compiler correctness, source trust, runtime authority, isolation,
durability, release eligibility or completed adoption.

The [immutable baseline commands](2026-10-28-supported-world-intake/baseline-checks.json)
passed **98 selected runtime/parity, actor ABI and extension tests**, zero failed
or ignored, and built the actual product library. The
[baseline log](2026-10-28-supported-world-intake/runtime-baseline.log) is retained.
The [targeted Nix invocation](2026-10-28-supported-world-intake/targeted-nix.json)
passed `release-dependency-profile`, `git-source-hash-binding`,
`wasm-component-profile` and `nextest-config` under the immutable pin.

The [full `nix flake check` invocation](2026-10-28-supported-world-intake/full-nix.json)
exited **1** at `world-promotion-octet-deny-all`: Octet rejects the 306-line
`src/world_replay/boundary/tests/admission.rs` with `excessive_file_length`.
The [full log](2026-10-28-supported-world-intake/full-nix.log) and
[attribution](2026-10-28-supported-world-intake/full-nix-attribution.json) are
retained. That file is unchanged from fetched `origin/molten`
`d4663cbdf4eba8b61607dcb242a86be42097445a`; the failed check was not rerun.
The targeted pin/profile checks above pass, but the broad Nix rail is not green.

## Canonical publication gates

The [executed strict sequence](2026-10-28-supported-world-intake/canonical-checks.json)
passed formatting; both workspace and library Octet producers with **zero
findings**; object-corpus receipt; artifact import; fail-closed `strict-ci`
source gate; remediation plan; workspace library tests (**1,782 + 630 passed**,
zero failed or ignored); workspace all-target Clippy with `-D warnings`; and
strict Cairn validation (`valid=true`, using the sibling provider's generated
policy). The complete [source-gate receipts](2026-10-28-supported-world-intake/source-gate/gate-receipt.preserves),
[workspace test log](2026-10-28-supported-world-intake/workspace-library-tests.log),
[Clippy log](2026-10-28-supported-world-intake/strict-clippy.log) and
[strict lifecycle report](2026-10-28-supported-world-intake/strict-cairn.json)
are retained. No `warning-only` result was accepted.

The [retained initial observer setup failure](2026-10-28-supported-world-intake/retained-setup-attempt.json)
occurred before gates started: its historical corpus operand named a file
already renamed from `producer_files.rs` to `producerfiles.rs`. Only that
observer operand was corrected; no product source or gate was weakened.
The [generated effect bindings](2026-10-28-supported-world-intake/effect-generated-bindings.txt)
and [producer toolchain manifest](2026-10-28-supported-world-intake/producer-toolchain-manifest.json)
complete the retained probe inputs. Temporary observer sources and build outputs
remain outside product source; tracked `.txt` copies are inert reproduction evidence.

The change is intentionally neither synced as a completed adoption nor archived.
Its eight end-to-end tasks remain open. This prerequisite publication does not
claim a passing broad Nix rail or authorize execution, release or cutover.

