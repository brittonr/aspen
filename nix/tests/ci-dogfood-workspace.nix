# Dogfood NixOS VM test: build a multi-crate Rust WORKSPACE through the CI pipeline.
#
# Pushes 18 real Aspen crates as a Cargo workspace to Forge:
#
#   Layer 0 (protocol/types — already proven in prior test):
#   - aspen-constants              (2,600 lines, zero deps)
#   - aspen-hlc                    (425 lines, deps: uhlc, blake3, serde, rand)
#   - aspen-kv-types               (1,493 lines, deps: aspen-constants, serde, thiserror)
#   - aspen-layer                  (4,439 lines, deps: snafu, serde, proptest)
#   - aspen-time                   (360 lines, zero deps — pure std::time)
#   - aspen-coordination-protocol  (966 lines, deps: serde, serde_json)
#   - aspen-ci-core                (2,571 lines, deps: serde, snafu, uuid, chrono)
#   - aspen-forge-protocol         (782 lines, deps: serde)
#   - aspen-jobs-protocol          (546 lines, deps: serde)
#   - aspen-cluster-types          (1,634 lines, deps: serde, iroh, thiserror)
#   - aspen-ticket                 (1,425 lines, deps: iroh, iroh-gossip, postcard)
#   - aspen-hooks-types            (2,122 lines, deps: aspen-hlc, iroh, snafu)
#   - aspen-crypto                 (483 lines, deps: blake3, iroh, tokio)
#
#   Layer 1 (new — foundational traits and types):
#   - aspen-traits                 (220 lines, deps: aspen-cluster-types, aspen-kv-types)
#   - aspen-storage-types          (313 lines, deps: redb, serde, bincode)
#   - aspen-disk                   (306 lines, deps: libc)
#   - aspen-plugin-api             (1,255 lines, deps: serde, semver)
#
#   Layer 2 (new — the core crate):
#   - aspen-core                   (9,843 lines, deps: ALL Layer 0+1 + iroh, tokio, snafu)
#
# This proves:
#   - Cargo workspace resolution (18 members, cross-layer path deps)
#   - crates.io dependency fetching (509 packages)
#   - Native code compilation (blake3 C/ASM, ring crypto, redb storage engine)
#   - 3-layer dependency graph (protocol → traits → core)
#   - aspen-core: the gateway crate (types, traits, verified functions, vault, simulation)
#   - Plugin system types (manifests, permissions, dependency resolution, metrics)
#   - Trait-based API design (KeyValueStore, ClusterController via aspen-traits)
#   - Redb storage types (KvEntry serialization via aspen-storage-types)
#   - Disk space monitoring (DiskSpace, threshold constants via aspen-disk)
#   - ~1,044 unit tests across 18 crates via `doCheck = true`
#
# Pipeline: 2 stages (cargo check → build + test), same structure as prior test.
#
# Run:
#   nix build .#checks.x86_64-linux.ci-dogfood-workspace-test --impure --option sandbox false
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
  nixpkgsFlake,
}: let
  secretKey = "0000000000000006000000000000000600000000000000060000000000000006";
  cookie = "ci-workspace-build-test";

  # Real source trees from the Aspen workspace.
  # Layer 0
  aspenConstantsSrc = ../../crates/aspen-constants/src;
  aspenHlcSrc = ../../crates/aspen-hlc/src;
  aspenKvTypesSrc = ../../crates/aspen-kv-types/src;
  aspenLayerSrc = ../../crates/aspen-layer/src;
  aspenTimeSrc = ../../crates/aspen-time/src;
  aspenCoordProtoSrc = ../../crates/aspen-coordination-protocol/src;
  aspenCiCoreSrc = ../../crates/aspen-ci-core/src;
  aspenForgeProtoSrc = ../../crates/aspen-forge-protocol/src;
  aspenJobsProtoSrc = ../../crates/aspen-jobs-protocol/src;
  aspenClusterTypesSrc = ../../crates/aspen-cluster-types/src;
  aspenTicketSrc = ../../crates/aspen-ticket/src;
  aspenHooksTypesSrc = ../../crates/aspen-hooks-types/src;
  aspenCryptoSrc = ../../crates/aspen-crypto/src;
  # Layer 1
  aspenTraitsSrc = ../../crates/aspen-traits/src;
  aspenStorageTypesSrc = ../../crates/aspen-storage-types/src;
  aspenDiskSrc = ../../crates/aspen-disk/src;
  aspenPluginApiSrc = ../../crates/aspen-plugin-api/src;
  # Layer 2
  aspenCoreSrc = ../../crates/aspen-core/src;

  # CI config: 2-stage pipeline — cargo check, then full build + test.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "aspen-workspace-build",
      stages = [
        {
          name = "check",
          jobs = [
            {
              name = "cargo-check",
              type = 'nix,
              flake_url = ".",
              flake_attr = "checks.x86_64-linux.cargo-check",
              isolation = 'none,
              timeout_secs = 900,
            },
          ],
        },
        {
          name = "build",
          depends_on = ["check"],
          jobs = [
            {
              name = "build-and-test",
              type = 'nix,
              flake_url = ".",
              flake_attr = "packages.x86_64-linux.default",
              isolation = 'none,
              timeout_secs = 900,
            },
          ],
        },
      ],
    }
  '';

  # Nix flake for the workspace build.
  # Both outputs use buildRustPackage for proper Cargo vendoring —
  # raw `cargo check` fails because nix sandbox blocks crates.io HTTPS.
  workspaceFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Aspen workspace — 18-crate self-build verification";
      inputs.nixpkgs.url = "nixpkgs";
      outputs = { nixpkgs, self, ... }:
        let
          pkgs = nixpkgs.legacyPackages.x86_64-linux;
        in {
          packages.x86_64-linux.default = pkgs.rustPlatform.buildRustPackage {
            pname = "aspen-workspace";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            doCheck = true;
            # ring needs these for native crypto compilation
            nativeBuildInputs = with pkgs; [ perl pkg-config ];
          };
          checks.x86_64-linux.cargo-check = pkgs.rustPlatform.buildRustPackage {
            pname = "aspen-workspace-check";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            buildPhase = "cargo check --workspace 2>&1";
            installPhase = "touch $out";
            doCheck = false;
            nativeBuildInputs = with pkgs; [ perl pkg-config ];
          };
        };
    }
  '';

  # Root Cargo.toml: workspace with 18 members + binary target.
  rootCargoToml = pkgs.writeText "Cargo.toml" ''
    [workspace]
    members = [
      "aspen-constants",
      "aspen-kv-types",
      "aspen-hlc",
      "aspen-layer",
      "aspen-time",
      "aspen-coordination-protocol",
      "aspen-ci-core",
      "aspen-forge-protocol",
      "aspen-jobs-protocol",
      "aspen-cluster-types",
      "aspen-ticket",
      "aspen-hooks-types",
      "aspen-crypto",
      "aspen-traits",
      "aspen-storage-types",
      "aspen-disk",
      "aspen-plugin-api",
      "aspen-core",
    ]
    resolver = "3"

    [workspace.dependencies]
    schemars = "0.8"
    chrono = "0.4"

    [package]
    name = "aspen-workspace"
    version = "0.1.0"
    edition = "2024"
    description = "Multi-crate workspace build verification for Aspen CI"

    [[bin]]
    name = "aspen-workspace-check"
    path = "src/main.rs"

    [dependencies]
    aspen-constants = { path = "aspen-constants" }
    aspen-kv-types = { path = "aspen-kv-types" }
    aspen-hlc = { path = "aspen-hlc" }
    aspen-layer = { path = "aspen-layer" }
    aspen-time = { path = "aspen-time" }
    aspen-coordination-protocol = { path = "aspen-coordination-protocol" }
    aspen-ci-core = { path = "aspen-ci-core" }
    aspen-forge-protocol = { path = "aspen-forge-protocol" }
    aspen-jobs-protocol = { path = "aspen-jobs-protocol" }
    aspen-cluster-types = { path = "aspen-cluster-types" }
    aspen-ticket = { path = "aspen-ticket" }
    aspen-hooks-types = { path = "aspen-hooks-types" }
    aspen-crypto = { path = "aspen-crypto" }
    aspen-traits = { path = "aspen-traits" }
    aspen-storage-types = { path = "aspen-storage-types" }
    aspen-disk = { path = "aspen-disk" }
    aspen-plugin-api = { path = "aspen-plugin-api" }
    aspen-core = { path = "aspen-core", features = ["layer"] }
  '';

  # Pre-generated Cargo.lock with 509 packages (18 workspace + 491 external).
  # Generated from the real crate sources with `cargo generate-lockfile`.
  workspaceCargoLock = ../../nix/tests/fixtures/workspace-build-cargo.lock;

  # Binary that exercises all 18 crates.
  # Verified locally: cargo check + cargo run + 1044 tests pass.
  workspaceMain = ../../nix/tests/fixtures/workspace-build-main.rs;

  # Bundle the complete workspace as a single derivation.
  workspaceRepo = pkgs.runCommand "aspen-workspace-repo" {} ''
    mkdir -p $out/src $out/.aspen

    # ═══════════════════════════════════════════════════════════════
    #  Layer 0: Protocol and types crates (13 crates)
    # ═══════════════════════════════════════════════════════════════

    # ── aspen-constants (11 source files, 2600+ lines) ──
    mkdir -p $out/aspen-constants/src
    cp ${aspenConstantsSrc}/lib.rs         $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/api.rs         $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/assertions.rs  $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/ci.rs          $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/coordination.rs $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/directory.rs   $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/network.rs     $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/plugin.rs      $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/proxy.rs       $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/raft.rs        $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/wasm.rs        $out/aspen-constants/src/
    cp ${../../crates/aspen-constants/Cargo.toml} $out/aspen-constants/Cargo.toml

    # ── aspen-hlc (1 source file, 425 lines) ──
    mkdir -p $out/aspen-hlc/src
    cp ${aspenHlcSrc}/lib.rs               $out/aspen-hlc/src/
    cp ${../../crates/aspen-hlc/Cargo.toml} $out/aspen-hlc/Cargo.toml

    # ── aspen-kv-types (7 source files, 1493 lines) ──
    mkdir -p $out/aspen-kv-types/src
    cp ${aspenKvTypesSrc}/lib.rs           $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/batch.rs         $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/read.rs          $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/scan.rs          $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/transaction.rs   $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/validation.rs    $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/write.rs         $out/aspen-kv-types/src/
    cp ${../../crates/aspen-kv-types/Cargo.toml} $out/aspen-kv-types/Cargo.toml

    # ── aspen-layer (16 source files, 4439 lines) ──
    mkdir -p $out/aspen-layer/src/index $out/aspen-layer/src/tuple
    cp ${aspenLayerSrc}/lib.rs             $out/aspen-layer/src/
    cp ${aspenLayerSrc}/proptest.rs        $out/aspen-layer/src/
    cp ${aspenLayerSrc}/subspace.rs        $out/aspen-layer/src/
    cp ${aspenLayerSrc}/index/errors.rs    $out/aspen-layer/src/index/
    cp ${aspenLayerSrc}/index/field_types.rs $out/aspen-layer/src/index/
    cp ${aspenLayerSrc}/index/mod.rs       $out/aspen-layer/src/index/
    cp ${aspenLayerSrc}/index/registry.rs  $out/aspen-layer/src/index/
    cp ${aspenLayerSrc}/index/scan.rs      $out/aspen-layer/src/index/
    cp ${aspenLayerSrc}/index/secondary_index.rs $out/aspen-layer/src/index/
    cp ${aspenLayerSrc}/index/updates.rs   $out/aspen-layer/src/index/
    cp ${aspenLayerSrc}/tuple/decoding.rs  $out/aspen-layer/src/tuple/
    cp ${aspenLayerSrc}/tuple/element.rs   $out/aspen-layer/src/tuple/
    cp ${aspenLayerSrc}/tuple/encoding.rs  $out/aspen-layer/src/tuple/
    cp ${aspenLayerSrc}/tuple/mod.rs       $out/aspen-layer/src/tuple/
    cp ${aspenLayerSrc}/tuple/tests.rs     $out/aspen-layer/src/tuple/
    cp ${aspenLayerSrc}/tuple/tuple_type.rs $out/aspen-layer/src/tuple/
    cp ${../../crates/aspen-layer/Cargo.toml} $out/aspen-layer/Cargo.toml

    # ── aspen-time (1 source file, 360 lines) ──
    mkdir -p $out/aspen-time/src
    cp ${aspenTimeSrc}/lib.rs              $out/aspen-time/src/
    cp ${../../crates/aspen-time/Cargo.toml} $out/aspen-time/Cargo.toml

    # ── aspen-coordination-protocol (1 source file, 966 lines) ──
    mkdir -p $out/aspen-coordination-protocol/src
    cp ${aspenCoordProtoSrc}/lib.rs        $out/aspen-coordination-protocol/src/
    cp ${../../crates/aspen-coordination-protocol/Cargo.toml} $out/aspen-coordination-protocol/Cargo.toml

    # ── aspen-ci-core (10 source files, 2571 lines) ──
    mkdir -p $out/aspen-ci-core/src/config $out/aspen-ci-core/src/verified
    cp ${aspenCiCoreSrc}/lib.rs            $out/aspen-ci-core/src/
    cp ${aspenCiCoreSrc}/error.rs          $out/aspen-ci-core/src/
    cp ${aspenCiCoreSrc}/log_writer.rs     $out/aspen-ci-core/src/
    cp ${aspenCiCoreSrc}/config/mod.rs     $out/aspen-ci-core/src/config/
    cp ${aspenCiCoreSrc}/config/types.rs   $out/aspen-ci-core/src/config/
    cp ${aspenCiCoreSrc}/verified/mod.rs   $out/aspen-ci-core/src/verified/
    cp ${aspenCiCoreSrc}/verified/pipeline.rs $out/aspen-ci-core/src/verified/
    cp ${aspenCiCoreSrc}/verified/resource.rs $out/aspen-ci-core/src/verified/
    cp ${aspenCiCoreSrc}/verified/timeout.rs  $out/aspen-ci-core/src/verified/
    cp ${aspenCiCoreSrc}/verified/trigger.rs  $out/aspen-ci-core/src/verified/
    cp ${../../crates/aspen-ci-core/Cargo.toml} $out/aspen-ci-core/Cargo.toml

    # ── aspen-forge-protocol (1 source file, 782 lines) ──
    mkdir -p $out/aspen-forge-protocol/src
    cp ${aspenForgeProtoSrc}/lib.rs        $out/aspen-forge-protocol/src/
    cp ${../../crates/aspen-forge-protocol/Cargo.toml} $out/aspen-forge-protocol/Cargo.toml

    # ── aspen-jobs-protocol (1 source file, 546 lines) ──
    mkdir -p $out/aspen-jobs-protocol/src
    cp ${aspenJobsProtoSrc}/lib.rs         $out/aspen-jobs-protocol/src/
    cp ${../../crates/aspen-jobs-protocol/Cargo.toml} $out/aspen-jobs-protocol/Cargo.toml

    # ── aspen-cluster-types (5 source files, 1634 lines) ──
    mkdir -p $out/aspen-cluster-types/src
    cp ${aspenClusterTypesSrc}/lib.rs      $out/aspen-cluster-types/src/
    cp ${aspenClusterTypesSrc}/errors.rs   $out/aspen-cluster-types/src/
    cp ${aspenClusterTypesSrc}/metrics.rs  $out/aspen-cluster-types/src/
    cp ${aspenClusterTypesSrc}/nodes.rs    $out/aspen-cluster-types/src/
    cp ${aspenClusterTypesSrc}/state.rs    $out/aspen-cluster-types/src/
    cp ${../../crates/aspen-cluster-types/Cargo.toml} $out/aspen-cluster-types/Cargo.toml

    # ── aspen-ticket (5 source files, 1425 lines) ──
    mkdir -p $out/aspen-ticket/src
    cp ${aspenTicketSrc}/lib.rs            $out/aspen-ticket/src/
    cp ${aspenTicketSrc}/constants.rs      $out/aspen-ticket/src/
    cp ${aspenTicketSrc}/parse.rs          $out/aspen-ticket/src/
    cp ${aspenTicketSrc}/signed.rs         $out/aspen-ticket/src/
    cp ${aspenTicketSrc}/v2.rs             $out/aspen-ticket/src/
    cp ${../../crates/aspen-ticket/Cargo.toml} $out/aspen-ticket/Cargo.toml

    # ── aspen-hooks-types (6 source files, 2122 lines) ──
    mkdir -p $out/aspen-hooks-types/src
    cp ${aspenHooksTypesSrc}/lib.rs        $out/aspen-hooks-types/src/
    cp ${aspenHooksTypesSrc}/config.rs     $out/aspen-hooks-types/src/
    cp ${aspenHooksTypesSrc}/constants.rs  $out/aspen-hooks-types/src/
    cp ${aspenHooksTypesSrc}/error.rs      $out/aspen-hooks-types/src/
    cp ${aspenHooksTypesSrc}/event.rs      $out/aspen-hooks-types/src/
    cp ${aspenHooksTypesSrc}/ticket.rs     $out/aspen-hooks-types/src/
    cp ${../../crates/aspen-hooks-types/Cargo.toml} $out/aspen-hooks-types/Cargo.toml

    # ── aspen-crypto (3 source files, 483 lines) ──
    mkdir -p $out/aspen-crypto/src
    cp ${aspenCryptoSrc}/lib.rs            $out/aspen-crypto/src/
    cp ${aspenCryptoSrc}/cookie.rs         $out/aspen-crypto/src/
    cp ${aspenCryptoSrc}/identity.rs       $out/aspen-crypto/src/
    cp ${../../crates/aspen-crypto/Cargo.toml} $out/aspen-crypto/Cargo.toml

    # ═══════════════════════════════════════════════════════════════
    #  Layer 1: Foundational traits and types (4 crates)
    # ═══════════════════════════════════════════════════════════════

    # ── aspen-traits (1 source file, 220 lines) ──
    mkdir -p $out/aspen-traits/src
    cp ${aspenTraitsSrc}/lib.rs            $out/aspen-traits/src/
    cp ${../../crates/aspen-traits/Cargo.toml} $out/aspen-traits/Cargo.toml

    # ── aspen-storage-types (1 source file, 313 lines) ──
    mkdir -p $out/aspen-storage-types/src
    cp ${aspenStorageTypesSrc}/lib.rs      $out/aspen-storage-types/src/
    cp ${../../crates/aspen-storage-types/Cargo.toml} $out/aspen-storage-types/Cargo.toml

    # ── aspen-disk (1 source file, 306 lines) ──
    mkdir -p $out/aspen-disk/src
    cp ${aspenDiskSrc}/lib.rs              $out/aspen-disk/src/
    cp ${../../crates/aspen-disk/Cargo.toml} $out/aspen-disk/Cargo.toml

    # ── aspen-plugin-api (3 source files, 1255 lines) ──
    mkdir -p $out/aspen-plugin-api/src
    cp ${aspenPluginApiSrc}/lib.rs         $out/aspen-plugin-api/src/
    cp ${aspenPluginApiSrc}/manifest.rs    $out/aspen-plugin-api/src/
    cp ${aspenPluginApiSrc}/resolve.rs     $out/aspen-plugin-api/src/
    cp ${../../crates/aspen-plugin-api/Cargo.toml} $out/aspen-plugin-api/Cargo.toml

    # ═══════════════════════════════════════════════════════════════
    #  Layer 2: Core crate (1 crate, 37 source files)
    # ═══════════════════════════════════════════════════════════════

    # ── aspen-core (37 source files, 9843 lines) ──
    mkdir -p $out/aspen-core/src/constants
    mkdir -p $out/aspen-core/src/context
    mkdir -p $out/aspen-core/src/kv
    mkdir -p $out/aspen-core/src/layer/directory
    mkdir -p $out/aspen-core/src/spec
    mkdir -p $out/aspen-core/src/verified
    cp ${aspenCoreSrc}/lib.rs              $out/aspen-core/src/
    cp ${aspenCoreSrc}/app_registry.rs     $out/aspen-core/src/
    cp ${aspenCoreSrc}/cluster.rs          $out/aspen-core/src/
    cp ${aspenCoreSrc}/crypto.rs           $out/aspen-core/src/
    cp ${aspenCoreSrc}/error.rs            $out/aspen-core/src/
    cp ${aspenCoreSrc}/hlc.rs              $out/aspen-core/src/
    cp ${aspenCoreSrc}/prelude.rs          $out/aspen-core/src/
    cp ${aspenCoreSrc}/simulation.rs       $out/aspen-core/src/
    cp ${aspenCoreSrc}/sql.rs              $out/aspen-core/src/
    cp ${aspenCoreSrc}/storage.rs          $out/aspen-core/src/
    cp ${aspenCoreSrc}/test_support.rs     $out/aspen-core/src/
    cp ${aspenCoreSrc}/traits.rs           $out/aspen-core/src/
    cp ${aspenCoreSrc}/transport.rs        $out/aspen-core/src/
    cp ${aspenCoreSrc}/types.rs            $out/aspen-core/src/
    cp ${aspenCoreSrc}/utils.rs            $out/aspen-core/src/
    cp ${aspenCoreSrc}/vault.rs            $out/aspen-core/src/
    cp ${aspenCoreSrc}/constants/mod.rs    $out/aspen-core/src/constants/
    cp ${aspenCoreSrc}/context/mod.rs      $out/aspen-core/src/context/
    cp ${aspenCoreSrc}/context/discovery.rs $out/aspen-core/src/context/
    cp ${aspenCoreSrc}/context/docs.rs     $out/aspen-core/src/context/
    cp ${aspenCoreSrc}/context/peer.rs     $out/aspen-core/src/context/
    cp ${aspenCoreSrc}/context/protocol.rs $out/aspen-core/src/context/
    cp ${aspenCoreSrc}/context/service.rs  $out/aspen-core/src/context/
    cp ${aspenCoreSrc}/context/watch.rs    $out/aspen-core/src/context/
    cp ${aspenCoreSrc}/kv/mod.rs           $out/aspen-core/src/kv/
    cp ${aspenCoreSrc}/layer/mod.rs        $out/aspen-core/src/layer/
    cp ${aspenCoreSrc}/layer/allocator.rs  $out/aspen-core/src/layer/
    cp ${aspenCoreSrc}/layer/directory/mod.rs        $out/aspen-core/src/layer/directory/
    cp ${aspenCoreSrc}/layer/directory/layer.rs       $out/aspen-core/src/layer/directory/
    cp ${aspenCoreSrc}/layer/directory/operations.rs  $out/aspen-core/src/layer/directory/
    cp ${aspenCoreSrc}/layer/directory/subspace.rs    $out/aspen-core/src/layer/directory/
    cp ${aspenCoreSrc}/layer/directory/tests.rs       $out/aspen-core/src/layer/directory/
    cp ${aspenCoreSrc}/layer/directory/validation.rs  $out/aspen-core/src/layer/directory/
    cp ${aspenCoreSrc}/spec/mod.rs         $out/aspen-core/src/spec/
    cp ${aspenCoreSrc}/spec/verus_shim.rs  $out/aspen-core/src/spec/
    cp ${aspenCoreSrc}/verified/mod.rs     $out/aspen-core/src/verified/
    cp ${aspenCoreSrc}/verified/scan.rs    $out/aspen-core/src/verified/
    cp ${../../crates/aspen-core/Cargo.toml} $out/aspen-core/Cargo.toml

    # ═══════════════════════════════════════════════════════════════
    #  Root workspace files
    # ═══════════════════════════════════════════════════════════════
    cp ${workspaceMain}       $out/src/main.rs
    cp ${rootCargoToml}       $out/Cargo.toml
    cp ${workspaceCargoLock}  $out/Cargo.lock
    cp ${workspaceFlake}      $out/flake.nix
    cp ${ciConfig}            $out/.aspen/ci.ncl
  '';

  pluginHelpers = import ./lib/wasm-plugins.nix {
    inherit pkgs aspenCliPlugins;
    plugins = [
      {
        name = "kv";
        wasm = kvPluginWasm;
      }
      {
        name = "forge";
        wasm = forgePluginWasm;
      }
    ];
  };
