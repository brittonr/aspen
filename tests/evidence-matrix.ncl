# Checked-in testing-harness evidence matrix.
# Runtime gates consume exported or parsed data; this Nickel source is the reviewed authoring contract.
#
# Every row is bound to exactly one executed command and to refs computed from real artifacts:
# - hardening rows: the input fixture artifact refs and the evidence-matrix manifest ref that the named
#   unit test computes (the test asserts its own row);
# - native producer rows (`molten evidence qualify --scenario <id>`): the canonical ref of the fixed scenario
#   plan and of the subject-independent runtime artifact (harness report or product observation) an actual
#   run produces. `capability_qualification::tests::evidence_matrix_rows_bind_every_scenario_plan` checks every
#   row's plan ref, and each scenario's own test (`capability_qualification::matrix::assert_scenario_bound`)
#   re-executes it once and requires its exact runtime-artifact ref. The subject-bound execution receipt itself is
#   produced only by the independently admitted signed binary (see scripts/qualify-capability-report.py).
# Sequential/repeated placeholder digests and one command aliased across rows are rejected by this contract,
# by `hardening_core::build_evidence_matrix_manifest` and by `tools/capability-report.rs`.
#
# Tracked evidence debt (requirements with no row here; see docs/proof-workflow.md#evidence-matrix-tracked-debt):
# - molten.system_extension.native_effect_{installation,authority,completion,lifetime,consumer}: their only
#   executable evidence is the ignored `tests/collaborationnative` rail, which needs an independently signed
#   cohort (MOLTEN_TEST_SOURCE_TRUST) and emits no canonical artifact with a reproducible ref; one ignored test
#   was previously aliased across ten rows with invented refs. Not claimed until a signed rail retains receipts.
# - molten.testing.cli_receipt_first.negative_fail_closed: the denial test emits no receipt by design (absence is
#   the property), so there is no canonical artifact to bind. Not claimed by this matrix.
# - VM and cross-host families of config/capability-report/capabilities.json (`*-nixos-vm`, `*-cross-host`,
#   `cross-host-transport`) and `post-effect-uncertainty-same-process` have no native producer: unqualified.

let Prelude = import "../docs/nickel-contract-prelude.ncl" in
let CoverageKinds = ["positive", "negative"] in
let EvidenceScopes = ["unit", "property", "cli", "integration", "vm", "dogfood", "exemption"] in
let CoverageKind = Prelude.contract (Prelude.is_one_of CoverageKinds) in
let EvidenceScope = Prelude.contract (Prelude.is_one_of EvidenceScopes) in
# A digest with 128 leading zero bits or one repeated nibble names no real bytes.
let is_real_ref = fun value =>
  Prelude.is_blake3_ref value
  && !(std.string.is_match "^blake3:0{32}" value)
  && std.string.replace (std.string.substring 7 8 value) "" (std.string.substring 7 71 value) != "" in
