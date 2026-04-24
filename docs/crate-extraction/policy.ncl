# Typed dependency policy for Aspen crate-extraction readiness.
# This file is data consumed by scripts/check-crate-extraction-readiness.rs.

let ReadinessState = String in
let CandidateClass = String in
let FeatureSetName = String in
let CrateName = String in
let RepoPath = String in
let Owner = String in
let Reason = String in

let Exception = {
  candidate | CrateName,
  feature_set | FeatureSetName,
  dependency_path | String,
  owner | Owner,
  reason | Reason,
} in

let Candidate = {
  class | CandidateClass,
  readiness_state | ReadinessState,
  manifest | String,
  owner | Owner,
  default_feature_sets | Array FeatureSetName,
  named_reusable_feature_sets | Array FeatureSetName,
  representative_consumers | Array CrateName,
  reexporters | Array CrateName,
  forbidden_unless_feature | Array CrateName,
  exceptions | Array Exception,
} in

{
  schema_version | String = "1",

  readiness_states | Array ReadinessState = [
    "workspace-internal",
    "extraction-ready-in-workspace",
    "blocked-pending-license/publication-policy",
    "publishable from monorepo",
    "future repository split candidate",
  ],

  blocked_until_license_publication_decision | Array ReadinessState = [
    "publishable from monorepo",
    "future repository split candidate",
  ],

  candidate_classes | Array CandidateClass = [
    "leaf type/helper",
    "protocol/wire",
    "storage/backend",
    "runtime adapter",
    "service library",
    "binary shell",
  ],

  required_exception_fields | Array String = [
    "candidate",
    "feature_set",
    "dependency_path",
    "owner",
    "reason",
  ],

  forbidden_by_default | Array CrateName = [
    "aspen",
    "aspen-cli",
    "aspen-tui",
    "aspen-dogfood",
    "aspen-forge-web",
    "aspen-nix-cache-gateway",
    "aspen-snix-bridge",
    "aspen-rpc-handlers",
    "aspen-core-essentials-handler",
    "aspen-blob-handler",
    "aspen-ci-handler",
    "aspen-cluster-handler",
    "aspen-docs-handler",
    "aspen-forge-handler",
    "aspen-job-handler",
    "aspen-nix-handler",
    "aspen-secrets-handler",
  ],

  feature_gated_by_default | Array CrateName = [
    "aspen-trust",
    "aspen-secrets",
    "aspen-sql",
    "aspen-coordination",
    "aspen-sharding",
    "aspen-proxy",
    "aspen-cluster",
    "aspen-core-shell",
    "aspen-client-api",
    "aspen-auth",
    "aspen-transport",
    "aspen-raft-network",
  ],

  concrete_transport_crates | Array CrateName = [
    "iroh",
    "iroh-base",
    "irpc",
    "aspen-transport",
    "aspen-raft-network",
  ],

  allowed_reusable_dependencies | Array CrateName = [
    "aspen-constants",
    "aspen-hlc",
    "aspen-kv-types",
    "aspen-storage-types",
    "aspen-cluster-types",
    "aspen-traits",
    "aspen-time",
    "aspen-auth-core",
    "aspen-hooks-ticket",
    "aspen-hooks-types",
    "aspen-coordination-protocol",
    "aspen-forge-protocol",
    "aspen-jobs-protocol",
    "aspen-raft-kv-types",
    "aspen-redb-storage",
    "aspen-raft-kv",
    "openraft",
    "redb",
    "serde",
    "postcard",
    "blake3",
    "hex",
    "tracing",
    "metrics",
  ],

  evidence_roots | Array RepoPath = [
    "openspec/changes/prepare-crate-extraction/evidence",
  ],

  candidates | { _: Candidate } = {
    aspen_kv_types = {
      class = "leaf type/helper",
      readiness_state = "workspace-internal",
      manifest = "docs/crate-extraction/aspen-kv-types.md",
      owner = "owner needed",
      default_feature_sets = ["default"],
      named_reusable_feature_sets = [],
      representative_consumers = ["aspen-raft", "aspen-coordination", "aspen-jobs", "aspen-kv-branch"],
      reexporters = ["aspen-raft", "aspen"],
      forbidden_unless_feature = [],
      exceptions = [
        {
          candidate = "aspen-kv-types",
          feature_set = "default",
          dependency_path = "aspen-kv-types -> aspen-constants",
          owner = "owner needed",
          reason = "KV validation limits are reusable contract data.",
        },
      ],
    },

    aspen_raft_kv_types = {
      class = "protocol/wire",
      readiness_state = "workspace-internal",
      manifest = "docs/crate-extraction/aspen-raft-kv-types.md",
      owner = "owner needed",
      default_feature_sets = ["default"],
      named_reusable_feature_sets = [],
      representative_consumers = ["aspen-redb-storage", "aspen-raft-kv", "aspen-raft-network", "aspen-raft"],
      reexporters = ["aspen-raft"],
      forbidden_unless_feature = [],
      exceptions = [
        {
          candidate = "aspen-raft-kv-types",
          feature_set = "default",
          dependency_path = "aspen-raft-kv-types -> openraft",
          owner = "owner needed",
          reason = "OpenRaft trait/type exposure is the crate purpose.",
        },
      ],
    },

    aspen_redb_storage = {
      class = "storage/backend",
      readiness_state = "workspace-internal",
      manifest = "docs/crate-extraction/aspen-redb-storage.md",
      owner = "owner needed",
      default_feature_sets = ["default"],
      named_reusable_feature_sets = ["raft-storage"],
      representative_consumers = ["aspen-raft-kv", "aspen-raft"],
      reexporters = ["aspen-raft"],
      forbidden_unless_feature = ["openraft", "redb", "tokio", "aspen-raft-kv-types"],
      exceptions = [
        {
          candidate = "aspen-redb-storage",
          feature_set = "raft-storage",
          dependency_path = "aspen-redb-storage -> redb",
          owner = "owner needed",
          reason = "Redb backend purpose.",
        },
        {
          candidate = "aspen-redb-storage",
          feature_set = "raft-storage",
          dependency_path = "aspen-redb-storage -> openraft",
          owner = "owner needed",
          reason = "OpenRaft storage trait implementation.",
        },
      ],
    },

    aspen_raft_kv = {
      class = "service library",
      readiness_state = "workspace-internal",
      manifest = "docs/crate-extraction/aspen-raft-kv.md",
      owner = "owner needed",
      default_feature_sets = ["default"],
      named_reusable_feature_sets = ["redb", "iroh-adapter"],
      representative_consumers = ["aspen-raft", "aspen-cluster", "aspen-core-essentials-handler"],
      reexporters = ["aspen-raft"],
      forbidden_unless_feature = ["aspen-raft-network", "aspen-transport", "aspen-auth", "aspen-client-api", "aspen-core-shell", "aspen-trust", "aspen-secrets", "aspen-sql", "aspen-coordination"],
      exceptions = [
        {
          candidate = "aspen-raft-kv",
          feature_set = "default",
          dependency_path = "aspen-raft-kv -> openraft",
          owner = "owner needed",
          reason = "Consensus engine and public trait boundary.",
        },
      ],
    },

    aspen_raft_network = {
      class = "runtime adapter",
      readiness_state = "workspace-internal",
      manifest = "docs/crate-extraction/aspen-raft-network.md",
      owner = "owner needed",
      default_feature_sets = ["default"],
      named_reusable_feature_sets = ["sharding"],
      representative_consumers = ["aspen-raft-kv", "aspen-raft", "aspen-cluster"],
      reexporters = ["aspen-raft"],
      forbidden_unless_feature = ["aspen-sharding"],
      exceptions = [
        {
          candidate = "aspen-raft-network",
          feature_set = "default",
          dependency_path = "aspen-raft-network -> iroh",
          owner = "owner needed",
          reason = "Explicit adapter purpose.",
        },
        {
          candidate = "aspen-raft-network",
          feature_set = "default",
          dependency_path = "aspen-raft-network -> irpc",
          owner = "owner needed",
          reason = "Adapter message framing.",
        },
      ],
    },

    aspen_raft_compat = {
      class = "service library",
      readiness_state = "workspace-internal",
      manifest = "docs/crate-extraction/aspen-raft-compat.md",
      owner = "owner needed",
      default_feature_sets = ["default"],
      named_reusable_feature_sets = ["sql", "coordination", "trust", "secrets", "sharding", "testing"],
      representative_consumers = ["aspen", "aspen-cluster", "aspen-rpc-handlers", "aspen-cli"],
      reexporters = ["aspen", "aspen-cluster"],
      forbidden_unless_feature = [],
      exceptions = [
        {
          candidate = "aspen-raft-compat",
          feature_set = "default",
          dependency_path = "aspen-raft -> aspen-core-shell",
          owner = "owner needed",
          reason = "Compatibility shell for existing Aspen integration.",
        },
        {
          candidate = "aspen-raft-compat",
          feature_set = "default",
          dependency_path = "aspen-raft -> iroh",
          owner = "owner needed",
          reason = "Existing Aspen runtime networking until adapter split completes.",
        },
      ],
    },
  },
}
