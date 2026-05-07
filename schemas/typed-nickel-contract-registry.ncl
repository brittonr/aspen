# Typed Nickel contract registry for Aspen.
#
# This registry is the source-of-truth map for which Aspen configuration,
# evidence, and policy families are Nickel-authored versus Rust-derived.
# It intentionally records planned contract families before their artifacts
# exist so implementation work cannot proceed without an explicit owner and
# freshness gate.

let contains = fun values value =>
  std.array.fold_left (fun found item => found || item == value) false values in

let non_empty_string = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_string value && std.string.length value > 0 then
      'Ok
    else
      'Error { message = "%{field} must be a non-empty string" }
  ) in

let enum_string = fun field allowed =>
  std.contract.from_validator (fun value =>
    if std.is_string value && contains allowed value then
      'Ok
    else
      'Error { message = "%{field} must be one of %{std.serialize 'Json allowed}" }
  ) in

let SourceKind = enum_string "source_kind" [
  "rust-derived",
  "nickel-authored",
  "non-candidate",
] in

let FamilyStatus = enum_string "status" [
  "existing",
  "planned",
  "needs-hardening",
] in

let CrunchClassification = enum_string "classification" [
  "vendor",
  "adapt",
  "reject",
] in

let ContractFamily = {
  id | non_empty_string "family.id",
  source_kind | SourceKind,
  owner | non_empty_string "family.owner",
  source_path | non_empty_string "family.source_path",
  artifact_path | String | optional,
  generated | Bool,
  generation_command | String | optional,
  validation_commands | Array String | default = [],
  freshness_gate | non_empty_string "family.freshness_gate",
  status | FamilyStatus,
  notes | non_empty_string "family.notes",
} in

let CrunchCandidate = {
  id | non_empty_string "crunch.id",
  source_path | non_empty_string "crunch.source_path",
  classification | CrunchClassification,
  applies_to | Array String | default = [],
  rationale | non_empty_string "crunch.rationale",
} in

let NonCandidate = {
  id | non_empty_string "non_candidate.id",
  owner | non_empty_string "non_candidate.owner",
  rationale | non_empty_string "non_candidate.rationale",
} in

let ContractRegistry = {
  schema_version | String,
  families | Array ContractFamily,
  crunch_prior_art | Array CrunchCandidate,
  non_candidates | Array NonCandidate,
} in