let RealRef = Prelude.contract is_real_ref in
let EntryContract = {
  requirement_id | Prelude.contracts.StableId,
  coverage_kind | CoverageKind,
  evidence_scope | EvidenceScope,
  target | Prelude.contracts.NonEmptyString,
  command | Prelude.contracts.NonEmptyString,
  artifact_refs | Array RealRef,
  receipt_ref | RealRef,
  caveats | Array Prelude.contracts.NonEmptyString | default = [],
} in
let ExemptionContract = {
  requirement_id | Prelude.contracts.StableId,
  reason_class | Prelude.contracts.StableId,
  evidence_ref | RealRef,
  scope | EvidenceScope,
  review_note | Prelude.contracts.NonEmptyString,
} in
let MatrixContract = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.testing.evidence-matrix.v1"),
  entries
    | std.contract.Sequence [
      Prelude.contracts.NonEmptyArray,
      # One executed command is evidence for exactly one row.
      Prelude.contract (Prelude.distinct_by (fun entry => entry.command)),
      Array EntryContract
    ],
  exemptions | Array ExemptionContract | default = [],
} in
{
  schema = "molten.testing.evidence-matrix.v1",
  entries = [
    {
      requirement_id = "molten.testing.evidence_matrix.checked_in_manifest",
      coverage_kind = "positive",
      evidence_scope = "unit",
      target = "src/testing/parts/hardening/tests/m000/p000/body.rs",
      command = "cargo test hardening_core::tests::evidence_matrix_manifest_accepts_positive_and_negative_entries --lib",
      artifact_refs = ["blake3:0145b7f47e548444909104bf004965d407ed08a0f0001c2b6adbc1d00d7a89f3", "blake3:d1384153c3dd263941542916fe641e3fdfc9bd3bad0e565852843cb871d958a8"],
      receipt_ref = "blake3:e6e5338f5460650ae1f4ce0272159b1ed2f3171e7a49327f5808a704ee9219e0",
      caveats = ["unit manifest fixture; the test asserts this row's refs equal the manifest it computes"],
    },
    {
      requirement_id = "molten.testing.evidence_matrix.changed_requirement_gate",
      coverage_kind = "negative",
      evidence_scope = "unit",
      target = "src/testing/parts/hardening/tests/m000/p000/body.rs",
      command = "cargo test hardening_core::tests::evidence_matrix_manifest_denies_missing_negative_and_stale_id --lib",
      artifact_refs = ["blake3:0145b7f47e548444909104bf004965d407ed08a0f0001c2b6adbc1d00d7a89f3", "blake3:f96585c28da101779d09119c71189f27e8a28e1f07d28c15a3ae845265d40573"],
      receipt_ref = "blake3:c5775a4010c948baaad86e11958a78a9e75e637d9e619c347ba6bfdb248a43c4",
      caveats = ["denied manifest fixture; the test asserts this row's refs equal the manifest it computes"],
    },
    {
      requirement_id = "molten.testing.cli_receipt_first.normative_artifacts",
      coverage_kind = "positive",
      evidence_scope = "cli",
      target = "tests/parts/cliharness/p025/body.rs",
      command = "cargo test --test cliharness cli_ci_run_receipt_binds_nextest_metadata_and_junit_view",
      artifact_refs = ["blake3:28562fa2922cc78fd5f95d0261112fa7fd5ab940ed717bda74c323a8f1226f4d", "blake3:33bbf15f27aada1ea6288c694beba477dbc0364029a163830d3941fb23c3d11b", "blake3:455c02bf2a8416a03d71684659d2e8ffc92b39db095f5ebc27c6d85ff105ee60", "blake3:e6146ed04706ade27bbafe02203837d9be4dfc919b5e1657adebd16c466c7faa"],
      receipt_ref = "blake3:8e8e55a98303b48189ed61a059cfe385b3d0c380170d829be431d251c157b78d",
      caveats = ["CLI-written ci-test-run receipt over fixed inputs; the test asserts this row's refs equal its input content refs and the canonical receipt; stdout is a diagnostic view"],
    },
    {
      requirement_id = "molten.testing.capability_context.request_binding",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/mod.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario granted-send --out $RUN/granted-send.preserves",
      artifact_refs = ["blake3:08f762295c0698bbc8441d192a131abed4abf98dc83698b5ee42d3dd48793638"],
      receipt_ref = "blake3:0a046c7b9b473f34dfe36107d084d0c214c9c81d9ec5a9f8eb3c3872bb3e7050",
      caveats = ["native harness simulation of the granted send fixture; not durability, live transport or distributed evidence"],
    },
    {
      requirement_id = "molten.testing.capability_context.request_binding",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/mod.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario denied-send --out $RUN/denied-send.preserves",
      artifact_refs = ["blake3:6ed85a99928dabbc902bb6c03315a45e143386d36559a7199be5608ea8764a08"],
      receipt_ref = "blake3:fa92e18df2e988baa9f37cdd015be4e36cbbc8c0e579ab7c6b5e48c0aaa3a4c2",
      caveats = ["native harness simulation preserves the missing-grant denial with no effect; not a live failover negative"],
    },
    {
      requirement_id = "molten.fabric_durability.snapshot_recovery",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/durable.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario durable-restart-recovery --out $RUN/durable-restart-recovery.preserves",
      artifact_refs = ["blake3:30bb19d8ac78aee1b4b6bf4dc294cd3fccaa84b309be1c5e2f6ae48137c33205"],
      receipt_ref = "blake3:375709fd42d666537f8ba34894537cf7c4ed530fbc5e5ae99506190bff03710b",
      caveats = ["same-process Redb close/reopen restart, admitted recovery and content-verified checkpoint restore; not machine-loss, VM or cross-host evidence"],
    },
    {
      requirement_id = "molten.fabric_durability.snapshot_recovery",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/durable.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario durable-corrupt-snapshot-denied --out $RUN/durable-corrupt-snapshot-denied.preserves",
      artifact_refs = ["blake3:afc5678b74a1df1dd86a39a98ea234aeba886f2f5b11dc061d1a83397c783602"],
      receipt_ref = "blake3:f912fc953c3f11ec444b7912b92bc4d228e67b69eab82fc65474645198b18521",
      caveats = ["mismatched snapshot identity denied, corrupted payload quarantined on reopen, stale generation not admitted; same-process only"],
    },
    {
      requirement_id = "molten.fabric_execution.uncertainty",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/effect.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario execution-unknown-after-start --out $RUN/execution-unknown-after-start.preserves",
      artifact_refs = ["blake3:e3e21f922250ba236d43b081f0bb1193a5531df1eed1347345a776a6d3fc3a1e"],
      receipt_ref = "blake3:cb1397849cef15f8374b487d0132c7534baf9bb34e8c3e76ec2898941080fc81",
      caveats = ["a started /bin/sh child that never consumes its input stays unknown and requires reconciliation; one host only"],
    },
    {
      requirement_id = "molten.fabric_execution.uncertainty",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/effect.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario execution-prestart-definite --out $RUN/execution-prestart-definite.preserves",
      artifact_refs = ["blake3:faa27c82a3387133b9483fe333f92ebc75c55f6a894356bf91147773df31e1fa"],
      receipt_ref = "blake3:c5302ec2ae5449ea820c7fc3341fc3267a40842908e62c856c11ac908fd3a408",
      caveats = ["crossed resolution and spawn failure are definite pre-start outcomes, never unknown or published; one host only"],
    },
    {
      requirement_id = "molten.system_extension.native_effect_uncertainty",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/effect.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario native-work-unknown-after-effect --out $RUN/native-work-unknown-after-effect.preserves",
      artifact_refs = ["blake3:1d9c8c2babe4ea0c9b75dbb86ec1396987b8b2f030ebeccc00c7b45d666e5867"],
      receipt_ref = "blake3:f3dac0a339dfbf3ae6789a3dd09488bc743c01a2120cddc4c4dff61ce0248788",
      caveats = ["PARTIAL: service-host durable work stays Unknown across host reopen after a started /bin/sh child effect; the work runtime is the producer-owned EffectRuntime port, not the native supervisor, and the Raft-committed native effect path is not exercised"],
    },
    {
      requirement_id = "molten.system_extension.native_effect_uncertainty",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/effect.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario native-uncertain-work-not-readmitted --out $RUN/native-uncertain-work-not-readmitted.preserves",
      artifact_refs = ["blake3:6ed797c37016e99ebcc6ddedaddb906ae357e5cfca12f4fdf95089ddadbce912"],
      receipt_ref = "blake3:bef9636add086a8f8ae84384eec36079a89bd41c9e3c04072766d27a88c8ba96",
      caveats = ["PARTIAL: uncertain durable work is never readmitted or redelivered, before or after host reopen, and the started child effect is not repeated; producer-owned EffectRuntime port, not the native supervisor"],
    },
    {
      requirement_id = "molten.fabric_transport.cross_process_endpoint",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/transport.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario transport-endpoint-import --out $RUN/transport-endpoint-import.preserves",
      artifact_refs = ["blake3:ba3586a816484170000d2899f1f4e41800d7410bb576016e5c667ad673bb2ddf"],
      receipt_ref = "blake3:58713195ff20c554223b48421a5a23f057342bf16a41699f65afc46f505ef3db",
      caveats = ["distinct listener process hands off a canonical descriptor imported against an independently derived identity; loopback only"],
    },
    {
      requirement_id = "molten.fabric_transport.cross_process_endpoint",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/transport.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario transport-endpoint-tamper-denied --out $RUN/transport-endpoint-tamper-denied.preserves",
      artifact_refs = ["blake3:2dcbbe557b50e7565418f3ecaf0e06d5070b2c672d95b49114ad55fc81919299"],
      receipt_ref = "blake3:570a6a42ebfa195073e1488df7d352aa2f1c4fd9fc3ce5931de6e96e3f0bdfe7",
      caveats = ["tampered descriptor ref, unexpected identity and stale generation deny before dial; loopback only"],
    },
    {
      requirement_id = "molten.fabric_transport.cross_process_session",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/transport.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario transport-session-acknowledged --out $RUN/transport-session-acknowledged.preserves",
      artifact_refs = ["blake3:010aaeb06aa496591b88e74f01bd33146f438be19bc509371e9eec961a270bca"],
      receipt_ref = "blake3:bfbe229d65aa4261c71f8b10d79aff5f907463c1ee4a2848f7186a887d7d5591",
      caveats = ["one bounded frame acknowledged across distinct processes with no automatic retry; loopback only"],
    },
    {
      requirement_id = "molten.fabric_transport.cross_process_session",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/transport.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario transport-session-lost-acknowledgement --out $RUN/transport-session-lost-acknowledgement.preserves",
      artifact_refs = ["blake3:84d8d80dadf3126142dc0633bca8130ab0ce39967c29c5da726b7a0523a299b4"],
      receipt_ref = "blake3:c1d30ebfe80ecb6b7c8f9181c53ab610b735e2f1860a21e01958bd33c501f106",
      caveats = ["withheld acknowledgement leaves the submission undelivered and the client does not retry; loopback only"],
    },
    {
      requirement_id = "molten.world_replay.execution_boundary",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/world.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario world-replay-admitted --out $RUN/world-replay-admitted.preserves",
      artifact_refs = ["blake3:334efc0271dd2b79e5c4dfe8ffede8d87d9f0d733d37f7a8012e85a05f19d119"],
      receipt_ref = "blake3:7fd418a9c53b238b322db0b2f3c01f103cdb038c419fb496648194461a6dfa3a",
      caveats = ["run_world_replay over three captured committed turns with current signed-token authority; producer-owned fixture keys; same-process only, not VM evidence"],
    },
    {
      requirement_id = "molten.world_replay.execution_boundary",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/world.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario world-replay-authority-denied --out $RUN/world-replay-authority-denied.preserves",
      artifact_refs = ["blake3:2c8eec003858979f1b7e1bda3d1e6f4c4b478616790db7a44e1df638177bacb5"],
      receipt_ref = "blake3:a680b6f48bd5ca7c118b101ac5e477b83a7b0d4130b24972e39ed575d4c5624f",
      caveats = ["revoked issuer denial receipt is published with zero transitions and captures; same-process only"],
    },
    {
      requirement_id = "molten.world_snapshot.restore",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/world.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario world-restore-current-admission --out $RUN/world-restore-current-admission.preserves",
      artifact_refs = ["blake3:308726645a97e925305e9c0f21d9846e1b47990829b6697087a14d0974f0ceda"],
      receipt_ref = "blake3:e534b38e592d6a006b5378eb85703f694813f48d88b8cc24400a6bb3f02b2a5d",
      caveats = ["runtime-spine logical restore rechecks current admission, recreates handles and never dispatches unresolved requests; producer-owned fixture keys; same-process only"],
    },
    {
      requirement_id = "molten.world_snapshot.restore",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/world.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario world-restore-stale-admission-denied --out $RUN/world-restore-stale-admission-denied.preserves",
      artifact_refs = ["blake3:f73f35f4aee6e30aa802bf47f26b31e3217d66bb28e8026df6263439bc56746a"],
      receipt_ref = "blake3:0efa2ac0abe56deddcdd696a76e55e5b7177c114a17f96a6f4d94aa7add7ba06",
      caveats = ["revoked, stale-generation, crossed-descriptor and untrusted-signer admissions deny before activation; same-process only"],
    },
    {
      requirement_id = "molten.world_branch_authority.activation",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/world.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario world-activation-current-facts --out $RUN/world-activation-current-facts.preserves",
      artifact_refs = ["blake3:3e82c708c87f28a8714581840a2b34aead6c99854e685761bcde5947c0c453b9"],
      receipt_ref = "blake3:79fa76e458ebc1150bfda4eb4bce571f4aaea8bdf03f9307e3c0d1d26a3dede2",
      caveats = ["copyable-path branch activation rechecks current facts once; promotion/linear-transfer ports refuse rather than act; same-process only"],
    },
    {
      requirement_id = "molten.world_branch_authority.activation",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/world.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario world-activation-revoked-denied --out $RUN/world-activation-revoked-denied.preserves",
      artifact_refs = ["blake3:6b8ec3f1b69a157fe5b7193b637471da09be77cfacd4f0ae9387b38774454d36"],
      receipt_ref = "blake3:34b801ec7b17559b0081873eb31ce22d8aa452941f857b77d7d818a39e246e1f",
      caveats = ["a revocation effective after planning is caught at the activation recheck; nothing activated and the denial receipt is published; same-process only"],
    },
    {
      requirement_id = "molten.fabric_consistency.live_raft",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/control_plane.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario raft-failover-commit-survives --out $RUN/raft-failover-commit-survives.preserves",
      artifact_refs = ["blake3:7498527d5ed9f936891980e9da67641f2c8fc61f88baf32d53e27675ec0e750b"],
      receipt_ref = "blake3:75a29d3033e86146965ff76d163869341bed4dbb2468176d65b3d5d28e9734e7",
      caveats = ["three replica processes on one host over Iroh with Redb logs; committed entry survives SIGKILL of every process and durable restart; campaigns are signal-driven, not timer-driven elections; not VM or cross-host evidence"],
    },
    {
      requirement_id = "molten.fabric_consistency.live_raft",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/control_plane.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario raft-stale-leader-fenced --out $RUN/raft-stale-leader-fenced.preserves",
      artifact_refs = ["blake3:10ce4d6090ac413df8cd0f0bad49aac248d11c2b9521085542a7400abbff3fa1"],
      receipt_ref = "blake3:7468156f526ce7378c960c13afa8987f8e0a8c9b29488b66d3015b63f3ab8a0f",
      caveats = ["after recovery the crashed leader's stale-term append/vote, stale generation and stale fencing epoch are refused without changing the new leader; distinct processes on one host"],
    },
    {
      requirement_id = "molten.fabric_consistency.final_validation",
      coverage_kind = "positive",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/control_plane.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario raft-final-quorum-evidence --out $RUN/raft-final-quorum-evidence.preserves",
      artifact_refs = ["blake3:e7d1f17a492a5ca0a8ff722fa9b6895c70aed1fcc3b772824ee5deefd7c2d7c8"],
      receipt_ref = "blake3:29bd766781e80c40bfe3da2c4e74361647700ead178936061fd8b33c5af72554",
      caveats = ["the observed commit quorum validates through the production validate_replica_quorum; distinct processes on one host"],
    },
    {
      requirement_id = "molten.fabric_consistency.final_validation",
      coverage_kind = "negative",
      evidence_scope = "integration",
      target = "src/capability_qualification/products/control_plane.rs",
      command = "$SIGNED_OUTPUT/bin/molten evidence qualify --source-root $SOURCE_ROOT --scenario raft-false-quorum-denied --out $RUN/raft-false-quorum-denied.preserves",
      artifact_refs = ["blake3:1609244779b500d0f7fd046db1291ba4797d922c4374bf0718207036ffd71dec"],
      receipt_ref = "blake3:d01cad0edfb9ece6545333b7d05aeedb2e80eb63fabde6c473457d84bdb2c293",
      caveats = ["duplicate, outsider and minority quorum evidence built from the observed run is denied by the production validator; distinct processes on one host"],
    },
  ],
  exemptions = [],
} | MatrixContract
