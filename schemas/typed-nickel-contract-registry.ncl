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
        "cargo test -p aspen-ci config::schema --features nickel",
      ],
      freshness_gate = "The embedded schema test must keep include_str!(schema/ci_schema.ncl) present and Nickel typecheck must pass for schema edits.",
      status = "existing",
      notes = "Human-authored CI pipeline config uses Nickel defaults/contracts before Rust deserializes PipelineConfig.",
    },
    {
      id = "deploy-protocol-dtos",
      source_kind = "rust-derived",
      owner = "aspen-ci deploy orchestrator maintainers",
      source_path = "crates/aspen-ci/src/orchestrator/deploy_executor.rs",
      artifact_path = "schemas/deploy-protocol.ncl",
      generated = true,
      generation_command = "cargo run -p aspen-ci --bin generate-deploy-schema > schemas/deploy-protocol.ncl",
      validation_commands = [
        "nix run nixpkgs#nickel -- typecheck schemas/deploy-protocol.ncl",
        "cargo test -p aspen-ci schema_gen",
      ],
      freshness_gate = "Generated schemas/deploy-protocol.ncl must match JsonSchema metadata for DeployRequest, DeployInitResult, DeployNodeStatus, and DeployStatusResult.",
      status = "needs-hardening",
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
        "cargo run --bin aspen-generate-schema -- --output-dir schemas",
      ],
      freshness_gate = "NodeConfig Rust schema generation and Nickel-authored profile contract reviews must agree on public operator fields.",
      status = "needs-hardening",
      notes = "Node/profile overlays are human-authored; Nickel should own config contracts while Rust owns runtime behavior.",
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
        "scripts/test-harness.sh check",
      ],
      freshness_gate = "scripts/test-harness.sh check rejects stale test-harness/generated/inventory.json after suite manifest edits.",
      status = "existing",
      notes = "Suite manifests, including patchbay suites, are Nickel-authored and export normalized inventory consumed by scripts/Nix.",
    },
    {
      id = "patchbay-fault-scenarios",
      source_kind = "nickel-authored",
      owner = "Aspen patchbay test maintainers",
      source_path = "test-harness/suites/patchbay/patchbay-fault.ncl",
      generated = false,
      validation_commands = [
        "scripts/test-harness.sh list --layer patchbay --format commands",
        "scripts/test-harness.sh check",
      ],
      freshness_gate = "Patchbay suite/scenario manifests must be represented in fresh test-harness generated inventory before execution.",
      status = "needs-hardening",
      notes = "Current patchbay coverage is suite-manifest based; future fault-dimension contracts should remain Nickel-authored and bounded.",
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
        "cargo run --script scripts/check-crate-extraction-readiness.rs",
      ],
      freshness_gate = "The readiness checker must reject policy/schema drift and stale/missing evidence for candidate crates.",
      status = "existing",
      notes = "Maintainer-authored dependency/readiness rails already live in Nickel and should be deepened rather than duplicated.",
    },
    {
      id = "feature-bundle-policy",
      source_kind = "nickel-authored",
      owner = "Aspen release/profile maintainers",
      source_path = "Cargo.toml",
      artifact_path = "schemas/feature-bundles.ncl",
      generated = false,
      generation_command = "planned: encode profile bundles in Nickel from release policy, not from Cargo feature resolution internals",
      validation_commands = [
        "planned: Nickel validate minimal/dogfood/ci-worker/forge/snix profile fixtures",
      ],
      freshness_gate = "Feature-bundle policy changes must validate supported profile fixtures and reject contradictory bundles.",
      status = "planned",
      notes = "Nickel may validate supported human-facing product profiles; Rust/Cargo retain actual compilation feature semantics.",
    },
    {
      id = "snix-build-executor-policy",
      source_kind = "nickel-authored",
      owner = "Aspen snix/Nix executor maintainers",
      source_path = "crates/aspen-cluster/src/config/snix.rs",
      artifact_path = "schemas/snix-build-executor-policy.ncl",
      generated = false,
      generation_command = "planned: author bounded executor/cache/fallback policy contract in Nickel",
      validation_commands = [
        "planned: Nickel validate snix executor positive and negative policy fixtures",
      ],
      freshness_gate = "Snix executor policy fixtures must reject unsupported sandbox/cache/fallback combinations before build execution.",
      status = "planned",
      notes = "Nickel validates operator policy references and bounds only; snix build behavior remains Rust-owned.",
    },
    {
      id = "trust-bootstrap-policy",
      source_kind = "nickel-authored",
      owner = "Aspen trust/bootstrap maintainers",
      source_path = "docs/trust-quorum.md",
      artifact_path = "schemas/trust-bootstrap-policy.ncl",
      generated = false,
      generation_command = "planned: author secret-free trust/quorum/bootstrap policy contract in Nickel",
      validation_commands = [
        "planned: Nickel validate threshold/participant/rotation policy fixtures without secret values",
      ],
      freshness_gate = "Trust/bootstrap policy fixtures must reject impossible thresholds and raw secret material.",
      status = "planned",
      notes = "Nickel can validate quorum references and bounds but not cryptographic operations or secret material.",
    },
    {
      id = "operator-diagnostics-evidence",
      source_kind = "rust-derived",
      owner = "Aspen operator evidence maintainers",
      source_path = "docs/operator-receipts.md",
      artifact_path = "schemas/operator-diagnostics-evidence.ncl",
      generated = true,
      generation_command = "planned: generate evidence contracts from Rust diagnostic/receipt DTOs as they are promoted",
      validation_commands = [
        "planned: Rust round-trip tests plus Nickel validation for diagnostic evidence fixtures",
      ],
      freshness_gate = "Promoted operator diagnostic DTOs must generate fresh Nickel contracts and validate serialized evidence fixtures.",
      status = "planned",
      notes = "Crunch's diagnostics/report patterns are adaptable, but Aspen evidence DTO serialization remains Rust-owned.",
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