{
  schema_version = "1",

  families = [
    {
      id = "ci-pipeline-config",
      source_kind = "nickel-authored",
      owner = "aspen-ci config maintainers",
      source_path = "crates/aspen-ci/src/config/schema/ci_schema.ncl",
      artifact_path = "crates/aspen-ci/src/config/schema/ci_schema.ncl",
      generated = false,
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck crates/aspen-ci/src/config/schema/ci_schema.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
        "cargo test -p aspen-ci config::schema --features nickel",
      ],
      freshness_gate = "The embedded schema test must keep include_str!(schema/ci_schema.ncl) present and Nickel typecheck/export fixtures must pass for CI config edits.",
      status = "existing",
      notes = "Human-authored CI pipeline config now includes typed cache/artifact/deploy policy contracts before Rust deserializes PipelineConfig; validation remains shape-only.",
    },
    {
      id = "deploy-protocol-dtos",
      source_kind = "rust-derived",
      owner = "aspen-ci deploy orchestrator maintainers",
      source_path = "crates/aspen-ci/src/orchestrator/deploy_executor.rs",
      artifact_path = "schemas/deploy-protocol.ncl",
      generated = true,
      generation_command = "UPDATE_SNAPSHOTS=1 cargo test -p aspen-ci test_deploy_protocol_schema_snapshot",
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck schemas/deploy-protocol.ncl",
        "cargo test -p aspen-ci test_deploy_protocol_schema_snapshot",
      ],
      freshness_gate = "cargo test -p aspen-ci test_deploy_protocol_schema_snapshot rejects stale schemas/deploy-protocol.ncl after DeployRequest, DeployInitResult, DeployNodeStatus, or DeployStatusResult schema changes.",
      status = "existing",
      notes = "Rust owns the serialized deploy request/status DTOs; Nickel validates generated operator/protocol shape only.",
    },
    {
      id = "dogfood-run-receipt",
      source_kind = "rust-derived",
      owner = "aspen-dogfood receipt maintainers",
      source_path = "crates/aspen-dogfood/src/receipt.rs",
      artifact_path = "schemas/dogfood-run-receipt.ncl",
      generated = true,
      generation_command = "python3 scripts/generate-typed-nickel-contracts.py --write",
      validation_commands = [
        "python3 scripts/generate-typed-nickel-contracts.py --check",
        "nix run nixpkgs#nickel -- typecheck schemas/dogfood-run-receipt.ncl",
        "cargo test -p aspen-dogfood receipt",
      ],
      freshness_gate = "scripts/generate-typed-nickel-contracts.py --check rejects stale schemas/dogfood-run-receipt.ncl after DogfoodRunReceipt schema changes.",
      status = "existing",
      notes = "Operator evidence is serialized by Rust; the generated Nickel contract validates the saved/published receipt shape without owning runtime validation.",
    },
    {
      id = "native-ci-run-receipt",
      source_kind = "rust-derived",
      owner = "aspen-client-api CI maintainers",
      source_path = "crates/aspen-client-api/src/messages/ci.rs",
      artifact_path = "schemas/ci-run-receipt.ncl",
      generated = true,
      generation_command = "python3 scripts/generate-typed-nickel-contracts.py --write",
      validation_commands = [
        "python3 scripts/generate-typed-nickel-contracts.py --check",
        "nix run nixpkgs#nickel -- typecheck schemas/ci-run-receipt.ncl",
        "cargo test -p aspen-client-api ci_receipt_schema_and_status_labels_are_documented --features ci",
      ],
      freshness_gate = "scripts/generate-typed-nickel-contracts.py --check rejects stale schemas/ci-run-receipt.ncl after CiRunReceipt schema changes.",
      status = "existing",
      notes = "Native CI receipts are Rust-owned operator evidence with schema aspen.ci.run-receipt.v1; the Nickel contract is generated from the serialized DTO shape.",
    },
    {
      id = "node-cluster-profile-config",
      source_kind = "nickel-authored",
      owner = "aspen-cluster config maintainers",
      source_path = "crates/aspen-nickel/src/schema/node_config.ncl",
      artifact_path = "crates/aspen-nickel/src/schema/node_config.ncl",
      generated = false,
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck crates/aspen-nickel/src/schema/node_config.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
        "cargo run --bin aspen-generate-schema -- --output-dir schemas",
      ],
      freshness_gate = "NodeConfig Rust schema generation and Nickel-authored profile contract reviews must agree on public operator fields; Nickel fixtures validate bootstrap/profile/metrics/trust refs.",
      status = "existing",
      notes = "Node/profile overlays are human-authored; Nickel now owns typed bootstrap peers, feature bundle refs, metrics/OTLP, and trust reference contracts while Rust owns runtime behavior.",
    },
    {
      id = "test-harness-suite-manifests",
      source_kind = "nickel-authored",
      owner = "Aspen test harness maintainers",
      source_path = "test-harness/schema.ncl",
      artifact_path = "test-harness/generated/inventory.json",
      generated = false,
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck test-harness/schema.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
        "scripts/test-harness.sh check",
      ],
      freshness_gate = "scripts/test-harness.sh check rejects stale test-harness/generated/inventory.json after suite manifest edits; Nickel fixtures validate capability, timeout, artifact, fault, and convergence fields.",
      status = "existing",
      notes = "Suite manifests, including patchbay suites, are Nickel-authored and export normalized inventory consumed by scripts/Nix with typed capability/isolation/fault metadata.",
    },
    {
      id = "patchbay-fault-scenarios",
      source_kind = "nickel-authored",
      owner = "Aspen patchbay test maintainers",
      source_path = "test-harness/suites/patchbay/patchbay-fault.ncl",
      generated = false,
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck test-harness/schema.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
        "scripts/test-harness.sh list --layer patchbay --format commands",
        "scripts/test-harness.sh check",
      ],
      freshness_gate = "Patchbay suite/scenario manifests must be represented in fresh test-harness generated inventory before execution and fault dimensions must validate against bounded Nickel contracts.",
      status = "existing",
      notes = "Patchbay coverage remains suite-manifest based, now with typed fault-dimension/convergence contracts in the shared test harness schema.",
    },
    {
      id = "crate-extraction-readiness-policy",
      source_kind = "nickel-authored",
      owner = "Aspen crate extraction maintainers",
      source_path = "docs/crate-extraction/policy.ncl",
      artifact_path = "docs/crate-extraction/policy.ncl",
      generated = false,
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck docs/crate-extraction/policy.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
        "scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family trust-crypto-secrets --output-json openspec/changes/type-nickel-contract-boundaries/evidence/crate-extraction-readiness.json --output-markdown openspec/changes/type-nickel-contract-boundaries/evidence/crate-extraction-readiness.md",
      ],
      freshness_gate = "The readiness checker must reject policy/schema drift and stale/missing evidence for candidate crates; Nickel typecheck validates readiness states, classes, publication metadata, and evidence requirements.",
      status = "existing",
      notes = "Maintainer-authored dependency/readiness rails now include typed readiness/class enums, publication metadata, and required evidence contracts.",
    },
    {
      id = "feature-bundle-policy",
      source_kind = "nickel-authored",
      owner = "Aspen release/profile maintainers",
      source_path = "Cargo.toml",
      artifact_path = "schemas/feature-bundles.ncl",
      generated = false,
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck schemas/feature-bundles.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
      ],
      freshness_gate = "Feature-bundle policy changes must typecheck supported profile fixtures and reject unknown feature/profile names in Nickel fixture validation.",
      status = "existing",
      notes = "Nickel validates supported human-facing product profiles in schemas/feature-bundles.ncl; Rust/Cargo retain actual compilation feature semantics.",
    },
    {
      id = "snix-build-executor-policy",
      source_kind = "nickel-authored",
      owner = "Aspen snix/Nix executor maintainers",
      source_path = "crates/aspen-cluster/src/config/snix.rs",
      artifact_path = "schemas/snix-build-executor-policy.ncl",
      generated = false,
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck schemas/snix-build-executor-policy.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
      ],
      freshness_gate = "Snix executor policy fixtures reject unsupported sandbox/cache/fallback combinations before build execution.",
      status = "existing",
      notes = "Nickel validates operator policy references and bounds only in schemas/snix-build-executor-policy.ncl; snix build behavior remains Rust-owned.",
    },
    {
      id = "trust-bootstrap-policy",
      source_kind = "nickel-authored",
      owner = "Aspen trust/bootstrap maintainers",
      source_path = "docs/trust-quorum.md",
      artifact_path = "schemas/trust-bootstrap-policy.ncl",
      generated = false,
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck schemas/trust-bootstrap-policy.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
      ],
      freshness_gate = "Trust/bootstrap policy fixtures reject raw secret material while accepting secret references, participants, thresholds, and rotation metadata.",
      status = "existing",
      notes = "Nickel validates quorum references and bounds in schemas/trust-bootstrap-policy.ncl but not cryptographic operations or secret material.",
    },
    {
      id = "sponsored-runtime-policy",
      source_kind = "nickel-authored",
      owner = "Aspen runtime sponsorship maintainers",
      source_path = "schemas/sponsored-runtime-policy.ncl",
      artifact_path = "schemas/sponsored-runtime-policy.ncl",
      generated = false,
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck schemas/sponsored-runtime-policy.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
      ],
      freshness_gate = "Sponsorship policy fixtures must typecheck provider offers, sponsor policies, resource class catalogs, and admission profiles while rejecting raw secrets and mismatched principal roles before Rust runtime side effects.",
      status = "existing",
      notes = "Nickel owns human-authored sponsorship policy/config shape; Rust owns runtime authorization, metering, quota ledgers, usage receipts, and distributed state transitions.",
    },
    {
      id = "sponsored-runtime-grant",
      source_kind = "rust-derived",
      owner = "Aspen runtime sponsorship maintainers",
      source_path = "crates/aspen-runtime-core/src/lib.rs",
      artifact_path = "schemas/sponsored-runtime-grant.ncl",
      generated = true,
      generation_command = "python3 scripts/generate-typed-nickel-contracts.py --write",
      validation_commands = [
        "python3 scripts/generate-typed-nickel-contracts.py --check",
        "nix run nixpkgs#nickel -- typecheck schemas/sponsored-runtime-grant.ncl",
      ],
      freshness_gate = "scripts/generate-typed-nickel-contracts.py --check rejects stale schemas/sponsored-runtime-grant.ncl after SponsoredRuntimeGrant schema changes.",
      status = "existing",
      notes = "Rust owns the serialized sponsored grant DTO; Nickel validates generated operator/evidence shape only.",
    },
    {
      id = "sponsored-quota-ledger",
      source_kind = "rust-derived",
      owner = "Aspen runtime sponsorship maintainers",
      source_path = "crates/aspen-runtime-core/src/lib.rs",
      artifact_path = "schemas/sponsored-quota-ledger.ncl",
      generated = true,
      generation_command = "python3 scripts/generate-typed-nickel-contracts.py --write",
      validation_commands = [
        "python3 scripts/generate-typed-nickel-contracts.py --check",
        "nix run nixpkgs#nickel -- typecheck schemas/sponsored-quota-ledger.ncl",
      ],
      freshness_gate = "scripts/generate-typed-nickel-contracts.py --check rejects stale schemas/sponsored-quota-ledger.ncl after SponsoredQuotaLedger schema changes.",
      status = "existing",
      notes = "Rust owns quota arithmetic and ledger DTO serialization; Nickel validates generated evidence shape only.",
    },
    {
      id = "sponsored-usage-receipt",
      source_kind = "rust-derived",
      owner = "Aspen runtime sponsorship maintainers",
      source_path = "crates/aspen-runtime-core/src/lib.rs",
      artifact_path = "schemas/sponsored-usage-receipt.ncl",
      generated = true,
      generation_command = "python3 scripts/generate-typed-nickel-contracts.py --write",
      validation_commands = [
        "python3 scripts/generate-typed-nickel-contracts.py --check",
        "nix run nixpkgs#nickel -- typecheck schemas/sponsored-usage-receipt.ncl",
      ],
      freshness_gate = "scripts/generate-typed-nickel-contracts.py --check rejects stale schemas/sponsored-usage-receipt.ncl after SponsoredUsageReceipt schema changes.",
      status = "existing",
      notes = "Rust owns metering and usage receipt emission; Nickel validates generated receipt evidence shape only and must not own runtime behavior.",
    },
    {
      id = "operator-diagnostics-evidence",
      source_kind = "rust-derived",
      owner = "Aspen operator evidence maintainers",
      source_path = "docs/operator-receipts.md",
      artifact_path = "schemas/operator-diagnostics-evidence.ncl",
      generated = true,
      generation_command = "planned: generate future promoted diagnostic evidence DTO contracts from Rust schemas; current envelope contract is hand-authored as a boundary fixture",
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck schemas/operator-diagnostics-evidence.ncl",
        "python3 scripts/check-typed-nickel-contract-fixtures.py",
      ],
      freshness_gate = "Promoted operator diagnostic DTOs must generate fresh Nickel contracts and validate serialized evidence fixtures; current envelope fixtures validate redaction-oriented shape.",
      status = "existing",
      notes = "Crunch's diagnostics/report patterns are adaptable; Aspen evidence DTO serialization remains Rust-owned while this contract validates the shared diagnostic envelope boundary.",
    },
  ],

  crunch_prior_art = [
    {
      id = "crunch-contract-helpers",
      source_path = "../crunch/crunch/lib/contracts.ncl",
      classification = "adapt",
      applies_to = ["ci-pipeline-config", "node-cluster-profile-config", "test-harness-suite-manifests", "feature-bundle-policy"],
      rationale = "Reusable enum/string/path/range helper idioms should be adapted to Aspen names and bounds instead of copied with Crunch-specific store semantics.",
    },
    {
      id = "crunch-project-contracts",
      source_path = "../crunch/crunch/lib/project.ncl",
      classification = "adapt",
      applies_to = ["feature-bundle-policy", "node-cluster-profile-config"],
      rationale = "Project/default composition patterns are useful for Aspen profiles, but Crunch project fields and build assumptions are domain-specific.",
    },
    {
      id = "crunch-project-outputs",
      source_path = "../crunch/crunch/lib/project_outputs.ncl",
      classification = "adapt",
      applies_to = ["ci-pipeline-config", "deploy-protocol-dtos"],
      rationale = "Output-selection/default patterns can inform CI artifacts and deploy outputs; Crunch output semantics must not be adopted directly.",
    },
    {
      id = "crunch-derivation-builder",
      source_path = "../crunch/crunch/builders/mk_derivation.ncl",
      classification = "reject",
      applies_to = ["snix-build-executor-policy"],
      rationale = "The builder encodes Crunch derivation/runtime semantics; Aspen may only re-read it for shape/default inspiration under a later Aspen-specific design.",
    },
    {
      id = "crunch-inventory-topology",
      source_path = "../crunch/crunch/lib/inventory.ncl",
      classification = "adapt",
      applies_to = ["test-harness-suite-manifests", "node-cluster-profile-config"],
      rationale = "Inventory/topology shape is reusable for Aspen suites and clusters, but field names and execution semantics are Aspen-specific.",
    },
    {
      id = "crunch-system-module",
      source_path = "../crunch/crunch/lib/system_module.ncl",
      classification = "adapt",
      applies_to = ["node-cluster-profile-config", "feature-bundle-policy"],
      rationale = "Module topology/default conventions map well to Aspen node/profile modules without importing Crunch runtime behavior.",
    },
    {
      id = "crunch-glue-rust-types",
      source_path = "../crunch/crunch/crates/crunch-glue/src/types.rs",
      classification = "adapt",
      applies_to = ["ci-pipeline-config", "node-cluster-profile-config"],
      rationale = "Rust DTO boundary conventions for Nickel-authored exports are useful; Aspen must define its own DTOs and validation errors.",
    },
    {
      id = "crunch-project-manifest-validation",
      source_path = "../crunch/crunch/crates/crunch-project-core/src/manifest.rs",
      classification = "adapt",
      applies_to = ["ci-pipeline-config", "crate-extraction-readiness-policy"],
      rationale = "Version/default/bounds/duplicate-reference checks are reusable validation patterns for Aspen config manifests.",
    },
    {
      id = "crunch-build-report-evidence",
      source_path = "../crunch/crunch/src/build_report.rs",
      classification = "adapt",
      applies_to = ["operator-diagnostics-evidence", "dogfood-run-receipt"],
      rationale = "Report/evidence DTO organization is useful for Aspen operator receipts, but build report semantics stay Crunch-owned.",
    },
    {
      id = "crunch-operator-diagnostics",
      source_path = "../crunch/crunch/src/operator_diagnostics.rs",
      classification = "adapt",
      applies_to = ["operator-diagnostics-evidence"],
      rationale = "Diagnostic report structuring can inform Aspen operator evidence while retaining Aspen-specific redaction and source-of-truth rules.",
    },
    {
      id = "crunch-witness-rebuild",
      source_path = "../crunch/crunch/src/witness_rebuild.rs",
      classification = "reject",
      applies_to = ["operator-diagnostics-evidence"],
      rationale = "Witness/rebuild workflow semantics are Crunch-specific; Aspen may adapt only neutral evidence-shape ideas after separate review.",
    },
    {
      id = "crunch-attestation-schema",
      source_path = "../crunch/crunch/crates/crunch-attestation-core/src/schema.rs",
      classification = "adapt",
      applies_to = ["operator-diagnostics-evidence", "dogfood-run-receipt", "native-ci-run-receipt"],
      rationale = "Attestation schema versioning and evidence DTO patterns are useful, but Aspen must generate contracts from its own Rust receipt types.",
    },
  ],

  non_candidates = [
    {
      id = "raft-behavior",
      owner = "aspen-raft maintainers",
      rationale = "Raft state transitions and consensus behavior require Rust/openraft ownership, deterministic tests, and protocol compatibility guarantees.",
    },
    {
      id = "protocol-discriminant-ownership",
      owner = "aspen-rpc/client-api maintainers",
      rationale = "Wire enum discriminants and compatibility layout are Rust protocol ABI concerns; Nickel may validate exported DTO shape only.",
    },
    {
      id = "cryptographic-internals",
      owner = "aspen-auth/trust maintainers",
      rationale = "Token verification, key derivation, Shamir shares, HMACs, and cryptographic algorithms remain Rust-owned and test/proof-backed.",
    },
    {
      id = "raw-secret-material",
      owner = "Aspen security maintainers",
      rationale = "Nickel may validate secret references, paths, handles, and capability names, but must never embed bearer tokens, private keys, or generated secret material.",
    },
    {
      id = "hot-path-runtime-constants",
      owner = "Aspen runtime maintainers",
      rationale = "Tiger Style limits and hot-path runtime constants must stay in Rust constants/modules where compile-time checks and performance review apply.",
    },
    {
      id = "crunch-runtime-build-semantics",
      owner = "Aspen snix/Nix maintainers",
      rationale = "Crunch derivation, store hashing, witness, and runtime build semantics are not Aspen contract sources unless a future Aspen-specific OpenSpec adopts them.",
    },
  ],
} | ContractRegistry