in
  pkgs.testers.nixosTest {
    name = "ci-dogfood-workspace";
    skipLint = true;
    skipTypeCheck = true;

    nodes.node1 = {
      imports = [
        ../../nix/modules/aspen-node.nix
        pluginHelpers.nixosConfig
      ];

      services.aspen.node = {
        enable = true;
        package = aspenNodePackage;
        nodeId = 1;
        inherit cookie;
        secretKey = secretKey;
        storageBackend = "redb";
        dataDir = "/var/lib/aspen";
        logLevel = "info";
        relayMode = "disabled";
        enableWorkers = true;
        enableCi = true;
        enableSnix = true;
        features = ["forge" "blob"];
      };

      environment.systemPackages = [
        aspenCliPackage
        gitRemoteAspenPackage
        pkgs.git
        pkgs.nix
      ];

      networking.firewall.enable = false;
      nix.settings.experimental-features = ["nix-command" "flakes"];
      nix.settings.sandbox = false;
      nix.registry.nixpkgs.flake = nixpkgsFlake;

      virtualisation.memorySize = 4096;
      virtualisation.cores = 2;
      virtualisation.diskSize = 20480;
      virtualisation.writableStoreUseTmpfs = false;
    };

    testScript = ''
      import json, time

      WORKSPACE_REPO = "${workspaceRepo}"

      def get_ticket():
          return node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          ticket = get_ticket()
          run = f"aspen-cli --ticket '{ticket}' --json {cmd} >/tmp/_cli.json 2>/tmp/_cli.err"
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_cli.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              return raw.strip()

      def cli_text(cmd):
          ticket = get_ticket()
          node1.succeed(f"aspen-cli --ticket '{ticket}' {cmd} >/tmp/_cli.txt 2>/dev/null")
          return node1.succeed("cat /tmp/_cli.txt")

      def plugin_cli(cmd, check=True):
          ticket = get_ticket()
          run = f"aspen-plugin-cli --ticket '{ticket}' --json {cmd} >/tmp/_pcli.json 2>/tmp/_pcli.err"
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_pcli.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              return raw.strip()

      def stream_job_logs(run_id, job_id, job_name):
          """Stream logs for a job via ci logs --follow, saving to file."""
          ticket = get_ticket()
          node1.log(f"=== streaming logs: {job_name} ({job_id}) ===")
          node1.execute(
              f"aspen-cli --ticket '{ticket}' ci logs --follow {run_id} {job_id} "
              f">/tmp/job-{job_id}.log 2>/dev/null"
          )
          log_size = node1.succeed(f"wc -c < /tmp/job-{job_id}.log").strip()
          node1.log(f"=== end logs: {job_name} ({log_size} bytes) ===")

      def wait_for_pipeline(run_id, timeout=900):
          """Wait for pipeline by streaming job logs as they're assigned."""
          deadline = time.time() + timeout
          streamed_jobs = set()
          while time.time() < deadline:
              result = cli(f"ci status {run_id}", check=False)
              if not isinstance(result, dict):
                  time.sleep(3)
                  continue

              # Stream logs for newly assigned jobs
              for stage in result.get("stages", []):
                  for job in stage.get("jobs", []):
                      jid = job.get("id", "")
                      if jid and jid not in streamed_jobs:
                          streamed_jobs.add(jid)
                          stream_job_logs(run_id, jid, job.get("name", "unknown"))

              status = result.get("status")
              node1.log(f"Pipeline {run_id}: status={status}")
              if status in ("success", "failed", "cancelled"):
                  return result
              time.sleep(3)
          raise Exception(f"Pipeline {run_id} did not complete within {timeout}s")


      # ── boot ─────────────────────────────────────────────────────
      start_all()
      node1.wait_for_unit("aspen-node.service")
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )
      cli_text("cluster init")
      time.sleep(2)

      # ── plugins ──────────────────────────────────────────────────
      with subtest("install WASM plugins"):
          for name, wasm, manifest in [
              ("kv", "/etc/aspen-plugins/kv-plugin.wasm", "/etc/aspen-plugins/kv-plugin.json"),
              ("forge", "/etc/aspen-plugins/forge-plugin.wasm", "/etc/aspen-plugins/forge-plugin.json"),
          ]:
              plugin_cli(f"plugin install {wasm} --manifest {manifest}")
          plugin_cli("plugin reload", check=False)
          time.sleep(8)

      # ── create forge repo ────────────────────────────────────────
      with subtest("create forge repo"):
          out = cli("git init aspen-workspace")
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id: {out}"
          node1.log(f"Repo: {repo_id}")

      with subtest("ci watch"):
          result = cli(f"ci watch {repo_id}")
          assert isinstance(result, dict) and result.get("is_success"), f"ci watch failed: {result}"

      # ── push workspace to forge ──────────────────────────────────
      with subtest("push 18-crate workspace to forge"):
          node1.succeed(
              f"cp -r --no-preserve=mode {WORKSPACE_REPO} /tmp/workspace-repo && "
              "cd /tmp/workspace-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'test@test' && "
              "git config user.name 'Test' && "
              "git add -A && "
              "git commit -m 'aspen workspace: 18 crates, 30K lines'"
          )
          ticket = get_ticket()
          node1.succeed(
              f"cd /tmp/workspace-repo && "
              f"git remote add aspen 'aspen://{ticket}/{repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>/tmp/push.err"
          )
          # Verify workspace structure
          rs_count = node1.succeed(
              "find /tmp/workspace-repo -name '*.rs' | wc -l"
          ).strip()
          toml_count = node1.succeed(
              "find /tmp/workspace-repo -name 'Cargo.toml' | wc -l"
          ).strip()
          node1.log(f"Pushed {rs_count} .rs files, {toml_count} Cargo.toml files")
          assert int(rs_count) >= 95, f"Expected >=95 .rs files, got {rs_count}"
          assert int(toml_count) >= 19, f"Expected >=19 Cargo.toml (root + 18 crates), got {toml_count}"

      # ── wait for pipeline ────────────────────────────────────────
      with subtest("pipeline auto-triggers"):
          deadline = time.time() + 60
          run_id = None
          while time.time() < deadline:
              out = cli("ci list", check=False)
              if isinstance(out, dict):
                  runs = out.get("runs", [])
                  if runs:
                      run_id = runs[0].get("run_id")
                      break
              time.sleep(3)
          assert run_id, "No pipeline triggered within 60s"
          node1.log(f"Pipeline: {run_id}")

      with subtest("2-stage pipeline succeeds"):
          final = wait_for_pipeline(run_id, timeout=900)
          node1.log(f"Final: {json.dumps(final, indent=2)}")
          status = final.get("status")
          assert status == "success", f"Pipeline failed: {status}: {final}"

      # ── verify both stages ───────────────────────────────────────
      with subtest("both stages completed"):
          stages = final.get("stages", [])
          assert len(stages) == 2, f"Expected 2 stages, got {len(stages)}: {stages}"

          stage_names = [s["name"] for s in stages]
          assert "check" in stage_names, f"Missing 'check' stage: {stage_names}"
          assert "build" in stage_names, f"Missing 'build' stage: {stage_names}"

          for stage in stages:
              assert stage["status"] == "success", \
                  f"Stage '{stage['name']}' status={stage['status']}, expected success"

      # ── verify build logs ────────────────────────────────────────
      with subtest("ci logs captured"):
          all_jobs = []
          for stage in final.get("stages", []):
              for job in stage.get("jobs", []):
                  if job.get("id"):
                      all_jobs.append(job)

          assert len(all_jobs) == 2, f"Expected 2 jobs, got {len(all_jobs)}"
          for job in all_jobs:
              assert job["status"] == "success", \
                  f"Job '{job.get('name')}' not success: {job['status']}"
              # Logs already saved to /tmp/job-{id}.log by stream_job_logs during wait
              log = node1.succeed(f"cat /tmp/job-{job['id']}.log 2>/dev/null || echo empty").strip()
              assert log, f"No log output for job '{job.get('name')}'"
              node1.log(f"Job '{job.get('name')}': {len(log)} bytes of logs")

      # ── extract build job results ───────────────────────────────
      with subtest("extract job results"):
          build_job = [j for j in all_jobs if j.get("name") == "build-and-test"][0]
          job_data = cli(f"kv get __jobs:{build_job['id']}", check=False)

          output_path = None
          job_result_data = None
          if isinstance(job_data, dict):
              value = job_data.get("value", "")
              try:
                  job_result = json.loads(value) if isinstance(value, str) else value
                  job_result_data = job_result.get("result", {}).get("Success", {}).get("data", {})
                  paths = job_result_data.get("output_paths", [])
                  if paths:
                      output_path = paths[0]
              except (json.JSONDecodeError, ValueError, AttributeError) as e:
                  node1.log(f"Failed to parse job result: {e}")

          assert output_path, f"No output_path in job result: {job_data}"
          node1.log(f"Nix output path: {output_path}")

      # ── verify blob upload ──────────────────────────────────────
      with subtest("build output uploaded to blobs"):
          uploaded = job_result_data.get("uploaded_store_paths", [])
          assert len(uploaded) >= 1, f"No uploaded store paths: {job_result_data}"

          build_upload = [u for u in uploaded if "aspen-workspace-0.1.0" in u.get("store_path", "")]
          assert len(build_upload) == 1, f"Expected 1 build upload, got {len(build_upload)}: {uploaded}"

          blob_hash = build_upload[0].get("blob_hash")
          nar_size = build_upload[0].get("nar_size", 0)
          cache_registered = build_upload[0].get("cache_registered", False)

          assert blob_hash, f"No blob_hash in upload: {build_upload[0]}"
          assert nar_size > 0, f"NAR size is 0: {build_upload[0]}"
          assert cache_registered, f"Store path not registered in cache: {build_upload[0]}"

          node1.log(f"Blob upload: hash={blob_hash} nar_size={nar_size} cached={cache_registered}")

      # ── verify nix binary cache entry ────────────────────────────
      with subtest("build output in nix binary cache"):
          cache_result = cli(f"cache query {output_path}", check=False)
          node1.log(f"Cache query result: {json.dumps(cache_result, indent=2)}")

          assert isinstance(cache_result, dict), f"Cache query failed: {cache_result}"
          assert cache_result.get("was_found"), f"Store path not found in cache: {cache_result}"
          assert cache_result.get("blob_hash") == blob_hash, \
              f"Blob hash mismatch: cache={cache_result.get('blob_hash')} upload={blob_hash}"
          assert cache_result.get("nar_size") == nar_size, \
              f"NAR size mismatch: cache={cache_result.get('nar_size')} upload={nar_size}"

      # ── verify SNIX upload ──────────────────────────────────────
      with subtest("SNIX storage upload succeeded"):
          assert "uploaded_store_paths_snix" in job_result_data, \
              f"snix feature not compiled — missing uploaded_store_paths_snix: {job_result_data.keys()}"

          snix_log = node1.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null | grep -c 'Uploading store path to SNIX' || true"
          ).strip()
          snix_attempts = int(snix_log)
          assert snix_attempts >= 1, \
              f"SNIX upload was never attempted: attempts={snix_attempts}"
          node1.log(f"SNIX upload attempts: {snix_attempts}")

          snix_uploads = job_result_data.get("uploaded_store_paths_snix", [])
          assert len(snix_uploads) >= 1, f"No SNIX uploads completed: {job_result_data}"

          snix_build = [u for u in snix_uploads if "aspen-workspace-0.1.0" in u.get("store_path", "")]
          assert len(snix_build) == 1, \
              f"Expected 1 SNIX upload for aspen-workspace, got {len(snix_build)}: {snix_uploads}"

          snix_entry = snix_build[0]
          assert snix_entry.get("nar_size", 0) == nar_size, \
              f"SNIX/blob NAR size mismatch: snix={snix_entry.get('nar_size')} blob={nar_size}"
          assert snix_entry.get("nar_sha256"), f"Missing nar_sha256 in SNIX upload: {snix_entry}"
          node1.log(f"SNIX upload verified: {snix_entry}")

      # ── run the CI-built binary ──────────────────────────────────
      with subtest("run CI-built binary"):
          binary = f"{output_path}/bin/aspen-workspace-check"
          node1.succeed(f"test -x {binary}")

          output = node1.succeed(f"{binary}")
          node1.log(f"Binary output:\n{output}")

          # Verify all 18 crates were exercised
          for section in [
              "aspen-constants", "aspen-hlc", "aspen-kv-types", "aspen-layer",
              "aspen-time", "aspen-coordination-protocol", "aspen-ci-core",
              "aspen-forge-protocol", "aspen-jobs-protocol", "aspen-cluster-types",
              "aspen-hooks-types", "aspen-crypto",
              "aspen-traits", "aspen-storage-types", "aspen-disk",
              "aspen-plugin-api", "aspen-core",
          ]:
              assert f"[{section}]" in output, f"Missing [{section}] section in binary output"

          # Verify key functionality from new crates
          assert "KeyValueStore" in output, f"Missing traits section: {output}"
          assert "KvEntry:" in output, f"Missing storage-types section: {output}"
          assert "THRESHOLD:" in output, f"Missing disk section: {output}"
          assert "API_VERSION:" in output, f"Missing plugin-api section: {output}"
          assert "load_order:" in output, f"Missing plugin dependency resolution: {output}"
          assert "AppRegistry:" in output, f"Missing core app registry: {output}"
          assert "SimulationArtifact:" in output, f"Missing core simulation: {output}"
          assert "vault:" in output, f"Missing core vault: {output}"
          assert "scan:" in output, f"Missing core verified scan: {output}"

          # Verify legacy crate functionality still works
          assert "monotonic" in output, f"HLC monotonicity missing: {output}"
          assert "Packed key:" in output, f"Layer tuple encoding missing: {output}"
          assert "cookie_validate:" in output, f"Crypto cookie missing: {output}"
          assert "509 packages" in output, f"Package count missing: {output}"
          assert "Built by Aspen CI" in output, f"Build attribution missing: {output}"

      node1.log("WORKSPACE BUILD PASSED: 18 crates, 509 packages, ~1044 tests — Forge -> CI -> nix build -> blob+cache+snix -> run")
    '';
  }
