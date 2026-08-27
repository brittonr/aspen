{
  description = "molten — Rust project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/f9d8b65950353691ab56561e7c73d2e1063d810b";
    nickel-cli.url = "github:tweag/nickel/1320a983e6c3d1e2fb53dd2464b084b4903b1426";
    flux-src = {
      url = "github:gattaca-com/flux/2a1916465ae6649aebef3758233cfea98e5d33db";
      flake = false;
    };
    unit2nix = {
      url = "github:brittonr/unit2nix/d4883180de0ce3033b7e4e2ab4216f33134863c5";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay/6cddd512fa2bf7231f098d3a2f92f6e4cff71e0a";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    onix-core-src = {
      url = "git+ssh://git@github.com/onixcomputer/onix-core.git?rev=ae895854eb049ff152d3f1b96cb90a5fa45c3ec6&shallow=1";
      flake = false;
    };
    basalt-src = {
      url = "github:OnixResearch/basalt/d913dc01e765c9b297df5fcc57dfa06aac39bc74";
      flake = false;
    };
    artifact-src = {
      url = "git+ssh://git@github.com/OnixResearch/onix-artifact.git?rev=c932138d880ddf4c2967f4c024b489b5c0022bf1";
      flake = false;
    };
    choregraph-src = {
      url = "git+rad://zL2ncTUeASVYwcoGkEXv9JKgGbAF?rev=b3e08e19750f53bdbcae970cdf58a47a791ed20b";
      flake = false;
    };
    executable-extent-src = {
      url = "git+rad://z37R1bP1kHcELs89RNbQRaqbCVKxB?rev=025d9636f0161777710dac37b3c210ca0ad9483f";
      flake = false;
    };
    executable-extent-octet = {
      url = "github:OnixResearch/octet/cf04e894e53eb0947230118a086ef6066ddba38c";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    mantle-executable-extent-src = {
      url = "git+rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb?rev=2c636b1b25353a1b0befa5af48dc68615cd686dd";
      flake = false;
    };
    kamacite-src = {
      url = "github:OnixResearch/kamacite/d76fe4abe543724d8fc0ac4b362187caf2e27622";
      flake = false;
    };
    cairn-src = {
      url = "github:OnixResearch/cairn/3b4c280b893f2709aebea21fc51a4f9eeba3fe3b";
      flake = false;
    };
    hegel-src = {
      url = "github:hegeldev/hegel-rust/ed949b8084595cb467e983747f1089e214965ac6";
      flake = false;
    };
    octet-cutover-src = {
      url = "github:OnixResearch/octet/4367300e10740ecc99ba4b2171ace561b4787327";
      flake = false;
    };
    octet-toolchain.url = "github:OnixResearch/octet?rev=fc38f59330b626961d166febfdf1a5aa6575460f";
    ucan-src = {
      url = "github:OnixResearch/ucan/2aad993027d48ff148028c537cdaf91f6e5285ca";
      flake = false;
    };
    valence-src = {
      url = "github:OnixResearch/valence/5f1c2ba5072c6f9622fa59b1af20502985f569fd";
      flake = false;
    };
    schema-migration-core-src = {
      url = "git+https://seed.radicle.garden/z1C4YVMgDGyVdQa72uPNj3UDS5cY.git?rev=4fe90e130f2871cf69a6febcdc70785adca98aea";
      flake = false;
    };
    schema-identity-src = {
      url = "git+https://seed.radicle.garden/z6gGpUJtzdVBCCtZTzh4cV1skv4H.git?rev=2562c8aa38a034061f9af9f3e17280494a5b8de2";
      flake = false;
    };
    flake-utils.url = "github:numtide/flake-utils/11707dc2f618dd54ca8739b309ec4fc024de578b";
  };

  outputs =
    {
      nixpkgs,
      nickel-cli,
      flux-src,
      unit2nix,
      rust-overlay,
      flake-utils,
      onix-core-src,
      basalt-src,
      artifact-src,
      choregraph-src,
      executable-extent-src,
      executable-extent-octet,
      mantle-executable-extent-src,
      kamacite-src,
      cairn-src,
      hegel-src,
      octet-cutover-src,
      octet-toolchain,
      ucan-src,
      valence-src,
      schema-identity-src,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgsBase = import nixpkgs {
          localSystem = system;
          overlays = [ (import rust-overlay) ];
        };

        nickelPackage = nickel-cli.packages.${system}.default;
        pkgs = pkgsBase // {
          nickel = nickelPackage;
        };

        localSourceExcludedBaseNames = [
          ".direnv"
          ".git"
          ".hegel"
          ".pre-commit-cache"
          ".pytest_cache"
          "mutants.out"
          "result"
          "target"
        ];
        cleanLocalGitSource =
          src:
          pkgs.lib.cleanSourceWith {
            inherit src;
            filter = path: _type: !(builtins.elem (baseNameOf path) localSourceExcludedBaseNames);
          };
        # Relative path inputs outside the flake root can arrive as
        # /nix/store/...-source/../repo. Do not hand those unusable paths to
        # cleanSourceWith; callers can pass real local paths with --override-input.
        maybeCleanLocalGitSource =
          src: if pkgs.lib.hasInfix "/../" (toString src) then null else cleanLocalGitSource src;
        artifactRevision = "c932138d880ddf4c2967f4c024b489b5c0022bf1";
        artifactRepository = "ssh://git@github.com/OnixResearch/onix-artifact.git";
        rootCargoManifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        artifactRootDependencies = rootCargoManifest.dependencies;
        artifactRootDevelopmentDependencies = rootCargoManifest.dev-dependencies;
        moltenCoreDependencies =
          (builtins.fromTOML (builtins.readFile ./crates/molten-core/Cargo.toml)).dependencies;
        artifactCargoDependencies = [
          artifactRootDependencies.artifact-auth-core
          artifactRootDependencies.artifact-auth-ed25519
          artifactRootDependencies.artifact-binding-core
          moltenCoreDependencies.artifact-auth-core
          moltenCoreDependencies.artifact-binding-core
        ];
        artifactRequiredPackages = [
          "artifact-auth-core"
          "artifact-auth-ed25519"
          "artifact-binding-core"
        ];
        artifactSourceExpectedMembers = [
          "crates/artifact-auth-core"
          "crates/artifact-auth-ed25519"
          "crates/artifact-binding-core"
          "crates/artifact-transfer-core"
        ];
        artifactExpectedLockSource = "git+${artifactRepository}?rev=${artifactRevision}#${artifactRevision}";
        artifactLockPackages = builtins.filter (
          package: (package.source or "") == artifactExpectedLockSource
        ) (builtins.fromTOML (builtins.readFile ./Cargo.lock)).package;
        artifactLockedPackageNames = builtins.map (package: package.name) artifactLockPackages;
        artifactWorkspace = builtins.fromTOML (builtins.readFile (artifact-src + "/Cargo.toml"));
        artifactSource =
          assert pkgs.lib.assertMsg (
            builtins.all (
              dependency: dependency.git == artifactRepository && dependency.rev == artifactRevision
            ) artifactCargoDependencies
            && artifact-src.rev == artifactRevision
            &&
              builtins.sort builtins.lessThan artifactLockedPackageNames
              == builtins.sort builtins.lessThan artifactRequiredPackages
            &&
              builtins.sort builtins.lessThan artifactWorkspace.workspace.members
              == builtins.sort builtins.lessThan artifactSourceExpectedMembers
            && artifactWorkspace.workspace.package.license == "MIT OR Apache-2.0"
          ) "Molten Artifact Cargo/Nix source identity, package set, workspace, or license drifted";
          artifact-src;
        choregraphRevision = "b3e08e19750f53bdbcae970cdf58a47a791ed20b";
        choregraphRepository = "https://seed.radicle.garden/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git";
        choregraphHistoryDependencies = [
          artifactRootDependencies."choregraph-history"
          moltenCoreDependencies."choregraph-history"
        ];
        choregraphExpectedLockSource = "git+${choregraphRepository}?rev=${choregraphRevision}#${choregraphRevision}";
        choregraphLockPackages = builtins.filter (
          package: (package.source or "") == choregraphExpectedLockSource
        ) (builtins.fromTOML (builtins.readFile ./Cargo.lock)).package;
        choregraphWorkspace = builtins.fromTOML (builtins.readFile (choregraph-src + "/Cargo.toml"));
        choregraphHistoryManifest = builtins.fromTOML (
          builtins.readFile (choregraph-src + "/crates/choregraph-history/Cargo.toml")
        );
        choregraphSource =
          assert pkgs.lib.assertMsg (
            builtins.all (
              dependency:
              dependency.git == choregraphRepository
              && dependency.rev == choregraphRevision
              && dependency.version == "0.1.0"
            ) choregraphHistoryDependencies
            && choregraph-src.rev == choregraphRevision
            && builtins.length choregraphLockPackages == 1
            && (builtins.head choregraphLockPackages).name == "choregraph-history"
            && builtins.elem "crates/choregraph-history" choregraphWorkspace.workspace.members
            && choregraphWorkspace.workspace.package.license == "AGPL-3.0-or-later"
            && choregraphHistoryManifest.package.name == "choregraph-history"
          ) "Molten Choregraph branch-history Cargo/Nix source identity, package, or license drifted";
          choregraph-src;
        executableExtentRevision = "025d9636f0161777710dac37b3c210ca0ad9483f";
        executableExtentRepository = "rad://z37R1bP1kHcELs89RNbQRaqbCVKxB";
        executableExtentOctetRevision = "cf04e894e53eb0947230118a086ef6066ddba38c";
        executableExtentOctetAdmitted =
          assert pkgs.lib.assertMsg (
            executable-extent-octet.rev == executableExtentOctetRevision
          ) "Molten executable-extent Octet input drifted";
          true;
        executableExtentDependencies = [
          artifactRootDependencies.executable-extent-core
          artifactRootDependencies.executable-extent-conformance
          artifactRootDependencies.executable-extent-linux
          moltenCoreDependencies.executable-extent-core
        ];
        executableExtentExpectedPackages = [
          "executable-extent-conformance"
          "executable-extent-core"
          "executable-extent-linux"
        ];
        executableExtentExpectedLockSource = "git+${executableExtentRepository}?rev=${executableExtentRevision}#${executableExtentRevision}";
        executableExtentLockPackages = builtins.filter (
          package: (package.source or "") == executableExtentExpectedLockSource
        ) (builtins.fromTOML (builtins.readFile ./Cargo.lock)).package;
        executableExtentWorkspace = builtins.fromTOML (
          builtins.readFile (executable-extent-src + "/Cargo.toml")
        );
        executableExtentSource =
          assert pkgs.lib.assertMsg (
            builtins.all (
              dependency:
              dependency.git == executableExtentRepository && dependency.rev == executableExtentRevision
            ) executableExtentDependencies
            && executable-extent-src.rev == executableExtentRevision
            &&
              builtins.sort builtins.lessThan (map (package: package.name) executableExtentLockPackages)
              == executableExtentExpectedPackages
            &&
              builtins.sort builtins.lessThan executableExtentWorkspace.workspace.members == [
                "crates/executable-extent-conformance"
                "crates/executable-extent-core"
                "crates/executable-extent-linux"
              ]
            && executableExtentWorkspace.workspace.dependencies.blake3.version == ">=1.8.2, <2"
          ) "Molten executable-extent Cargo/Nix source identity, package set, or hash cohort drifted";
          executable-extent-src;
        mantleExecutableExtentRevision = "2c636b1b25353a1b0befa5af48dc68615cd686dd";
        mantleExecutableExtentSource =
          assert pkgs.lib.assertMsg (
            mantle-executable-extent-src.rev == mantleExecutableExtentRevision
          ) "Molten Mantle producer Nix source identity drifted";
          mantle-executable-extent-src;
        kamaciteRevision = "d76fe4abe543724d8fc0ac4b362187caf2e27622";
        kamaciteRepository = "ssh://git@github.com/OnixResearch/kamacite.git";
        kamaciteCargoDependencies = [
          artifactRootDependencies.kamacite-core
          moltenCoreDependencies.kamacite-core
        ];
        kamaciteSource =
          assert pkgs.lib.assertMsg (
            builtins.all (
              dependency: dependency.git == kamaciteRepository && dependency.rev == kamaciteRevision
            ) kamaciteCargoDependencies
            && kamacite-src.rev == kamaciteRevision
          ) "Molten Kamacite Cargo/Nix source identity drifted";
          kamacite-src;
        schemaIdentityRevision = "2562c8aa38a034061f9af9f3e17280494a5b8de2";
        schemaIdentityRepository = "https://seed.radicle.garden/z6gGpUJtzdVBCCtZTzh4cV1skv4H.git";
        schemaIdentityCargoDependencies = [
          artifactRootDependencies.schema-identity-core
          artifactRootDevelopmentDependencies.schema-identity-conformance
        ];
        schemaIdentitySource =
          assert pkgs.lib.assertMsg (
            builtins.all (
              dependency: dependency.git == schemaIdentityRepository && dependency.rev == schemaIdentityRevision
            ) schemaIdentityCargoDependencies
            && schema-identity-src.rev == schemaIdentityRevision
          ) "Molten schema-identity Cargo/Nix source identity drifted";
          schema-identity-src;
        localGitSources = pkgs.lib.filterAttrs (_key: src: src != null) {
          "${artifactRepository}#${artifactRevision}" = maybeCleanLocalGitSource artifactSource;
          "${executableExtentRepository}#${executableExtentRevision}" =
            maybeCleanLocalGitSource executableExtentSource;
          "${kamaciteRepository}#${kamaciteRevision}" = maybeCleanLocalGitSource kamaciteSource;
          "ssh://git@github.com/OnixResearch/basalt.git#d913dc01e765c9b297df5fcc57dfa06aac39bc74" =
            maybeCleanLocalGitSource basalt-src;
          "ssh://git@github.com/OnixResearch/cairn.git#3b4c280b893f2709aebea21fc51a4f9eeba3fe3b" =
            maybeCleanLocalGitSource cairn-src;
          "ssh://git@github.com/OnixResearch/ucan.git#2aad993027d48ff148028c537cdaf91f6e5285ca" =
            maybeCleanLocalGitSource ucan-src;
          "ssh://git@github.com/OnixResearch/valence.git#5f1c2ba5072c6f9622fa59b1af20502985f569fd" =
            maybeCleanLocalGitSource valence-src;
          "https://github.com/hegeldev/hegel-rust#ed949b8084595cb467e983747f1089e214965ac6" =
            maybeCleanLocalGitSource hegel-src;
          "${schemaIdentityRepository}#${schemaIdentityRevision}" =
            maybeCleanLocalGitSource schemaIdentitySource;
        };
        unit2nixPkgsBase = pkgsBase.extend (
          final: prev: {
            fetchgit =
              (prev.lib.makeOverridable (
                args:
                let
                  localGitKey = if args ? rev then "${args.url}#${args.rev}" else "";
                  localGitSource = localGitSources.${localGitKey} or null;
                in
                if localGitSource != null then
                  prev.runCommand (args.name or "local-git-source") { } ''
                    cp -R ${localGitSource} "$out"
                    chmod -R u+w "$out"
                    ${args.postFetch or ""}
                  ''
                else
                  prev.fetchgit args
              ))
              // {
                inherit (prev.fetchgit) getRevWithTag;
              };
          }
        );

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        fluxProfilerRustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
        fluxProfilerCli =
          if system == "x86_64-linux" then
            fluxProfilerRustPlatform.buildRustPackage {
              pname = "flux-profiler";
              version = "0.1.3-${builtins.substring 0 8 flux-src.rev}";
              src = flux-src;
              cargoLock.lockFile = "${flux-src}/Cargo.lock";
              cargoBuildFlags = [
                "-p"
                "flux-profiler"
                "--bin"
                "flux-profiler"
              ];
              cargoInstallFlags = [
                "-p"
                "flux-profiler"
                "--bin"
                "flux-profiler"
              ];
              doCheck = false;
              postInstall = ''
                $out/bin/flux-profiler --help | grep --fixed-strings -- "--duration"
                $out/bin/flux-profiler --help | grep --fixed-strings -- "--max-mem"
              '';
            }
          else
            null;
        rustToolchainCompat = rustToolchain // {
          unwrapped = rustToolchain // {
            configureFlags = [ "--target=${pkgs.stdenv.hostPlatform.rust.rustcTarget}" ];
          };
        };
        unit2nixPkgs = unit2nixPkgsBase.extend (
          final: prev: {
            # unit2nix auto mode forwards the custom toolchain to Cargo's
            # unit-graph generation, but this pinned revision does not forward it
            # to the clippy wrapper. Make pkgs.clippy/pkgs.rustc match the
            # dependency compiler so Nix flake checks do not mix rustc metadata.
            clippy = rustToolchain;
            rustc = rustToolchainCompat;
          }
        );

        kacheCacheDir = "/var/cache/kache-nix";
        kacheKeySalt = "molten-unit2nix-kache-v1";
        kachePackage = pkgs.callPackage (onix-core-src + "/pkgs/kache") { };
        kacheLib = import (onix-core-src + "/lib/kache-nix-rust.nix") {
          lib = pkgs.lib;
          inherit pkgs kachePackage;
        };
        mkUnit2nixRust =
          {
            enableKache ? false,
            cacheDir ? kacheCacheDir,
            keySalt ? kacheKeySalt,
          }:
          if enableKache then
            kacheLib.mkWrappedRustPackage {
              name = "molten-kache-rust";
              rust = rustToolchain;
              inherit cacheDir keySalt;
            }
          else
            rustToolchain;
        mkBuildRustCrateForPkgs =
          {
            enableKache ? false,
            cacheDir ? kacheCacheDir,
            keySalt ? kacheKeySalt,
          }:
          pkgs:
          let
            unit2nixRust = mkUnit2nixRust { inherit enableKache cacheDir keySalt; };
          in
          pkgs.buildRustCrate.override {
            cargo = unit2nixRust;
            rustc = unit2nixRust;
          };
        mkUnit2nixWorkspace =
          {
            enableKache ? false,
            cacheDir ? kacheCacheDir,
            keySalt ? kacheKeySalt,
            resolvedJson ? ./build-plan.json,
          }:
          unit2nix.lib.${system}.buildFromUnitGraph {
            pkgs = unit2nixPkgs;
            inherit rustToolchain;
            src = ./.;
            # Keep unit graphs checked in so package evaluation does not
            # depend on unit2nix IFD.
            inherit resolvedJson;
            clippyArgs = [
              "-D"
              "warnings"
            ];
            buildRustCrateForPkgs = mkBuildRustCrateForPkgs { inherit enableKache cacheDir keySalt; };
            extraCrateOverrides = {
              # nickel-lang-core declares links="nix", but the Nix FFI is behind
              # the disabled nix-experimental feature in this workspace.
              nickel-lang-core = attrs: { };
              # verus_prettyplease declares links="prettyplease-verus02" but
              # vendors its implementation; no native libraries are required.
              verus_prettyplease = attrs: { };
              # kamacite-core includes governed adapter and Wasm fixtures from
              # the producer workspace root. Keep that full immutable source
              # while compiling from the crate subdirectory.
              kamacite-core = attrs: {
                src = kamaciteSource;
                sourceRoot = "source/crates/kamacite-core";
              };
            };
          };

        ws = mkUnit2nixWorkspace { enableKache = false; };
        releasePolicyWs = mkUnit2nixWorkspace {
          enableKache = false;
          resolvedJson = ./release-policy-build-plan.json;
        };
        kacheWs = mkUnit2nixWorkspace { enableKache = true; };
        kacheWrappedRust = mkUnit2nixRust { enableKache = true; };

        moltenPkg = ws.workspaceMembers."molten".build;
        moltenNodeHostPkg = ws.workspaceMembers."molten-node-host".build;
        moltenNodeHostTests = ws.test.workspaceMembers."molten-node-host".build;
        releasePolicyPkg = releasePolicyWs.rootCrate.build;
        moltenTestBinaries = (ws.test.workspaceMembers."molten".build).override { buildTests = true; };
        targetTriple = pkgs.stdenv.hostPlatform.rust.rustcTarget;
        rustLibDir = "${rustToolchain}/lib/rustlib/${targetTriple}/lib";
        nextestCi = pkgs.writeShellApplication {
          name = "molten-nextest-ci";
          runtimeInputs = [
            rustToolchain
            pkgs.cargo-nextest
          ];
          text = ''
            exec cargo nextest run --profile ci "$@"
          '';
        };
        sourceForConfigChecks = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            let
              base = baseNameOf path;
            in
            !(base == "target" || base == ".direnv" || base == ".git");
        };
        executableExtentOctetWorkspace = pkgs.runCommand "molten-executable-extent-octet-workspace" { } ''
          mkdir -p \
            "$out/src" \
            "$out/product/molten-core/src"
          cp ${./checks/executable-extent-octet/Cargo.toml} "$out/Cargo.toml"
          cp ${./checks/executable-extent-octet/Cargo.lock} "$out/Cargo.lock"
          cp ${./checks/executable-extent-octet/dylint.toml} "$out/dylint.toml"
          cp ${./checks/executable-extent-octet/src/lib.rs} "$out/src/lib.rs"
          substituteInPlace "$out/Cargo.toml" \
            --replace-fail 'vendor/executable-extent' '${executableExtentSource}' \
            --replace-fail 'vendor/artifact' '${artifactSource}'
          cp ${./checks/executable-extent-octet/molten-core.Cargo.toml} \
            "$out/product/molten-core/Cargo.toml"
          cp ${./checks/executable-extent-octet/molten-core.lib.rs} \
            "$out/product/molten-core/src/lib.rs"
          cp -R ${./src/executable_extent} "$out/product/executable_extent"
          cp -R ${./crates/molten-core/src/executable_extent} \
            "$out/product/molten-core/src/executable_extent"
        '';
        worldCommitOctetWorkspace = pkgs.runCommand "molten-world-commit-octet-workspace" { } ''
          mkdir -p "$out/src"
          cp ${./checks/world-commit-octet/Cargo.toml} "$out/Cargo.toml"
          cp ${./checks/world-commit-octet/Cargo.lock} "$out/Cargo.lock"
          cp ${./checks/world-commit-octet/dylint.toml} "$out/dylint.toml"
          cp ${./checks/world-commit-octet/src/lib.rs} "$out/src/lib.rs"
          cp -R ${./crates/molten-core/src/worldcommit} "$out/src/world_commit"
        '';
        worldHeadOctetWorkspace = pkgs.runCommand "molten-world-head-octet-workspace" { } ''
          mkdir -p \
            "$out/src" \
            "$out/vendor/choregraph-history"
          cp ${./checks/world-head-octet/Cargo.toml} "$out/Cargo.toml"
          cp ${./checks/world-head-octet/Cargo.lock} "$out/Cargo.lock"
          cp ${./checks/world-head-octet/dylint.toml} "$out/dylint.toml"
          cp ${./checks/world-head-octet/src/lib.rs} "$out/src/lib.rs"
          cp ${./checks/world-head-octet/choregraph-history.Cargo.toml} \
            "$out/vendor/choregraph-history/Cargo.toml"
          cp -R ${choregraphSource}/crates/choregraph-history/src \
            "$out/vendor/choregraph-history/src"
          cp -R ${./crates/molten-core/src/worldcommit} "$out/src/world_commit"
          cp -R ${./crates/molten-core/src/world_head} "$out/src/world_head"
        '';
        verifiedNodeReplicationPilot = import ./nix/verified-node-replication-pilot.nix {
          inherit pkgs;
          octetPackages = octet-toolchain.packages.${system};
          octetRevision = "fc38f59330b626961d166febfdf1a5aa6575460f";
          profileDir = ./verification/verified-node-replication-pilot;
          workspaceSource = sourceForConfigChecks;
          savedEvidenceDir = ./.cairn/archive/2026-07-11-consume-octet-verus-toolchain/evidence;
        };

        moltenVmNodeModule =
          nodeId:
          { pkgs, ... }:
          {
            virtualisation.graphics = false;
            networking.hostName = nodeId;
            networking.firewall.enable = false;
            networking.extraHosts = ''
              192.168.1.1 node-a node_a
              192.168.1.2 node-b node_b
            '';
            environment.systemPackages = [
              moltenPkg
              pkgs.coreutils
              pkgs.gnugrep
              pkgs.iputils
            ];
            systemd.services.molten-node = {
              description = "Molten node VM integration service";
              wantedBy = [ "multi-user.target" ];
              after = [ "network-online.target" ];
              wants = [ "network-online.target" ];
              path = [
                moltenPkg
                pkgs.coreutils
                pkgs.gnugrep
              ];
              serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = true;
                StateDirectory = "molten";
                WorkingDirectory = "${sourceForConfigChecks}";
                ExecStop = "${moltenPkg}/bin/molten node stop --state-root /var/lib/molten --shutdown-out /var/lib/molten/vm-evidence/shutdown.preserves --receipt-out /var/lib/molten/vm-evidence/shutdown-control.preserves";
              };
              script = ''
                set -euo pipefail
                state=/var/lib/molten
                evidence="$state/vm-evidence"
                mkdir -p "$evidence"
                if [ ! -f "$state/config.preserves" ]; then
                  molten node init \
                    --state-root "$state" \
                    --node-id "node:${nodeId}" \
                    --config-out "$evidence/node-config.preserves" \
                    --identity-receipt-out "$evidence/identity.preserves" \
                    > "$evidence/init.txt"
                fi
                molten node run \
                  --state-root "$state" \
                  --startup-out "$evidence/startup.preserves" \
                  > "$evidence/run.txt"
                molten node status \
                  --state-root "$state" \
                  --health-out "$evidence/health.preserves" \
                  --receipt-out "$evidence/status.preserves" \
                  > "$evidence/status.txt"
                molten node run-loop \
                  --state-root "$state" \
                  --max-requests 1 \
                  --receipt-out "$evidence/control-loop.preserves" \
                  --heartbeat-out "$evidence/heartbeat.preserves" \
                  > "$evidence/run-loop.txt"
              '';
            };
            system.stateVersion = "24.11";
          };
      in
      {
        packages = {
          default = moltenPkg;
          molten = moltenPkg;
          molten-node-host = moltenNodeHostPkg;
          molten-release-policy = releasePolicyPkg;
          molten-kache = kacheWs.workspaceMembers."molten".build;
          molten-kache-rust = kacheWrappedRust;
          all = ws.allWorkspaceMembers;
        }
        // pkgs.lib.optionalAttrs (system == "x86_64-linux") {
          flux-profiler-cli = fluxProfilerCli;
          verified-node-replication-pilot = verifiedNodeReplicationPilot.check;
        };

        checks =
          let
            vmShardCheck =
              shardId:
              pkgs.runCommand "molten-${shardId}"
                {
                  nativeBuildInputs = [ moltenPkg ];
                }
                ''
                  set -euo pipefail
                  mkdir -p "$out"
                  make_ref() {
                    label=$1
                    file="$TMPDIR/$label.preserves"
                    printf '<vm-shard-fixture "%s">\n' "$label" > "$file"
                    summary=$(molten test nixos-vm show "$file")
                    ref=''${summary#* ref=}
                    ref=''${ref%% *}
                    printf '%s\n' "$ref"
                  }
                  scenario_ref=$(make_ref "scenario-${shardId}")
                  topology_ref=$(make_ref "topology-${shardId}")
                  package_ref=$(make_ref "package-${shardId}")
                  node_ref=$(make_ref "node-evidence-${shardId}")
                  child_ref=$(make_ref "child-${shardId}")
                  log_ref=$(make_ref "log-${shardId}")
                  molten test nixos-vm shard-run \
                    --shard-id ${shardId} \
                    --scenario-fixture-ref "$scenario_ref" \
                    --topology-ref "$topology_ref" \
                    --package-ref "$package_ref" \
                    --node-evidence-ref "$node_ref" \
                    --child-receipt-ref "$child_ref" \
                    --diagnostic-log-ref "$log_ref" \
                    --claimed-decision pass \
                    --caveat 'Nix shard check emits bounded fixture evidence only' \
                    --out "$out/shard.preserves" \
                    > "$out/shard.txt"
                  grep -q nixos-vm-shard-run-v1 "$out/shard.preserves"
                  grep -q 'decision "pass"' "$out/shard.preserves"
                '';
            capStdStoreAuthorityCheck =
              pkgs.runCommand "molten-cap-std-store-authority"
                {
                  nativeBuildInputs = [ pkgs.ast-grep ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  rule=tools/ast-grep/runtime-authority/rules/store-ambient-filesystem-call.yml
                  positive=tools/ast-grep/runtime-authority/fixtures/positive/store_ambient_filesystem_calls.rs
                  if ast-grep scan --rule "$rule" --json=compact "$positive" > "$TMPDIR/positive.json"; then
                    echo "blocking store fixture unexpectedly produced no findings" >&2
                    exit 1
                  fi
                  ast-grep scan --rule "$rule" --json=compact \
                    tools/ast-grep/runtime-authority/fixtures/negative/store_capability_shells.rs \
                    tools/ast-grep/runtime-authority/fixtures/negative/tests/adversarial_store_setup.rs \
                    > "$TMPDIR/negative.json"
                  ast-grep scan --rule "$rule" --json=compact \
                    src/artifacts/parts/mod \
                    src/chunk/parts/store \
                    src/retention/parts/mod \
                    src/remote/parts/dataspace \
                    src/iroh/parts/exchange \
                    > "$TMPDIR/converted.json"
                  touch "$out"
                '';
            capStdTestWorkspaceCheck =
              pkgs.runCommand "molten-cap-std-test-workspaces"
                {
                  nativeBuildInputs = [ pkgs.ast-grep ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  rule=tools/ast-grep/runtime-authority/rules/test-ambient-temp-workspace.yml
                  positive=tools/ast-grep/runtime-authority/fixtures/positive/test_ambient_temp_workspace.rs
                  if ast-grep scan --rule "$rule" --json=compact "$positive" > "$TMPDIR/positive.json"; then
                    echo "blocking test-workspace fixture unexpectedly produced no findings" >&2
                    exit 1
                  fi
                  ast-grep scan --rule "$rule" --json=compact \
                    tools/ast-grep/runtime-authority/fixtures/negative/test_capability_workspace.rs \
                    > "$TMPDIR/negative.json"
                  ast-grep scan --rule "$rule" --json=compact \
                    src/test/support.rs \
                    src/main/tests/ops/helpers.rs \
                    src/local_store_tests.rs \
                    src/chunk/parts/store/tests/m000/p002/body.rs \
                    src/retention/parts/mod/tests/m000/p000/body.rs \
                    src/remote/parts/dataspace/tests/m000/p001/body.rs \
                    src/iroh/parts/exchange/tests/m000/p001/body.rs \
                    src/evidence/parts/chain/tests/m000/p003/body.rs \
                    src/node/parts/daemon/tests/m000/p000/body.rs \
                    src/node/parts/daemon/tests/m000/p010/body.rs \
                    tests/parts/cliharness/p000/body.rs \
                    tests/parts/cliharness/p013/body.rs \
                    > "$TMPDIR/converted.json"
                  touch "$out"
                '';
            nodeStateAuthorityCheck =
              pkgs.runCommand "molten-node-state-authority"
                {
                  nativeBuildInputs = [ pkgs.ast-grep ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  ambient_rule=tools/ast-grep/runtime-authority/rules/node-state-ambient-filesystem-call.yml
                  reacquisition_rule=tools/ast-grep/runtime-authority/rules/node-state-root-reacquisition.yml
                  positive=tools/ast-grep/runtime-authority/fixtures/positive/node_state_ambient_authority.rs
                  if ast-grep scan --rule "$ambient_rule" --json=compact "$positive" > "$TMPDIR/ambient-positive.json"; then
                    echo "blocking node-state ambient fixture unexpectedly produced no findings" >&2
                    exit 1
                  fi
                  if ast-grep scan --rule "$reacquisition_rule" --json=compact "$positive" > "$TMPDIR/reacquisition-positive.json"; then
                    echo "blocking node-state reacquisition fixture unexpectedly produced no findings" >&2
                    exit 1
                  fi
                  ast-grep scan --rule "$ambient_rule" --json=compact \
                    tools/ast-grep/runtime-authority/fixtures/negative/node_state_capability_shell.rs \
                    > "$TMPDIR/ambient-negative.json"
                  ast-grep scan --rule "$reacquisition_rule" --json=compact \
                    tools/ast-grep/runtime-authority/fixtures/negative/node_state_carried_authority.rs \
                    > "$TMPDIR/reacquisition-negative.json"
                  ast-grep scan --rule "$ambient_rule" --json=compact \
                    src/node/parts/daemon \
                    src/node/parts/identity/p001/body.rs \
                    src/job/parts/dag/p009/body.rs \
                    src/job/parts/dag/p017/body.rs \
                    > "$TMPDIR/ambient-converted.json"
                  ast-grep scan --rule "$reacquisition_rule" --json=compact \
                    src/node/parts/daemon \
                    src/node/parts/identity/p001/body.rs \
                    src/job/parts/dag/p009/body.rs \
                    src/job/parts/dag/p017/body.rs \
                    > "$TMPDIR/reacquisition-converted.json"
                  touch "$out"
                '';
            materializationAuthorityCheck =
              pkgs.runCommand "molten-materialization-authority"
                {
                  nativeBuildInputs = [ pkgs.ast-grep ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  rule=tools/ast-grep/runtime-authority/rules/materialization-ambient-output.yml
                  positive=tools/ast-grep/runtime-authority/fixtures/positive/materialization_ambient_output.rs
                  if ast-grep scan --rule "$rule" --json=compact "$positive" > "$TMPDIR/positive.json"; then
                    echo "blocking materialization fixture unexpectedly produced no findings" >&2
                    exit 1
                  fi
                  ast-grep scan --rule "$rule" --json=compact \
                    tools/ast-grep/runtime-authority/fixtures/negative/materialization_capability_shell.rs \
                    > "$TMPDIR/negative.json"
                  ast-grep scan --rule "$rule" --json=compact \
                    src/cli/runtime/repro/bundle.rs \
                    src/cli/runtime/repro/bundle/unpack.rs \
                    src/retention/parts/mod/p028/body.rs \
                    src/cli/ops/dogfood/archive.rs \
                    src/cli/ops/dogfood/io.rs \
                    src/operator/parts/dogfood/p008/body.rs \
                    > "$TMPDIR/converted.json"
                  touch "$out"
                '';
            wasmComponentProfileCheck =
              pkgs.runCommand "molten-wasm-component-profile"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.diffutils
                    pkgs.b3sum
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  profile=docs/wasm-component-runtime/profile.ncl
                  generated=docs/wasm-component-runtime/generated/profile.json
                  nickel export "$profile" --format json > "$TMPDIR/profile.json"
                  diff -u "$generated" "$TMPDIR/profile.json"
                  expected_wit_hash=83be349bb27975ada30dbe60817c5404df7862babdf02ae229b399886e76d5e8
                  printf '%s  %s\n' "$expected_wit_hash" wit/molten-component-runtime/runtime.wit \
                    | b3sum --check
                  for fixture in \
                    docs/wasm-component-runtime/fixtures/negative/stale-cohort.ncl \
                    docs/wasm-component-runtime/fixtures/negative/incomplete-cohort.ncl \
                    docs/wasm-component-runtime/fixtures/negative/ambient-wasi.ncl \
                    docs/wasm-component-runtime/fixtures/negative/nondeterministic-growth.ncl
                  do
                    if nickel export "$fixture" --format json > "$TMPDIR/negative.json" 2> "$TMPDIR/negative.err"; then
                      echo "negative Wasm component profile fixture unexpectedly exported: $fixture" >&2
                      exit 1
                    fi
                  done
                  touch "$out"
                '';
            wasmComponentPerformanceProfileCheck =
              pkgs.runCommand "molten-wasm-component-performance-profile"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.diffutils
                    pkgs.gnugrep
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  profile=docs/wasm-component-performance/profile.ncl
                  generated=docs/wasm-component-performance/generated/profile.json
                  nickel export "$profile" --format json > "$TMPDIR/profile.json"
                  diff -u "$generated" "$TMPDIR/profile.json"
                  grep -q 'c18bbe75803a6a610f7ff3b15549c927c6e02667' "$generated"
                  for fixture in docs/wasm-component-performance/fixtures/negative/*.ncl
                  do
                    if nickel export "$fixture" --format json > "$TMPDIR/negative.json" 2> "$TMPDIR/negative.err"; then
                      echo "negative Wasm performance profile fixture unexpectedly exported: $fixture" >&2
                      exit 1
                    fi
                  done
                  if grep -R -E 'Command::new\([^)]*(wizer|precompile)' src/wasm/performance; then
                    echo "Wasm performance shell must not invoke Wizer or precompile tools" >&2
                    exit 1
                  fi
                  touch "$out"
                '';
            fabricMembershipPlacementProfileCheck =
              pkgs.runCommand "molten-fabric-membership-placement-profile"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.diffutils
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  profile=docs/fabric-membership-placement/profile.ncl
                  generated=docs/fabric-membership-placement/generated/profile.json
                  nickel export "$profile" --format json > "$TMPDIR/profile.json"
                  diff -u "$generated" "$TMPDIR/profile.json"
                  for fixture in docs/fabric-membership-placement/fixtures/negative/*.ncl
                  do
                    if nickel export "$fixture" --format json > "$TMPDIR/negative.json" 2> "$TMPDIR/negative.err"; then
                      echo "negative membership profile fixture unexpectedly exported: $fixture" >&2
                      exit 1
                    fi
                  done
                  touch "$out"
                '';
            fabricCryptographicIdentityProfileCheck =
              pkgs.runCommand "molten-fabric-cryptographic-identity-profile"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.diffutils
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  profile=docs/fabric-cryptographic-identity/profile.ncl
                  generated=docs/fabric-cryptographic-identity/generated/profile.json
                  nickel export "$profile" --format json > "$TMPDIR/profile.json"
                  diff -u "$generated" "$TMPDIR/profile.json"
                  for fixture in docs/fabric-cryptographic-identity/fixtures/negative/*.ncl
                  do
                    if nickel export "$fixture" --format json > "$TMPDIR/negative.json" 2> "$TMPDIR/negative.err"; then
                      echo "negative cryptographic identity profile fixture unexpectedly exported: $fixture" >&2
                      exit 1
                    fi
                  done
                  touch "$out"
                '';
            fabricObservabilityProfileCheck =
              pkgs.runCommand "molten-fabric-observability-profile"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.diffutils
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  profile=docs/fabric-observability/profile.ncl
                  generated=docs/fabric-observability/generated/profile.json
                  nickel export "$profile" --format json > "$TMPDIR/profile.json"
                  diff -u "$generated" "$TMPDIR/profile.json"
                  for fixture in docs/fabric-observability/fixtures/negative/*.ncl
                  do
                    if nickel export "$fixture" --format json > "$TMPDIR/negative.json" 2> "$TMPDIR/negative.err"; then
                      echo "negative observability profile fixture unexpectedly exported: $fixture" >&2
                      exit 1
                    fi
                  done
                  touch "$out"
                '';
            releaseDependencyProfileCheck =
              pkgs.runCommand "molten-release-dependency-profile"
                {
                  nativeBuildInputs = [
                    releasePolicyPkg
                    pkgs.nickel
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cp -R $src source
                  chmod -R u+w source
                  cd source
                  molten-release-policy \
                    --root . \
                    --evidence-source valence-integrity=${valence-src} \
                    --evidence-source octet-cutover=${octet-cutover-src} \
                    > "$out"
                  nickel export config/release-dependencies/fixtures/positive/exact-pins.ncl \
                    > "$TMPDIR/release-profile-positive.json"
                  for fixture in config/release-dependencies/fixtures/negative/*.ncl
                  do
                    if nickel export "$fixture" > "$TMPDIR/release-profile-negative.json" 2> "$TMPDIR/release-profile-negative.err"; then
                      echo "negative release dependency fixture unexpectedly exported: $fixture" >&2
                      exit 1
                    fi
                  done
                '';
            # r[impl molten.prod_release_profile.executable_gate]
            # r[verify molten.prod_release_profile.executable_gate]
            # r[impl molten.prod_release_profile.fixture_non_claim]
            # r[verify molten.prod_release_profile.fixture_non_claim]
            releaseProfileValidationCheck =
              pkgs.runCommand "molten-release-profile-validation"
                {
                  nativeBuildInputs = [ moltenPkg ];
                }
                ''
                  set -euo pipefail
                  fixture_ref=blake3:a043cb9fc4524bda0424a13e2ff02772cce5b0dd9692db4f8dc62b2b0d2e4274
                  placeholder_ref=blake3:0000000000000000000000000000000000000000000000000000000000000000
                  valence_policy_hash=8f5174292fe31f8fc364dc8f49560b21581f2cf01e54ae3fe8820c6d90d62f65
                  run_profile() {
                    local source_gate_ref="$1"
                    shift
                    molten test gate release-profile \
                      --profile-id release-profile-conformance-fixture \
                      --tier release \
                      --source-gate-ref "$source_gate_ref" \
                      --policy-ref "$fixture_ref" \
                      --octet-ref "$fixture_ref" \
                      --cairn-ref "$fixture_ref" \
                      --stack-provenance-ref "$fixture_ref" \
                      --production-profile-ref "$fixture_ref" \
                      --expected-generated-export-ref "$fixture_ref" \
                      --actual-generated-export-ref "$fixture_ref" \
                      --stack-provenance-required \
                      --accepted-valence-policy-hash "$valence_policy_hash" \
                      --caveat validator-wiring-fixture-only \
                      "$@"
                  }

                  run_profile "$fixture_ref" \
                    --candidate-ref "$fixture_ref" \
                    --out "$out"
                  grep -q release-profile-validation-v1 "$out"
                  grep -q 'candidate-ref' "$out"
                  grep -q pass "$out"

                  if run_profile "$fixture_ref" \
                    --out "$TMPDIR/missing-candidate.preserves"; then
                    echo "missing candidate release profile unexpectedly passed" >&2
                    exit 1
                  fi
                  grep -q missing-release-candidate-ref "$TMPDIR/missing-candidate.preserves"

                  if run_profile "$placeholder_ref" \
                    --candidate-ref "$fixture_ref" \
                    --out "$TMPDIR/placeholder-source.preserves"; then
                    echo "placeholder source release profile unexpectedly passed" >&2
                    exit 1
                  fi
                  grep -q placeholder-release-ref "$TMPDIR/placeholder-source.preserves"
                '';
            # r[impl molten.prod_release_candidate.evidence_source_binding]
            # r[verify molten.prod_release_candidate.evidence_source_binding]
            # r[impl molten.prod_release_candidate.evidence_binding_non_claim]
            releaseCandidateBindingCheck =
              pkgs.runCommand "molten-release-candidate-binding"
                {
                  nativeBuildInputs = [ moltenPkg ];
                }
                ''
                  set -euo pipefail
                  candidate_ref=blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6
                  artifact_ref=blake3:a043cb9fc4524bda0424a13e2ff02772cce5b0dd9692db4f8dc62b2b0d2e4274
                  other_candidate_ref=blake3:6c0498ca351e3817c49603bc14dede0ecc22c5df9718c8f0ffa71f31aba38203
                  binding="$artifact_ref@$candidate_ref"
                  run_candidate() {
                    local rust_binding="$1"
                    shift
                    molten test prod-soak release-candidate-gate \
                      --candidate release-candidate-binding-conformance-fixture \
                      --source-ref "$candidate_ref" \
                      --rust-validation-binding "$rust_binding" \
                      --nextest-binding "$binding" \
                      --nix-check-binding "$binding" \
                      --cairn-validation-binding "$binding" \
                      --octet-binding "$binding" \
                      --dogfood-binding "$binding" \
                      --bundle-verify-binding "$binding" \
                      --promotion-binding "$binding" \
                      --export-verify-binding "$binding" \
                      --pilot-decision-binding "$binding" \
                      "$@"
                  }

                  run_candidate "$binding" --out "$out"
                  grep -q prod-release-candidate-gate-v2 "$out"
                  grep -q candidate-evidence "$out"
                  grep -q "$candidate_ref" "$out"
                  grep -q all-evidence-candidate-bound "$out"

                  mismatch_binding="$artifact_ref@$other_candidate_ref"
                  if run_candidate "$mismatch_binding" \
                    --out "$TMPDIR/mismatch.preserves" \
                    > "$TMPDIR/mismatch.stdout" \
                    2> "$TMPDIR/mismatch.stderr"; then
                    echo "mixed-candidate release evidence unexpectedly passed" >&2
                    exit 1
                  fi
                  grep -q 'Rust validation candidate source mismatch' "$TMPDIR/mismatch.stderr"
                '';
            # r[verify molten.prod_release.pilot_candidate_freeze]
            # r[verify molten.prod_release.pilot_evidence_publication]
            releasePilotManifestCheck =
              pkgs.runCommand "molten-release-pilot-manifest"
                {
                  nativeBuildInputs = [ pkgs.nickel ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  nickel typecheck release/molten-0.1.0-pilot-contracts.ncl
                  nickel export release/molten-0.1.0-pilot.ncl > "$TMPDIR/manifest.json"
                  if nickel export release/fixtures/negative/mixed-source.ncl \
                    > "$TMPDIR/mixed-source.json" \
                    2> "$TMPDIR/mixed-source.stderr"; then
                    echo "mixed-source pilot manifest unexpectedly passed" >&2
                    exit 1
                  fi
                  grep -q 'contract broken by a value' "$TMPDIR/mixed-source.stderr"
                  mkdir -p "$out"
                  cp "$TMPDIR/manifest.json" "$out/manifest.json"
                '';
            contentStoreAdapterProfileCheck =
              pkgs.runCommand "molten-content-store-adapter-profile"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.diffutils
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  profile=docs/content-store-adapter/profile.ncl
                  generated=docs/content-store-adapter/generated/profile.json
                  nickel export "$profile" --format json > "$TMPDIR/profile.json"
                  diff -u "$generated" "$TMPDIR/profile.json"
                  for fixture in docs/content-store-adapter/fixtures/negative/*.ncl
                  do
                    if nickel export "$fixture" --format json > "$TMPDIR/negative.json" 2> "$TMPDIR/negative.err"; then
                      echo "negative content-store adapter fixture unexpectedly exported: $fixture" >&2
                      exit 1
                    fi
                  done
                  touch "$out"
                '';
            # r[verify molten.project.inherited_tracey_classification.verified_repair]
            # r[verify molten.project.runtime_spine_tracey.direct_repairs]
            # r[verify molten.project.runtime_spine_tracey.exact_manifest]
            # r[impl molten.project.runtime_spine_tracey.growth_denial]
            # r[verify molten.project.runtime_spine_tracey.non_claims]
            inheritedTraceyDebtCheck =
              pkgs.runCommand "molten-inherited-tracey-debt"
                {
                  nativeBuildInputs = [
                    rustToolchain
                    pkgs.nickel
                    pkgs.diffutils
                    pkgs.b3sum
                    pkgs.gnugrep
                    pkgs.jq
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  guard_source=tools/tracey/inherited_debt_guard.rs
                  classifier_source=tools/tracey/inherited_debt_classifier.rs
                  baseline=evidence/tracey/inherited-debt-baseline.txt
                  metadata=evidence/tracey/inherited-debt-baseline.ncl
                  generated=evidence/tracey/inherited-debt-baseline.json
                  classification=evidence/tracey/inherited-debt-classification.tsv
                  classification_summary=evidence/tracey/inherited-debt-classification.md
                  classification_metadata=evidence/tracey/inherited-debt-classification.ncl
                  classification_generated=evidence/tracey/inherited-debt-classification.json
                  runtime_spine_repairs=evidence/tracey/runtime-spine-direct-repairs.ncl
                  runtime_spine_repairs_generated=evidence/tracey/runtime-spine-direct-repairs.json
                  expected_runtime_spine_repair_count=14
                  # r[impl molten.project.runtime_spine_job_dag_tracey.direct_repairs]
                  # r[verify molten.project.runtime_spine_job_dag_tracey.direct_repairs]
                  # r[impl molten.project.runtime_spine_job_dag_tracey.exact_manifest]
                  # r[verify molten.project.runtime_spine_job_dag_tracey.exact_manifest]
                  # r[impl molten.project.runtime_spine_job_dag_tracey.growth_denial]
                  # r[verify molten.project.runtime_spine_job_dag_tracey.growth_denial]
                  # r[impl molten.project.runtime_spine_job_dag_tracey.non_claims]
                  # r[verify molten.project.runtime_spine_job_dag_tracey.non_claims]
                  runtime_spine_job_dag_repairs=evidence/tracey/runtime-spine-job-dag-direct-repairs.ncl
                  runtime_spine_job_dag_repairs_generated=evidence/tracey/runtime-spine-job-dag-direct-repairs.json
                  expected_runtime_spine_job_dag_repair_count=9
                  expected_runtime_spine_job_dag_rejected_count=3
                  # r[impl molten.project.runtime_spine_sam_tracey.direct_repairs]
                  # r[verify molten.project.runtime_spine_sam_tracey.direct_repairs]
                  # r[impl molten.project.runtime_spine_sam_tracey.exact_manifest]
                  # r[verify molten.project.runtime_spine_sam_tracey.exact_manifest]
                  # r[impl molten.project.runtime_spine_sam_tracey.growth_denial]
                  # r[verify molten.project.runtime_spine_sam_tracey.growth_denial]
                  # r[impl molten.project.runtime_spine_sam_tracey.non_claims]
                  # r[verify molten.project.runtime_spine_sam_tracey.non_claims]
                  runtime_spine_sam_repairs=evidence/tracey/runtime-spine-sam-direct-repairs.ncl
                  runtime_spine_sam_repairs_generated=evidence/tracey/runtime-spine-sam-direct-repairs.json
                  expected_runtime_spine_sam_repair_count=13
                  # r[impl molten.project.runtime_spine_content_refs_tracey.direct_repairs]
                  # r[verify molten.project.runtime_spine_content_refs_tracey.direct_repairs]
                  # r[impl molten.project.runtime_spine_content_refs_tracey.exact_manifest]
                  # r[verify molten.project.runtime_spine_content_refs_tracey.exact_manifest]
                  # r[impl molten.project.runtime_spine_content_refs_tracey.growth_denial]
                  # r[verify molten.project.runtime_spine_content_refs_tracey.growth_denial]
                  # r[impl molten.project.runtime_spine_content_refs_tracey.non_claims]
                  # r[verify molten.project.runtime_spine_content_refs_tracey.non_claims]
                  # r[impl molten.runtime_spine.canonical_content_refs.cleanup_tests]
                  # r[verify molten.runtime_spine.canonical_content_refs.cleanup_tests]
                  # r[verify molten.runtime_spine.canonical_content_refs.migration]
                  runtime_spine_content_refs_repairs=evidence/tracey/runtime-spine-content-refs-direct-repairs.ncl
                  runtime_spine_content_refs_repairs_generated=evidence/tracey/runtime-spine-content-refs-direct-repairs.json
                  expected_runtime_spine_content_refs_candidate_count=12
                  expected_runtime_spine_content_refs_repair_count=10
                  expected_runtime_spine_content_refs_rejected_count=2
                  # r[impl molten.project.runtime_spine_preserves_boundary_tracey.direct_repairs]
                  # r[verify molten.project.runtime_spine_preserves_boundary_tracey.direct_repairs]
                  # r[impl molten.project.runtime_spine_preserves_boundary_tracey.exact_manifest]
                  # r[verify molten.project.runtime_spine_preserves_boundary_tracey.exact_manifest]
                  # r[impl molten.project.runtime_spine_preserves_boundary_tracey.growth_denial]
                  # r[verify molten.project.runtime_spine_preserves_boundary_tracey.growth_denial]
                  # r[impl molten.project.runtime_spine_preserves_boundary_tracey.non_claims]
                  # r[verify molten.project.runtime_spine_preserves_boundary_tracey.non_claims]
                  runtime_spine_preserves_boundary_repairs=evidence/tracey/runtime-spine-preserves-boundary-direct-repairs.ncl
                  runtime_spine_preserves_boundary_repairs_generated=evidence/tracey/runtime-spine-preserves-boundary-direct-repairs.json
                  expected_runtime_spine_preserves_boundary_candidate_count=9

                  rustc --edition=2024 --test "$guard_source" -o "$TMPDIR/guard-tests"
                  "$TMPDIR/guard-tests"
                  rustc --edition=2024 "$guard_source" -o "$TMPDIR/guard"
                  "$TMPDIR/guard" --root . --baseline "$baseline"

                  rustc --edition=2024 --test "$classifier_source" -o "$TMPDIR/classifier-tests"
                  "$TMPDIR/classifier-tests"
                  rustc --edition=2024 "$classifier_source" -o "$TMPDIR/classifier"
                  "$TMPDIR/classifier" \
                    --root . \
                    --baseline "$baseline" \
                    --output "$TMPDIR/classification.tsv" \
                    --summary-output "$TMPDIR/classification.md"
                  diff -u "$classification" "$TMPDIR/classification.tsv"
                  diff -u "$classification_summary" "$TMPDIR/classification.md"

                  nickel typecheck "$metadata"
                  nickel export "$metadata" --format json > "$TMPDIR/baseline.json"
                  diff -u "$generated" "$TMPDIR/baseline.json"
                  baseline_blake3="$(b3sum "$baseline" | cut -d ' ' -f 1)"
                  grep -Fq "\"uncovered_list_blake3\": \"$baseline_blake3\"" "$generated"

                  nickel typecheck "$classification_metadata"
                  nickel export "$classification_metadata" --format json > "$TMPDIR/classification.json"
                  diff -u "$classification_generated" "$TMPDIR/classification.json"
                  classification_blake3="$(b3sum "$classification" | cut -d ' ' -f 1)"
                  grep -Fq "\"blake3\": \"$classification_blake3\"" "$classification_generated"
                  classification_summary_blake3="$(b3sum "$classification_summary" | cut -d ' ' -f 1)"
                  grep -Fq "\"summary_blake3\": \"$classification_summary_blake3\"" "$classification_generated"

                  nickel typecheck "$runtime_spine_repairs"
                  nickel export "$runtime_spine_repairs" --format json > "$TMPDIR/runtime-spine-repairs.json"
                  diff -u "$runtime_spine_repairs_generated" "$TMPDIR/runtime-spine-repairs.json"
                  repair_count="$(jq '.repairs | length' "$runtime_spine_repairs_generated")"
                  test "$repair_count" -eq "$expected_runtime_spine_repair_count"
                  unique_repair_count="$(jq -r '.repairs[].requirement_id' "$runtime_spine_repairs_generated" | sort -u | wc -l)"
                  test "$unique_repair_count" -eq "$expected_runtime_spine_repair_count"
                  jq -r '.repairs[] | [.requirement_id, .implementation_path, .verification_path] | @tsv' \
                    "$runtime_spine_repairs_generated" \
                    | while IFS="$(printf '\t')" read -r requirement_id implementation_path verification_path
                  do
                    if grep -Fxq "$requirement_id" "$baseline"; then
                      echo "runtime-spine repair remains in inherited debt baseline: $requirement_id" >&2
                      exit 1
                    fi
                    grep -Fq "r[impl $requirement_id]" "$implementation_path"
                    grep -Fq "r[verify $requirement_id]" "$verification_path"
                  done

                  nickel typecheck "$runtime_spine_job_dag_repairs"
                  nickel export "$runtime_spine_job_dag_repairs" --format json > "$TMPDIR/runtime-spine-job-dag-repairs.json"
                  diff -u "$runtime_spine_job_dag_repairs_generated" "$TMPDIR/runtime-spine-job-dag-repairs.json"
                  job_dag_repair_count="$(jq '.repairs | length' "$runtime_spine_job_dag_repairs_generated")"
                  test "$job_dag_repair_count" -eq "$expected_runtime_spine_job_dag_repair_count"
                  unique_job_dag_repair_count="$(jq -r '.repairs[].requirement_id' "$runtime_spine_job_dag_repairs_generated" | sort -u | wc -l)"
                  test "$unique_job_dag_repair_count" -eq "$expected_runtime_spine_job_dag_repair_count"
                  rejected_job_dag_count="$(jq '.rejected_candidates | length' "$runtime_spine_job_dag_repairs_generated")"
                  test "$rejected_job_dag_count" -eq "$expected_runtime_spine_job_dag_rejected_count"
                  unique_rejected_job_dag_count="$(jq -r '.rejected_candidates[]' "$runtime_spine_job_dag_repairs_generated" | sort -u | wc -l)"
                  test "$unique_rejected_job_dag_count" -eq "$expected_runtime_spine_job_dag_rejected_count"
                  jq -r '.repairs[] | [.requirement_id, .implementation_path, .verification_path] | @tsv' \
                    "$runtime_spine_job_dag_repairs_generated" \
                    | while IFS="$(printf '\t')" read -r requirement_id implementation_path verification_path
                  do
                    if grep -Fxq "$requirement_id" "$baseline"; then
                      echo "runtime-spine job-DAG repair remains in inherited debt baseline: $requirement_id" >&2
                      exit 1
                    fi
                    grep -Fq "r[impl $requirement_id]" "$implementation_path"
                    grep -Fq "r[verify $requirement_id]" "$verification_path"
                  done
                  jq -r '.rejected_candidates[]' "$runtime_spine_job_dag_repairs_generated" \
                    | while IFS= read -r requirement_id
                  do
                    if ! grep -Fxq "$requirement_id" "$baseline"; then
                      echo "rejected runtime-spine job-DAG candidate left inherited debt: $requirement_id" >&2
                      exit 1
                    fi
                  done

                  nickel typecheck "$runtime_spine_sam_repairs"
                  nickel export "$runtime_spine_sam_repairs" --format json > "$TMPDIR/runtime-spine-sam-repairs.json"
                  diff -u "$runtime_spine_sam_repairs_generated" "$TMPDIR/runtime-spine-sam-repairs.json"
                  sam_repair_count="$(jq '.repairs | length' "$runtime_spine_sam_repairs_generated")"
                  test "$sam_repair_count" -eq "$expected_runtime_spine_sam_repair_count"
                  unique_sam_repair_count="$(jq -r '.repairs[].requirement_id' "$runtime_spine_sam_repairs_generated" | sort -u | wc -l)"
                  test "$unique_sam_repair_count" -eq "$expected_runtime_spine_sam_repair_count"
                  candidate_sam_count="$(jq '.candidate_count' "$runtime_spine_sam_repairs_generated")"
                  test "$candidate_sam_count" -eq "$expected_runtime_spine_sam_repair_count"
                  rejected_sam_count="$(jq '.rejected_candidates | length' "$runtime_spine_sam_repairs_generated")"
                  test "$rejected_sam_count" -eq 0
                  jq -r '.repairs[] | [.requirement_id, .implementation_path, .verification_path] | @tsv' \
                    "$runtime_spine_sam_repairs_generated" \
                    | while IFS="$(printf '\t')" read -r requirement_id implementation_path verification_path
                  do
                    if grep -Fxq "$requirement_id" "$baseline"; then
                      echo "runtime-spine SAM repair remains in inherited debt baseline: $requirement_id" >&2
                      exit 1
                    fi
                    grep -Fq "r[impl $requirement_id]" "$implementation_path"
                    grep -Fq "r[verify $requirement_id]" "$verification_path"
                  done

                  nickel typecheck "$runtime_spine_content_refs_repairs"
                  nickel export "$runtime_spine_content_refs_repairs" --format json \
                    > "$TMPDIR/runtime-spine-content-refs-repairs.json"
                  diff -u \
                    "$runtime_spine_content_refs_repairs_generated" \
                    "$TMPDIR/runtime-spine-content-refs-repairs.json"
                  content_refs_candidate_count="$(jq '.candidate_count' "$runtime_spine_content_refs_repairs_generated")"
                  test "$content_refs_candidate_count" -eq "$expected_runtime_spine_content_refs_candidate_count"
                  content_refs_repair_count="$(jq '.repairs | length' "$runtime_spine_content_refs_repairs_generated")"
                  test "$content_refs_repair_count" -eq "$expected_runtime_spine_content_refs_repair_count"
                  unique_content_refs_repair_count="$(jq -r '.repairs[].requirement_id' \
                    "$runtime_spine_content_refs_repairs_generated" | sort -u | wc -l)"
                  test "$unique_content_refs_repair_count" -eq "$expected_runtime_spine_content_refs_repair_count"
                  content_refs_rejected_count="$(jq '.rejected_candidates | length' \
                    "$runtime_spine_content_refs_repairs_generated")"
                  test "$content_refs_rejected_count" -eq "$expected_runtime_spine_content_refs_rejected_count"
                  unique_content_refs_rejected_count="$(jq -r '.rejected_candidates[].requirement_id' \
                    "$runtime_spine_content_refs_repairs_generated" | sort -u | wc -l)"
                  test "$unique_content_refs_rejected_count" -eq "$expected_runtime_spine_content_refs_rejected_count"
                  jq -r '.repairs[] | [.requirement_id, .implementation_path, .verification_path] | @tsv' \
                    "$runtime_spine_content_refs_repairs_generated" \
                    | while IFS="$(printf '\t')" read -r requirement_id implementation_path verification_path
                  do
                    if grep -Fxq "$requirement_id" "$baseline"; then
                      echo "runtime-spine content-ref repair remains in inherited debt baseline: $requirement_id" >&2
                      exit 1
                    fi
                    grep -Fq "r[impl $requirement_id]" "$implementation_path"
                    grep -Fq "r[verify $requirement_id]" "$verification_path"
                  done
                  jq -r '.rejected_candidates[] | [.requirement_id, .counterexample_path] | @tsv' \
                    "$runtime_spine_content_refs_repairs_generated" \
                    | while IFS="$(printf '\t')" read -r requirement_id counterexample_path
                  do
                    if ! grep -Fxq "$requirement_id" "$baseline"; then
                      echo "rejected runtime-spine content-ref candidate left inherited debt: $requirement_id" >&2
                      exit 1
                    fi
                    test -f "$counterexample_path"
                  done

                  nickel typecheck "$runtime_spine_preserves_boundary_repairs"
                  nickel export "$runtime_spine_preserves_boundary_repairs" --format json \
                    > "$TMPDIR/runtime-spine-preserves-boundary-repairs.json"
                  diff -u \
                    "$runtime_spine_preserves_boundary_repairs_generated" \
                    "$TMPDIR/runtime-spine-preserves-boundary-repairs.json"
                  preserves_boundary_candidate_count="$(jq '.candidate_count' \
                    "$runtime_spine_preserves_boundary_repairs_generated")"
                  test "$preserves_boundary_candidate_count" -eq \
                    "$expected_runtime_spine_preserves_boundary_candidate_count"
                  preserves_boundary_repair_count="$(jq '.repairs | length' \
                    "$runtime_spine_preserves_boundary_repairs_generated")"
                  test "$preserves_boundary_repair_count" -eq \
                    "$expected_runtime_spine_preserves_boundary_candidate_count"
                  unique_preserves_boundary_repair_count="$(jq -r '.repairs[].requirement_id' \
                    "$runtime_spine_preserves_boundary_repairs_generated" | sort -u | wc -l)"
                  test "$unique_preserves_boundary_repair_count" -eq \
                    "$expected_runtime_spine_preserves_boundary_candidate_count"
                  preserves_boundary_rejected_count="$(jq '.rejected_candidates | length' \
                    "$runtime_spine_preserves_boundary_repairs_generated")"
                  test "$preserves_boundary_rejected_count" -eq 0
                  jq -r '.repairs[] | [.requirement_id, .implementation_path, .verification_path] | @tsv' \
                    "$runtime_spine_preserves_boundary_repairs_generated" \
                    | while IFS="$(printf '\t')" read -r requirement_id implementation_path verification_path
                  do
                    if grep -Fxq "$requirement_id" "$baseline"; then
                      echo "runtime-spine Preserves boundary repair remains in inherited debt baseline: $requirement_id" >&2
                      exit 1
                    fi
                    grep -Fq "r[impl $requirement_id]" "$implementation_path"
                    grep -Fq "r[verify $requirement_id]" "$verification_path"
                  done

                  for repaired in \
                    molten.choreography.chorus_design_reference \
                    molten.evidence.valence_stack_adapter.docs \
                    molten.testing.receipt_driven_traceability.coverage_derivation
                  do
                    if grep -Fxq "$repaired" "$baseline"; then
                      echo "verified repair remains in inherited debt baseline: $repaired" >&2
                      exit 1
                    fi
                  done
                  grep -Fq 'r[impl molten.choreography.chorus_design_reference]' src/protocol/parts/session/p009/body.rs
                  grep -Fq 'r[verify molten.choreography.chorus_design_reference]' src/protocol/parts/session/tests/m000/p002/body.rs
                  grep -Fq 'r[impl molten.evidence.valence_stack_adapter.docs]' crates/molten-core/src/stack.rs
                  grep -Fq 'r[verify molten.evidence.valence_stack_adapter.docs]' crates/molten-core/src/stack.rs
                  grep -Fq 'r[impl molten.testing.receipt_driven_traceability.coverage_derivation]' src/testing/traceability/parts/p002/body.rs
                  grep -Fq 'r[verify molten.testing.receipt_driven_traceability.coverage_derivation]' src/testing/traceability/parts/tests/p001/body.rs
                  touch "$out"
                '';
          in
          rec {
            # The hermetic nextest check supplies binary metadata for CLI tests
            # using CARGO_BIN_EXE_molten; the raw unit2nix libtest runner does not.
            molten = nextest;
            molten-node-host = moltenNodeHostTests;
            clippy = ws.clippy.allWorkspaceMembers;
            executable-extent-consumer =
              assert executableExtentSource != null;
              assert mantleExecutableExtentSource != null;
              pkgs.runCommand "molten-executable-extent-consumer"
                {
                  src = ./.;
                  nativeBuildInputs = [
                    pkgs.jq
                    pkgs.nickel
                    pkgs.ripgrep
                  ];
                }
                ''
                  set -eu
                  cd "$src"

                  cmp \
                    tests/fixtures/executable-extent/executable-extent-bundle.valid.json \
                    ${mantleExecutableExtentSource}/schemas/machine-contracts/fixtures/executable-extent-bundle.valid.json
                  cmp \
                    tests/fixtures/executable-extent/executable-extent-producer-receipt.valid.json \
                    ${mantleExecutableExtentSource}/schemas/machine-contracts/fixtures/executable-extent-producer-receipt.valid.json

                  rg -Fq '${executableExtentRepository}' Cargo.toml crates/molten-core/Cargo.toml flake.nix
                  rg -Fq '${executableExtentRevision}' Cargo.toml crates/molten-core/Cargo.toml Cargo.lock flake.nix
                  rg -Fq '${mantleExecutableExtentRevision}' src/executable_extent/mod.rs flake.nix
                  rg -Fq '${executableExtentRepository}' checks/executable-extent-octet/Cargo.toml
                  rg -Fq '${executableExtentRevision}' checks/executable-extent-octet/Cargo.toml
                  rg -Fq '${executableExtentOctetRevision}' flake.nix flake.lock
                  if rg -n '/home/|\.\./\.\./executable-extent' \
                    checks/executable-extent-octet; then
                    echo 'focused Octet source contains an ambient sibling path' >&2
                    exit 1
                  fi
                  if rg -n 'executable-extent-(core|conformance|linux)[[:space:]]*=[[:space:]]*\{[[:space:]]*path' \
                    Cargo.toml crates/molten-core/Cargo.toml; then
                    echo 'executable-extent sibling path is not admitted' >&2
                    exit 1
                  fi

                  nickel format --check \
                    schemas/executable-extent/consumer-receipt-contract.ncl \
                    schemas/executable-extent/fixtures/*.ncl
                  nickel typecheck schemas/executable-extent/fixtures/consumer-receipt.valid.ncl
                  nickel export --format json \
                    schemas/executable-extent/fixtures/consumer-receipt.valid.ncl \
                    > "$TMPDIR/consumer-receipt.json"
                  jq -S . \
                    tests/fixtures/executable-extent/molten-executable-extent-consumer-receipt.valid.json \
                    > "$TMPDIR/checked-consumer-receipt.json"
                  jq -S . "$TMPDIR/consumer-receipt.json" \
                    > "$TMPDIR/exported-consumer-receipt.json"
                  diff -u \
                    "$TMPDIR/checked-consumer-receipt.json" \
                    "$TMPDIR/exported-consumer-receipt.json"
                  for fixture in schemas/executable-extent/fixtures/*.invalid.ncl; do
                    if nickel export --format json "$fixture" \
                      > "$TMPDIR/invalid.json" 2> "$TMPDIR/invalid.stderr"; then
                      echo "invalid executable-extent fixture passed: $fixture" >&2
                      exit 1
                    fi
                  done
                  jq -e '.additionalProperties == false' \
                    schemas/executable-extent/molten-executable-extent-consumer-receipt-v1.schema.json \
                    > /dev/null
                  touch "$out"
                '';
            executable-extent-octet-deny-all =
              assert executableExtentOctetAdmitted;
              (executable-extent-octet.lib.mkConsumerCheck {
                inherit system;
                src = executableExtentOctetWorkspace;
                packages = [ "molten-executable-extent-octet" ];
                cargoExtraArgs = "--all-targets --all-features";
                cargoLock = ./checks/executable-extent-octet/Cargo.lock;
              }).overrideAttrs
                (_previous: {
                  DYLINT_RUSTFLAGS = "--deny warnings";
                });
            world-commit-octet-deny-all =
              assert executableExtentOctetAdmitted;
              (executable-extent-octet.lib.mkConsumerCheck {
                inherit system;
                src = worldCommitOctetWorkspace;
                packages = [ "molten-world-commit-octet" ];
                cargoExtraArgs = "--all-targets --all-features";
                cargoLock = ./checks/world-commit-octet/Cargo.lock;
              }).overrideAttrs
                (_previous: {
                  DYLINT_RUSTFLAGS = "--deny warnings";
                });
            world-head-octet-deny-all =
              assert executableExtentOctetAdmitted;
              (executable-extent-octet.lib.mkConsumerCheck {
                inherit system;
                src = worldHeadOctetWorkspace;
                packages = [ "molten-world-head-octet" ];
                cargoExtraArgs = "--all-targets --all-features";
                cargoLock = ./checks/world-head-octet/Cargo.lock;
              }).overrideAttrs
                (_previous: {
                  DYLINT_RUSTFLAGS = "--deny warnings";
                });
            world-head-dependency-identity = pkgs.runCommand "molten-world-head-dependency-identity" { } ''
              test -f ${choregraphSource}/crates/choregraph-history/src/refs.rs
              test -f ${artifactSource}/crates/artifact-auth-core/src/lib.rs
              test -f ${artifactSource}/crates/artifact-auth-ed25519/src/lib.rs
              touch "$out"
            '';
            cap-std-store-authority = capStdStoreAuthorityCheck;
            cap-std-test-workspaces = capStdTestWorkspaceCheck;
            node-state-authority = nodeStateAuthorityCheck;
            materialization-authority = materializationAuthorityCheck;
            wasm-component-profile = wasmComponentProfileCheck;
            wasm-component-performance-profile = wasmComponentPerformanceProfileCheck;
            fabric-membership-placement-profile = fabricMembershipPlacementProfileCheck;
            fabric-cryptographic-identity-profile = fabricCryptographicIdentityProfileCheck;
            fabric-observability-profile = fabricObservabilityProfileCheck;
            content-store-adapter-profile = contentStoreAdapterProfileCheck;
            inherited-tracey-debt = inheritedTraceyDebtCheck;
            release-dependency-profile = releaseDependencyProfileCheck;
            release-profile-validation = releaseProfileValidationCheck;
            release-candidate-binding = releaseCandidateBindingCheck;
            release-pilot-manifest = releasePilotManifestCheck;

            # r[verify molten.artifact_auth_adoption.source]
            # r[verify molten.artifact_auth_adoption.radicle_transport]
            # r[verify molten.artifact_auth_adoption.radicle_agreement]
            # r[verify molten.artifact_auth_adoption.radicle_behavior]
            # r[verify molten.artifact_auth_adoption.radicle_fallback]
            # r[verify molten.artifact_auth_adoption.radicle_evidence]
            artifact-auth-radicle-cutover =
              pkgs.runCommand "molten-artifact-auth-radicle-cutover"
                {
                  nativeBuildInputs = [
                    pkgs.b3sum
                    pkgs.jq
                    pkgs.nickel
                    pkgs.ripgrep
                  ];
                  src = ./.;
                }
                ''
                  set -euo pipefail
                  cd "$src"

                  nickel typecheck evidence/radicle/artifact-auth-cutover-v1.ncl
                  nickel typecheck lib/artifact-auth-cutover-receipt.ncl
                  nickel export --format json tests/artifact-auth-cutover.ncl > "$TMPDIR/tests.json"
                  grep -Fq '"tests": true' "$TMPDIR/tests.json"

                  nickel export --format json evidence/radicle/artifact-auth-cutover-v1.ncl > "$TMPDIR/cutover.json"
                  jq --sort-keys . "$TMPDIR/cutover.json" > "$TMPDIR/cutover.normalized.json"
                  jq --sort-keys . evidence/radicle/artifact-auth-cutover-v1.json > "$TMPDIR/evidence.normalized.json"
                  diff --unified "$TMPDIR/cutover.normalized.json" "$TMPDIR/evidence.normalized.json"

                  receipt_hash="$(b3sum evidence/radicle/artifact-auth-cutover-v1.json | cut -d ' ' -f 1)"
                  expected_receipt_hash="$(tr -d '\n' < evidence/radicle/artifact-auth-cutover-v1.blake3)"
                  test "$receipt_hash" = "$expected_receipt_hash"

                  nickel typecheck evidence/source/artifact-workspace-migration-v1.ncl
                  nickel typecheck lib/artifact-source-migration-receipt.ncl
                  nickel export --format json tests/artifact-source-migration.ncl > "$TMPDIR/migration-tests.json"
                  grep -Fq '"tests": true' "$TMPDIR/migration-tests.json"
                  nickel export --format json evidence/source/artifact-workspace-migration-v1.ncl > "$TMPDIR/migration.json"
                  jq --sort-keys . "$TMPDIR/migration.json" > "$TMPDIR/migration.normalized.json"
                  jq --sort-keys . evidence/source/artifact-workspace-migration-v1.json > "$TMPDIR/migration-evidence.normalized.json"
                  diff --unified "$TMPDIR/migration.normalized.json" "$TMPDIR/migration-evidence.normalized.json"
                  migration_hash="$(b3sum evidence/source/artifact-workspace-migration-v1.json | cut -d ' ' -f 1)"
                  expected_migration_hash="$(tr -d '\n' < evidence/source/artifact-workspace-migration-v1.blake3)"
                  test "$migration_hash" = "$expected_migration_hash"

                  source_url='ssh://git@github.com/OnixResearch/onix-artifact.git'
                  source_rev='c932138d880ddf4c2967f4c024b489b5c0022bf1'
                  source_nar_hash='sha256-XGQLG60DNeY9FUYcOmn6cfYnhCIJzyqf+VW9yofDYFU='
                  jq -e \
                    --arg url "$source_url" \
                    --arg rev "$source_rev" \
                    --arg nar_hash "$source_nar_hash" \
                    '.nodes["artifact-src"] as $source
                     | $source.locked.url == $url
                     and $source.original.url == $url
                     and $source.locked.rev == $rev
                     and $source.original.rev == $rev
                     and $source.locked.narHash == $nar_hash' \
                    flake.lock >/dev/null

                  nickel export --format json config/release-dependencies/profile.ncl > "$TMPDIR/release-profile.json"
                  expected_profile_rows=3
                  test "$(jq --arg url "$source_url" --arg rev "$source_rev" \
                    '[.dependencies[] | select(.package_name == "artifact-auth-core" or .package_name == "artifact-auth-ed25519" or .package_name == "artifact-binding-core") | select(.source_coordinate == $url and .immutable_revision == $rev and .nix_input == "artifact-src" and .transport_policy == "ssh-pinned-nix-archive")] | length' \
                    "$TMPDIR/release-profile.json")" = "$expected_profile_rows"

                  expected_default_packages=3
                  expected_release_packages=2
                  for plan_binding in \
                    "build-plan.json:$expected_default_packages" \
                    "release-policy-build-plan.json:$expected_release_packages"; do
                    plan="''${plan_binding%%:*}"
                    expected_count="''${plan_binding#*:}"
                    actual_count="$(jq --arg url "$source_url" --arg rev "$source_rev" \
                      '[.crates[] | select((.crateName == "artifact-auth-core" or .crateName == "artifact-auth-ed25519" or .crateName == "artifact-binding-core") and .source.url == $url and .source.rev == $rev)] | length' \
                      "$plan")"
                    test "$actual_count" = "$expected_count"
                  done

                  radicle_host='git.onix.computer'
                  radicle_rid='z4JGYYW7WsesXUq7MXVdx16Fawu2f'
                  github_host='github.com'
                  for forbidden_source in \
                    "$radicle_host/$radicle_rid" \
                    "$github_host/OnixResearch/artifact-auth"; do
                    if rg -F "$forbidden_source" \
                      Cargo.toml crates/molten-core/Cargo.toml Cargo.lock \
                      flake.nix flake.lock config/release-dependencies/profile.ncl; then
                      echo "executable predecessor source remains: $forbidden_source" >&2
                      exit 1
                    fi
                    for plan in build-plan.json release-policy-build-plan.json; do
                      if jq -e --arg forbidden "$forbidden_source" \
                        '[.crates | to_entries[] | select((.key | contains($forbidden)) or ((.value.source.url // "") | contains($forbidden)))] | length > 0' \
                        "$plan" >/dev/null; then
                        echo "generated plan retains predecessor source: $plan" >&2
                        exit 1
                      fi
                    done
                  done

                  touch "$out"
                '';

            nixos-vm-smoke = vmShardCheck "nixos-vm-smoke";
            nixos-vm-live-control = vmShardCheck "nixos-vm-live-control";
            nixos-vm-service-job = vmShardCheck "nixos-vm-service-job";
            nixos-vm-restart = vmShardCheck "nixos-vm-restart";
            nixos-vm-fault = vmShardCheck "nixos-vm-fault";
            nixos-vm-aggregate =
              pkgs.runCommand "molten-nixos-vm-aggregate"
                {
                  nativeBuildInputs = [ moltenPkg ];
                }
                ''
                  set -euo pipefail
                  mkdir -p "$out"
                  shard_ref() {
                    summary=$(molten test nixos-vm show "$1/shard.preserves")
                    ref=''${summary#* ref=}
                    ref=''${ref%% *}
                    printf '%s\n' "$ref"
                  }
                  make_ref() {
                    label=$1
                    file="$TMPDIR/$label.preserves"
                    printf '<vm-aggregate-fixture "%s">\n' "$label" > "$file"
                    summary=$(molten test nixos-vm show "$file")
                    ref=''${summary#* ref=}
                    ref=''${ref%% *}
                    printf '%s\n' "$ref"
                  }
                  topology_ref=$(make_ref topology-aggregate)
                  package_ref=$(make_ref package-aggregate)
                  manifest_ref=$(make_ref manifest-aggregate)
                  smoke_ref=$(shard_ref ${nixos-vm-smoke})
                  live_ref=$(shard_ref ${nixos-vm-live-control})
                  service_ref=$(shard_ref ${nixos-vm-service-job})
                  restart_ref=$(shard_ref ${nixos-vm-restart})
                  fault_ref=$(shard_ref ${nixos-vm-fault})
                  molten test nixos-vm aggregate \
                    --topology-ref "$topology_ref" \
                    --package-ref "$package_ref" \
                    --manifest-ref "$manifest_ref" \
                    --required-shard-id nixos-vm-smoke \
                    --required-shard-id nixos-vm-live-control \
                    --required-shard-id nixos-vm-service-job \
                    --required-shard-id nixos-vm-restart \
                    --required-shard-id nixos-vm-fault \
                    --shard-ref "$smoke_ref" \
                    --shard-ref "$live_ref" \
                    --shard-ref "$service_ref" \
                    --shard-ref "$restart_ref" \
                    --shard-ref "$fault_ref" \
                    --caveat 'aggregate check indexes child shard fixture evidence only' \
                    --out "$out/aggregate.preserves" \
                    > "$out/aggregate.txt"
                  grep -q nixos-vm-multinode-aggregate-v1 "$out/aggregate.preserves"
                  grep -q 'decision "pass"' "$out/aggregate.preserves"
                '';

            deterministic-drift-gate =
              pkgs.runCommand "molten-deterministic-drift-gate"
                {
                  nativeBuildInputs = [ moltenPkg ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  mkdir -p "$out"
                  cp -R $src source
                  chmod -R u+w source
                  cd source

                  printf '%s\n' '<release-evidence-fixture "stable">' > left.preserves
                  cp left.preserves right.preserves
                  molten test drift compare \
                    --workflow release-evidence \
                    --left left.preserves \
                    --right right.preserves \
                    --out "$out/drift-pass.preserves" \
                    > "$out/drift-pass.txt"

                  molten test drift rerun \
                    --workflow nixos-vm-topology \
                    --left-root "$TMPDIR/left-root" \
                    --right-root "$TMPDIR/right-root" \
                    --command molten \
                    --arg test \
                    --arg nixos-vm \
                    --arg topology \
                    --arg=--node \
                    --arg node_a \
                    --arg=--node \
                    --arg node_b \
                    --arg=--package-ref \
                    --arg blake3:2ded4d8475648207836b950368aa4e1037b11b9aeb6f5b939482ad4d859664f7 \
                    --arg=--package-path \
                    --arg /nix/store/example-molten \
                    --arg=--network \
                    --arg nixos-test-private \
                    --arg=--caveat \
                    --arg 'topology rerun fixture is deterministic evidence only' \
                    --arg=--out \
                    --arg '{root}/topology.preserves' \
                    --artifact topology.preserves \
                    --out "$out/topology-drift-rerun.preserves" \
                    > "$out/topology-drift-rerun.txt"

                  printf '%s\n' '<release-evidence-fixture "drifted">' > drifted.preserves
                  if molten test drift compare \
                    --workflow release-evidence \
                    --left left.preserves \
                    --right drifted.preserves \
                    --out "$out/drift-deny.preserves" \
                    > "$out/drift-deny.txt" 2> "$out/drift-deny.err"; then
                    echo "negative drift fixture unexpectedly passed" >&2
                    exit 1
                  fi
                '';

            requirement-traceability-gate =
              pkgs.runCommand "molten-requirement-traceability-gate"
                {
                  nativeBuildInputs = [ moltenPkg ];
                }
                ''
                  set -euo pipefail
                  mkdir -p "$out" fixture/cairn/changes/traceability/specs/testing-harness fixture/tests
                  {
                    printf '%s\n' '## ADDED Requirements'
                    printf '\n'
                    printf '%s\n' '### Requirement: Traceability fixture'
                    printf '%s\n' 'r[molten.testing.traceability.fixture] Molten MUST bind positive and negative coverage in this fixture.'
                  } > fixture/cairn/changes/traceability/specs/testing-harness/spec.md
                  touch fixture/tests/coverage.rs
                  artifact_ref=blake3:8f5174292fe31f8fc364dc8f49560b21581f2cf01e54ae3fe8820c6d90d62f65
                  positive='molten.testing.traceability.fixture|positive|tests/coverage.rs|cargo test coverage|'
                  negative='molten.testing.traceability.fixture|negative|tests/coverage.rs|cargo test coverage|'
                  molten test traceability scan \
                    --root fixture \
                    --changed-only \
                    --coverage "$positive$artifact_ref" \
                    --coverage "$negative$artifact_ref" \
                    --out "$out/traceability-pass.preserves" \
                    --summary-out "$out/traceability-pass.txt"

                  if molten test traceability scan \
                    --root fixture \
                    --changed-only \
                    --coverage "$positive$artifact_ref" \
                    --out "$out/traceability-deny.preserves" \
                    --summary-out "$out/traceability-deny.txt" \
                    > "$out/traceability-deny.stdout" 2> "$out/traceability-deny.stderr"; then
                    echo "negative traceability fixture unexpectedly passed" >&2
                    exit 1
                  fi
                '';

            nominal-reference-domains =
              pkgs.runCommand "molten-nominal-reference-domains"
                {
                  nativeBuildInputs = [
                    pkgs.jq
                    pkgs.nickel
                    pkgs.ripgrep
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  profile=config/nominal-reference-domains.ncl
                  core=crates/molten-core/src/nominal.rs
                  expected_domains=15

                  nickel typecheck "$profile"
                  nickel export --format json "$profile" > "$TMPDIR/profile.json"
                  test "$(jq '.domains | length' "$TMPDIR/profile.json")" = "$expected_domains"
                  jq -e '.canonical_algorithm == "blake3" and .wire_authority == "preserves"' "$TMPDIR/profile.json"
                  jq -e '.constructor_non_claims | index("typed-reference-is-not-authority") != null' "$TMPDIR/profile.json"

                  rg -q 'SessionDomain, SessionRef' "$core"
                  rg -q 'AuthorityContextDomain, AuthorityContextRef' "$core"
                  rg -q 'ArtifactDomain, ArtifactRef' "$core"
                  rg -q 'ReceiptDomain, ReceiptRef' "$core"
                  rg -q '```compile_fail' "$core"
                  if rg -n 'pub [a-zA-Z_][a-zA-Z0-9_]*: String' "$core"; then
                    echo "migrated nominal core exposes a raw string field" >&2
                    exit 1
                  fi
                  touch "$out"
                '';

            fabric-port-boundaries =
              pkgs.runCommand "molten-fabric-port-boundaries"
                {
                  nativeBuildInputs = [ pkgs.ripgrep ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  adapters=(
                    src/fabric_membership/adapters.rs
                    src/fabric_time/adapters.rs
                    src/fabric_transport/adapters.rs
                    src/fabric_durability/adapters.rs
                  )
                  ports=(
                    src/fabric_membership/ports.rs
                    src/fabric_time/ports.rs
                    src/fabric_transport/ports.rs
                    src/fabric_durability/ports.rs
                  )

                  if rg -n 'pub trait ' "''${adapters[@]}"; then
                    echo "maintained adapter module owns an application port" >&2
                    exit 1
                  fi
                  if rg -n 'Result<[^>]*, *String>' "''${ports[@]}"; then
                    echo "maintained fabric port returns a raw string failure" >&2
                    exit 1
                  fi
                  if rg -n --glob '!**/tests.rs' --glob '!**/*_tests.rs' 'std::fs::|std::env::|std::process::|SystemTime::now|Instant::now|std::thread::sleep|iroh::Endpoint|redb::Database' crates/molten-core/src; then
                    echo "pure fabric core contains a host effect" >&2
                    exit 1
                  fi
                  if rg -n --glob '!**/tests.rs' --glob '!**/*_tests.rs' 'LiveClockAdapter::new|OperatingSystemEntropySource::default|IrohTransportAdapter::new|RedbDurableStateAdapter::' crates/molten-core/src; then
                    echo "pure fabric core constructs a concrete adapter" >&2
                    exit 1
                  fi
                  touch "$out"
                '';

            consensus-fastpath-model-profile =
              pkgs.runCommand "molten-consensus-fastpath-model-profile"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.jq
                    pkgs.ripgrep
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  profile=config/consensus-fastpath/profile.ncl
                  negative_production=config/consensus-fastpath/negative-production.ncl
                  negative_reference=config/consensus-fastpath/negative-reference.ncl
                  expected_profiles=2
                  expected_three_replicas=3
                  expected_five_replicas=5

                  nickel typecheck "$profile"
                  nickel export --format json "$profile" > "$TMPDIR/profile.json"
                  test "$(jq 'length' "$TMPDIR/profile.json")" = "$expected_profiles"
                  jq -e '.three_replica.claim_profile == "pure-model-only"' "$TMPDIR/profile.json"
                  jq -e \
                    --argjson three "$expected_three_replicas" \
                    --argjson five "$expected_five_replicas" \
                    '.three_replica.node_count == $three and .five_replica.node_count == $five' \
                    "$TMPDIR/profile.json"

                  for fixture in "$negative_production" "$negative_reference"; do
                    if nickel export --format json "$fixture" > "$TMPDIR/negative.json" 2> "$TMPDIR/negative.stderr"; then
                      echo "negative fast-path profile unexpectedly passed: $fixture" >&2
                      exit 1
                    fi
                  done

                  if rg -n 'flux_profiler|enable_profiler|std::fs|std::env|std::time::Instant' src/fabric_consistency/fastpath; then
                    echo "pure fast-path model contains an ambient shell dependency" >&2
                    exit 1
                  fi
                  touch "$out"
                '';

            nickel-toolchain-cohort =
              pkgs.runCommand "molten-nickel-toolchain-cohort"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.jq
                    pkgs.ripgrep
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cd "$src"
                  nickel --version | tee "$TMPDIR/nickel-version.txt"
                  grep -Fq 'nickel 1.17.0 (rev 1320a98)' "$TMPDIR/nickel-version.txt"
                  rg -U 'name = "nickel-lang"\nversion = "2.2.0"' Cargo.lock
                  rg -U 'name = "nickel-lang-core"\nversion = "0.18.0"' Cargo.lock
                  rg -U 'name = "nickel-lang-parser"\nversion = "0.3.0"' Cargo.lock
                  rg -U 'name = "nickel-lang-vector"\nversion = "0.2.0"' Cargo.lock
                  jq -e '.nodes."nickel-cli".locked.rev == "1320a983e6c3d1e2fb53dd2464b084b4903b1426"' flake.lock >/dev/null
                  fixture_candidate_ref=blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6
                  candidate_input="candidate_source_ref=\"$fixture_candidate_ref\""
                  nickel export docs/production-node-profile.ncl -- "$candidate_input" > "$TMPDIR/production-profile.json"
                  for fixture in \
                    docs/production-profile-fixtures/negative/malformed-ref.ncl \
                    docs/production-profile-fixtures/negative/fractional-limit.ncl \
                    docs/production-profile-fixtures/negative/unsupported-metadata.ncl \
                    docs/production-profile-fixtures/negative/missing-required-adapter.ncl
                  do
                    if nickel export "$fixture" > "$TMPDIR/negative.json" 2> "$TMPDIR/negative.err"; then
                      echo "negative Nickel cohort fixture unexpectedly exported: $fixture" >&2
                      exit 1
                    fi
                  done
                  printf '%s\n' 'import "missing-cohort-import.ncl"' > "$TMPDIR/missing-import.ncl"
                  if nickel export "$TMPDIR/missing-import.ncl" > "$TMPDIR/missing-import.json" 2> "$TMPDIR/missing-import.err"; then
                    echo 'missing Nickel import unexpectedly exported' >&2
                    exit 1
                  fi
                  touch "$out"
                '';

            production-profile-fixtures =
              pkgs.runCommand "molten-production-profile-fixtures"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.diffutils
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cp -R $src source
                  chmod -R u+w source
                  cd source

                  fixture_candidate_ref=blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6
                  candidate_input="candidate_source_ref=\"$fixture_candidate_ref\""
                  nickel export docs/production-node-profile.ncl -- "$candidate_input" > "$TMPDIR/production-node-profile.json"
                  nickel export docs/production-profile-fixtures/valid.ncl > "$TMPDIR/production-profile-valid.json"
                  diff -u "$TMPDIR/production-profile-valid.json" "$TMPDIR/production-node-profile.json"
                  nickel export docs/production-node-profile.ncl --field profile.resource_limits -- "$candidate_input" > "$TMPDIR/resource-limits.json"
                  diff -u docs/production-profile-fixtures/expected-resource-limits.json "$TMPDIR/resource-limits.json"

                  if nickel export docs/production-node-profile.ncl > "$TMPDIR/missing-candidate.json" 2> "$TMPDIR/missing-candidate.err"; then
                    echo "production profile without a candidate input unexpectedly exported" >&2
                    exit 1
                  fi

                  failed=0
                  for fixture in docs/production-profile-fixtures/negative/*.ncl; do
                    name=$(basename "$fixture")
                    if nickel export "$fixture" > "$TMPDIR/$name.json" 2> "$TMPDIR/$name.err"; then
                      echo "negative fixture unexpectedly exported: $fixture" >&2
                      failed=1
                    fi
                  done
                  if [ "$failed" -ne 0 ]; then
                    exit 1
                  fi
                  touch "$out"
                '';

            contract-export-drift-gate =
              pkgs.runCommand "molten-contract-export-drift-gate"
                {
                  nativeBuildInputs = [
                    pkgs.nickel
                    pkgs.diffutils
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cp -R $src source
                  chmod -R u+w source
                  cd source

                  nickel export cairn-policy/default.ncl > "$TMPDIR/cairn-policy.json"
                  diff -u cairn-policy/generated/cairn-policy.json "$TMPDIR/cairn-policy.json"

                  nickel export docs/plugin-extension-contracts/storage.contract-envelope.ncl > "$TMPDIR/storage.contract-envelope.json"
                  diff -u docs/plugin-extension-contracts/generated/storage.contract-envelope.json "$TMPDIR/storage.contract-envelope.json"
                  nickel export docs/plugin-extension-contracts/storage.grant-envelope.ncl > "$TMPDIR/storage.grant-envelope.json"
                  diff -u docs/plugin-extension-contracts/generated/storage.grant-envelope.json "$TMPDIR/storage.grant-envelope.json"

                  fixture_candidate_ref=blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6
                  candidate_input="candidate_source_ref=\"$fixture_candidate_ref\""
                  nickel export docs/production-node-profile.ncl --field profile.resource_limits -- "$candidate_input" > "$TMPDIR/resource-limits.json"
                  diff -u docs/production-profile-fixtures/expected-resource-limits.json "$TMPDIR/resource-limits.json"

                  failed=0
                  positive_fixture() {
                    label="$1"
                    fixture="$2"
                    if ! nickel export "$fixture" > "$TMPDIR/$label.json" 2> "$TMPDIR/$label.err"; then
                      echo "positive fixture failed: $fixture" >&2
                      cat "$TMPDIR/$label.err" >&2
                      failed=1
                    fi
                  }
                  negative_fixture() {
                    label="$1"
                    fixture="$2"
                    if nickel export "$fixture" > "$TMPDIR/$label.json" 2> "$TMPDIR/$label.err"; then
                      echo "negative fixture unexpectedly exported: $fixture" >&2
                      failed=1
                    fi
                  }

                  if ! nickel export docs/production-node-profile.ncl -- "$candidate_input" > "$TMPDIR/production-node-profile.json" 2> "$TMPDIR/production-node-profile.err"; then
                    echo "customized production profile failed" >&2
                    cat "$TMPDIR/production-node-profile.err" >&2
                    failed=1
                  fi
                  if nickel export docs/production-node-profile.ncl > "$TMPDIR/production-node-profile-missing.json" 2> "$TMPDIR/production-node-profile-missing.err"; then
                    echo "production profile without candidate input unexpectedly exported" >&2
                    failed=1
                  fi
                  positive_fixture production-profile-valid docs/production-profile-fixtures/valid.ncl
                  diff -u "$TMPDIR/production-profile-valid.json" "$TMPDIR/production-node-profile.json"
                  for fixture in docs/production-profile-fixtures/negative/*.ncl; do
                    negative_fixture "production-$(basename "$fixture" .ncl)" "$fixture"
                  done
                  # r[impl molten.runtime_spine.preserves_boundary_profile.final_validation]
                  # r[impl molten.runtime_spine.preserves_boundary_profile.final_validation.fixtures]
                  # r[verify molten.runtime_spine.preserves_boundary_profile.docs]
                  # r[verify molten.runtime_spine.preserves_boundary_profile.docs.non_claims]
                  # r[verify molten.runtime_spine.preserves_boundary_profile.fixtures.positive]
                  # r[verify molten.runtime_spine.preserves_boundary_profile.fixtures.negative]
                  positive_fixture preserves-boundary-profile-valid docs/preserves-boundary-profile/valid.ncl
                  for fixture in \
                    docs/preserves-boundary-profile/non-canonical.ncl \
                    docs/preserves-boundary-profile/missing-schema-label.ncl \
                    docs/preserves-boundary-profile/stale-ref.ncl \
                    docs/preserves-boundary-profile/raw-core-coupling.ncl
                  do
                    negative_fixture "preserves-boundary-$(basename "$fixture" .ncl)" "$fixture"
                  done
                  grep -Fq 'Profile success proves canonical boundary identity and adapter placement only' \
                    docs/modularity-boundaries.md
                  grep -Fq 'it does not prove transport liveness, actor authority correctness, replay completeness, or Valence Evidence IR acceptance' \
                    docs/modularity-boundaries.md
                  positive_fixture peer-profile-valid docs/peer-profile-fixtures/valid.ncl
                  for fixture in docs/peer-profile-fixtures/negative/*.ncl; do
                    negative_fixture "peer-$(basename "$fixture" .ncl)" "$fixture"
                  done
                  for fixture in docs/multinode-scenario-fixtures/valid/*.ncl; do
                    positive_fixture "multinode-valid-$(basename "$fixture" .ncl)" "$fixture"
                  done
                  for fixture in docs/multinode-scenario-fixtures/negative/*.ncl; do
                    negative_fixture "multinode-negative-$(basename "$fixture" .ncl)" "$fixture"
                  done
                  positive_fixture plugin-storage docs/plugin-extension-contracts/storage.ncl
                  positive_fixture plugin-storage-grant docs/plugin-extension-contracts/storage.grant.ncl
                  positive_fixture plugin-storage-revoked-grant docs/plugin-extension-contracts/storage-revoked.grant.ncl
                  positive_fixture plugin-storage-contract-envelope docs/plugin-extension-contracts/storage.contract-envelope.ncl
                  positive_fixture plugin-storage-grant-envelope docs/plugin-extension-contracts/storage.grant-envelope.ncl
                  for fixture in docs/plugin-extension-contracts/storage-*.ncl docs/plugin-extension-contracts/storage-*.grant.ncl; do
                    case "$fixture" in
                      docs/plugin-extension-contracts/storage-revoked.grant.ncl|docs/plugin-extension-contracts/storage.contract-envelope.ncl|docs/plugin-extension-contracts/storage.grant-envelope.ncl) continue ;;
                    esac
                    negative_fixture "plugin-$(basename "$fixture" .ncl)" "$fixture"
                  done
                  positive_fixture cairn-policy-default cairn-policy/default.ncl
                  for fixture in cairn-policy/fixtures/*.ncl; do
                    name=$(basename "$fixture" .ncl)
                    case "$name" in
                      valid*) positive_fixture "cairn-$name" "$fixture" ;;
                      *) negative_fixture "cairn-$name" "$fixture" ;;
                    esac
                  done

                  if [ "$failed" -ne 0 ]; then
                    exit 1
                  fi
                  touch "$out"
                '';

            kache-nix-rust-wrapper-contract =
              let
                missingCacheDiagnostic = "cache directory is not writable";
                fakeKache = pkgs.writeShellApplication {
                  name = "kache";
                  text = ''
                    if [ -n "''${KACHE_NIX_TRACE:-}" ]; then
                      {
                        printf 'fake_kache_invoked=true\n'
                        printf 'argv=%s\n' "$*"
                        printf 'KACHE_KEY_SALT=%s\n' "''${KACHE_KEY_SALT:-}"
                        printf 'KACHE_CACHE_DIR=%s\n' "''${KACHE_CACHE_DIR:-}"
                      } >> "$KACHE_NIX_TRACE"
                    fi
                    exec "$@"
                  '';
                };
                checkedKacheLib = import (onix-core-src + "/lib/kache-nix-rust.nix") {
                  lib = pkgs.lib;
                  inherit pkgs;
                  kachePackage = fakeKache;
                };
                checkedWrappedRust = checkedKacheLib.mkWrappedRustPackage {
                  name = "molten-checked-kache-rust";
                  rust = rustToolchain;
                  cacheDir = kacheCacheDir;
                  keySalt = kacheKeySalt;
                };
                disabledRust = mkUnit2nixRust { enableKache = false; };
              in
              pkgs.runCommand "molten-kache-nix-rust-wrapper-contract" { } ''
                set -eu

                if [ ${pkgs.lib.escapeShellArg (toString disabledRust)} != ${pkgs.lib.escapeShellArg (toString rustToolchain)} ]; then
                  echo "negative: disabled unit2nix path must use the unwrapped Rust toolchain" >&2
                  exit 1
                fi
                if [ ${pkgs.lib.escapeShellArg (toString checkedWrappedRust)} = ${pkgs.lib.escapeShellArg (toString rustToolchain)} ]; then
                  echo "positive: enabled unit2nix path must use a wrapped Rust toolchain" >&2
                  exit 1
                fi

                wrapper=${checkedWrappedRust}/bin/rustc
                cache_dir="$PWD/cache"
                trace="$PWD/kache.trace"

                KACHE_NIX_DISABLED=1 KACHE_NIX_TRACE="$PWD/disabled.trace" "$wrapper" -vV > "$PWD/disabled-rustc-version.txt"
                if [ -s "$PWD/disabled.trace" ]; then
                  echo "negative: disabled mode should not invoke kache" >&2
                  exit 1
                fi

                if KACHE_NIX_CACHE_DIR="$PWD/missing-cache" "$wrapper" -vV > "$PWD/missing-cache.stdout" 2> "$PWD/missing-cache.stderr"; then
                  echo "negative: missing cache directory unexpectedly succeeded" >&2
                  exit 1
                fi
                if ! ${pkgs.gnugrep}/bin/grep -Fq ${pkgs.lib.escapeShellArg missingCacheDiagnostic} "$PWD/missing-cache.stderr"; then
                  echo "negative: missing cache directory did not produce the expected diagnostic" >&2
                  cat "$PWD/missing-cache.stderr" >&2
                  exit 1
                fi

                mkdir -p "$cache_dir"
                KACHE_NIX_CACHE_DIR="$cache_dir" KACHE_NIX_TRACE="$trace" "$wrapper" -vV > "$PWD/wrapped-rustc-version.txt"
                if ! ${pkgs.gnugrep}/bin/grep -Fq 'fake_kache_invoked=true' "$trace"; then
                  echo "positive: wrapper did not invoke kache" >&2
                  cat "$trace" >&2
                  exit 1
                fi
                if ! ${pkgs.gnugrep}/bin/grep -Fq "KACHE_CACHE_DIR=$cache_dir" "$trace"; then
                  echo "positive: wrapper did not export the runtime cache directory" >&2
                  cat "$trace" >&2
                  exit 1
                fi
                if ! ${pkgs.gnugrep}/bin/grep -Fq 'operator=${kacheKeySalt}' "$trace"; then
                  echo "positive: wrapper did not include the operator key salt" >&2
                  cat "$trace" >&2
                  exit 1
                fi
                if ! [ -x ${checkedWrappedRust}/bin/rustdoc ]; then
                  echo "positive: wrapped rust package must preserve rustdoc compatibility" >&2
                  exit 1
                fi
                if ${pkgs.gnugrep}/bin/grep -R -Fq '/home/<user>/.cache/kache' ${checkedWrappedRust}; then
                  echo "negative: wrapper must not reference the user-level kache cache" >&2
                  exit 1
                fi
                if ${pkgs.gnugrep}/bin/grep -R -Fq '/home/<user>/.cargo/config.toml' ${checkedWrappedRust}; then
                  echo "negative: wrapper must not read user-level Cargo config" >&2
                  exit 1
                fi

                touch "$out"
              '';

            nextest =
              pkgs.runCommand "molten-nextest"
                {
                  nativeBuildInputs = [
                    rustToolchain
                    pkgs.cargo-nextest
                    pkgs.perl
                    moltenPkg
                  ];
                  src = sourceForConfigChecks;
                  testBinaries = moltenTestBinaries;
                  inherit targetTriple rustLibDir;
                }
                ''
                  set -euo pipefail
                  export HOME="$TMPDIR/home"
                  mkdir -p "$HOME"
                  cp -R $src source
                  chmod -R u+w source
                  cd source
                  cargo metadata --format-version 1 --no-deps --locked > cargo-metadata.json
                  perl -MJSON::PP -e '
                    use strict;
                    use warnings;
                    my $metadata_path = "cargo-metadata.json";
                    my $test_dir = "$ENV{testBinaries}/tests";
                    my $target_triple = $ENV{targetTriple};
                    my $rust_lib_dir = $ENV{rustLibDir};
                    die "missing unit2nix test binary directory: $test_dir\n" unless -d $test_dir;

                    my $metadata = decode_json(do { local $/; open my $fh, "<", $metadata_path or die "open $metadata_path: $!"; <$fh> });
                    my ($package) = grep { $_->{name} eq "molten" } @{$metadata->{packages}};
                    die "cargo metadata did not contain package molten\n" unless $package;

                    my @binaries;
                    for my $path (sort glob("$test_dir/*")) {
                      next unless -f $path && -x $path;
                      next unless system("$path --list --format terse >/dev/null 2>&1") == 0;
                      push @binaries, $path;
                    }
                    die "no libtest-compatible binaries found in $test_dir\n" unless @binaries;

                    my %rust_binaries;
                    my $index = 0;
                    for my $path (@binaries) {
                      (my $name = $path) =~ s{.*/}{};
                      my $id = "molten::nix-test/$index/$name";
                      $rust_binaries{$id} = {
                        "binary-id" => $id,
                        "binary-name" => $name,
                        "package-id" => $package->{id},
                        "kind" => "test",
                        "binary-path" => $path,
                        "build-platform" => "target",
                      };
                      $index++;
                    }

                    my $binaries_metadata = {
                      "rust-build-meta" => {
                        "target-directory" => $ENV{testBinaries},
                        "base-output-directories" => ["tests"],
                        "non-test-binaries" => {},
                        "build-script-out-dirs" => {},
                        "build-script-info" => {},
                        "linked-paths" => [],
                        "platforms" => {
                          "host" => {
                            "platform" => {
                              "triple" => $target_triple,
                              "target-features" => "unknown",
                            },
                            "libdir" => {
                              "status" => "available",
                              "path" => $rust_lib_dir,
                            },
                          },
                          "targets" => [],
                        },
                        "target-platforms" => [{
                          "triple" => $target_triple,
                          "target-features" => "unknown",
                        }],
                        "target-platform" => undef,
                      },
                      "rust-binaries" => \%rust_binaries,
                    };

                    open my $out, ">", "binaries-metadata.json" or die "write binaries-metadata.json: $!";
                    print $out JSON::PP->new->canonical->pretty->encode($binaries_metadata);
                  '
                  cargo nextest run \
                    --profile ci \
                    --user-config-file none \
                    --cargo-metadata cargo-metadata.json \
                    --binaries-metadata binaries-metadata.json \
                    --no-tests fail
                  mkdir -p "$out"
                  cp cargo-metadata.json binaries-metadata.json "$out"/
                  if [ ! -s target/nextest/ci/junit.xml ]; then
                    echo "missing CI JUnit evidence at target/nextest/ci/junit.xml" >&2
                    find target/nextest -maxdepth 4 -type f | sort >&2 || true
                    exit 1
                  fi
                  cp target/nextest/ci/junit.xml "$out"/
                  molten test traceability ci-run-receipt \
                    --source-marker "$src" \
                    --profile-id ci \
                    --command-surface 'cargo nextest run --profile ci --user-config-file none --cargo-metadata cargo-metadata.json --binaries-metadata binaries-metadata.json --no-tests fail' \
                    --nextest-config .config/nextest.toml \
                    --cargo-metadata cargo-metadata.json \
                    --binaries-metadata binaries-metadata.json \
                    --junit target/nextest/ci/junit.xml \
                    --caveat 'JUnit is a rendered view over canonical CI receipt metadata' \
                    --out "$out/ci-test-run-receipt.preserves"
                '';

            dogfood-local-node =
              pkgs.runCommand "molten-dogfood-local-node"
                {
                  nativeBuildInputs = [
                    moltenPkg
                    pkgs.gnugrep
                  ];
                  src = sourceForConfigChecks;
                  nextestCheck = nextest;
                }
                ''
                  set -euo pipefail
                  export HOME="$TMPDIR/home"
                  mkdir -p "$HOME"
                  cp -R $src source
                  chmod -R u+w source
                  cd source
                  molten dogfood local-node \
                    --state-root "$TMPDIR/dogfood-state" \
                    --out dogfood-report.preserves \
                    --release-gate-out release-gate.preserves \
                    --replay-verify-out replay-verify.preserves \
                    --replay-index-out replay-evidence-index.preserves \
                    > dogfood-summary.txt
                  grep -q 'decision=pass' dogfood-summary.txt
                  grep -q 'dogfood-report-v1' dogfood-report.preserves
                  grep -q 'release-gate-receipt-v1' release-gate.preserves
                  grep -q 'deterministic-replay-verify-v1' replay-verify.preserves
                  grep -q 'deterministic-replay-index-v1' replay-evidence-index.preserves
                  mkdir -p "$out"
                  cp dogfood-summary.txt dogfood-report.preserves release-gate.preserves replay-verify.preserves replay-evidence-index.preserves "$out"/
                  printf '%s\n' "$nextestCheck" > "$out/after-nextest.txt"
                  molten dogfood nix-release-export \
                    --output-path "$out" \
                    --out "$out/nix-dogfood-evidence.preserves"
                  molten dogfood nix-release-verify \
                    --output-path "$out" \
                    --evidence "$out/nix-dogfood-evidence.preserves" \
                    --receipt-out "$out/nix-dogfood-verify.preserves" \
                    | tee "$out/nix-dogfood-verify.txt"
                  grep -q 'decision=pass' "$out/nix-dogfood-verify.txt"
                  molten dogfood release-bundle-export \
                    --output-path "$out" \
                    --out "$out/release-evidence-bundle.preserves"
                  mkdir -p "$out/signed-keyring"
                  molten receipts key import \
                    --ledger "$out/signed-keyring" \
                    --key-id local-release-key-v1 \
                    --signer local-release-signer \
                    --trust-root local-release-trust-root \
                    --key local-release-key \
                    > "$out/signed-keyring-import.txt"
                  molten receipts sign "$out/dogfood-report.preserves" \
                    --out "$out/dogfood-report.signed.preserves" \
                    --signer local-release-signer \
                    --purpose release-evidence \
                    --trust-root local-release-trust-root \
                    --key local-release-key
                  molten receipts sign "$out/release-gate.preserves" \
                    --out "$out/release-gate.signed.preserves" \
                    --signer local-release-signer \
                    --purpose release-evidence \
                    --trust-root local-release-trust-root \
                    --key local-release-key
                  molten receipts sign "$out/replay-verify.preserves" \
                    --out "$out/replay-verify.signed.preserves" \
                    --signer local-release-signer \
                    --purpose release-evidence \
                    --trust-root local-release-trust-root \
                    --key local-release-key
                  molten receipts sign "$out/replay-evidence-index.preserves" \
                    --out "$out/replay-evidence-index.signed.preserves" \
                    --signer local-release-signer \
                    --purpose release-evidence \
                    --trust-root local-release-trust-root \
                    --key local-release-key
                  molten receipts sign "$out/nix-dogfood-evidence.preserves" \
                    --out "$out/nix-dogfood-evidence.signed.preserves" \
                    --signer local-release-signer \
                    --purpose release-evidence \
                    --trust-root local-release-trust-root \
                    --key local-release-key
                  molten receipts sign "$out/nix-dogfood-verify.preserves" \
                    --out "$out/nix-dogfood-verify.signed.preserves" \
                    --signer local-release-signer \
                    --purpose release-evidence \
                    --trust-root local-release-trust-root \
                    --key local-release-key
                  molten dogfood release-bundle-verify \
                    --output-path "$out" \
                    --bundle "$out/release-evidence-bundle.preserves" \
                    --receipt-out "$out/release-evidence-bundle-verify.preserves" \
                    --require-signed-members \
                    --signed-purpose release-evidence \
                    --signed-trust-root local-release-trust-root \
                    --signed-key-ledger "$out/signed-keyring" \
                    --signed-key-id local-release-key-v1 \
                    --signed-signer local-release-signer \
                    --signed-member "$out/dogfood-report.signed.preserves" \
                    --signed-member "$out/release-gate.signed.preserves" \
                    --signed-member "$out/replay-verify.signed.preserves" \
                    --signed-member "$out/replay-evidence-index.signed.preserves" \
                    --signed-member "$out/nix-dogfood-evidence.signed.preserves" \
                    --signed-member "$out/nix-dogfood-verify.signed.preserves" \
                    | tee "$out/release-evidence-bundle-verify.txt"
                  grep -q 'decision=pass' "$out/release-evidence-bundle-verify.txt"
                  molten dogfood release-promote \
                    --output-path "$out" \
                    --bundle-verify "$out/release-evidence-bundle-verify.preserves" \
                    --receipt-out "$out/release-promotion-gate.preserves" \
                    --signed-key-ledger "$out/signed-keyring" \
                    --signed-key-id local-release-key-v1 \
                    --signed-trust-root local-release-trust-root \
                    --signed-signer local-release-signer \
                    --source-evidence "flake-source:$src" \
                    --octet-evidence "octet:external-clean-gate-required" \
                    --cairn-evidence "cairn:external-strict-validate-required" \
                    | tee "$out/release-promotion-gate.txt"
                  grep -q 'decision=pass' "$out/release-promotion-gate.txt"
                  molten receipts sign "$out/release-promotion-gate.preserves" \
                    --out "$out/release-promotion-gate.signed.preserves" \
                    --signer local-release-signer \
                    --purpose release-promotion \
                    --trust-root local-release-trust-root \
                    --key local-release-key
                  promotion_ref=$(sed -n 's/.* receipt=\([^ ]*\).*/\1/p' "$out/release-promotion-gate.txt")
                  test -n "$promotion_ref"
                  molten receipts verify-signed "$out/release-promotion-gate.signed.preserves" \
                    --purpose release-promotion \
                    --trust-root local-release-trust-root \
                    --key-ledger "$out/signed-keyring" \
                    --key-id local-release-key-v1 \
                    --signer local-release-signer \
                    --subject-ref "$promotion_ref" \
                    | tee "$out/release-promotion-gate-signed-verify.txt"
                  grep -q 'receipts verify-signed ok' "$out/release-promotion-gate-signed-verify.txt"
                  molten dogfood release-promotion-summary \
                    --output-path "$out" \
                    --out "$out/release-promotion-summary.preserves" \
                    --signed-key-ledger "$out/signed-keyring" \
                    --signed-key-id local-release-key-v1 \
                    --signed-trust-root local-release-trust-root \
                    --signed-signer local-release-signer \
                    | tee "$out/release-promotion-summary.txt"
                  grep -q 'decision=pass' "$out/release-promotion-summary.txt"
                  molten dogfood release-export \
                    --output-path "$out" \
                    --out "$out/release-evidence.tar.zst" \
                    --manifest-out "$out/release-export-manifest.preserves" \
                    | tee "$out/release-export.txt"
                  grep -q 'release-export manifest=' "$out/release-export.txt"
                  molten dogfood release-export-verify \
                    --bundle "$out/release-evidence.tar.zst" \
                    --receipt-out "$out/release-export-verify.preserves" \
                    | tee "$out/release-export-verify.txt"
                  grep -q 'decision=pass' "$out/release-export-verify.txt"
                '';

            nixos-vm-multinode = pkgs.testers.runNixOSTest {
              name = "molten-nixos-vm-multinode";
              nodes = {
                node_a = moltenVmNodeModule "node-a";
                node_b = moltenVmNodeModule "node-b";
              };
              testScript = ''
                start_all()
                for machine in [node_a, node_b]:
                    machine.wait_for_unit("molten-node.service")
                    machine.succeed("test -s /var/lib/molten/vm-evidence/startup.preserves")
                    machine.succeed("test -s /var/lib/molten/vm-evidence/health.preserves")
                    machine.succeed("test -s /var/lib/molten/vm-evidence/control-loop.preserves")
                    machine.succeed("test -s /var/lib/molten/vm-evidence/heartbeat.preserves")
                    machine.succeed("grep -q node-startup-receipt-v1 /var/lib/molten/vm-evidence/startup.preserves")
                    machine.succeed("grep -q node-health-receipt-v1 /var/lib/molten/vm-evidence/health.preserves")
                    machine.succeed("grep -q node-control-loop-receipt-v1 /var/lib/molten/vm-evidence/control-loop.preserves")

                node_a.succeed("getent hosts node-b")
                node_b.succeed("getent hosts node-a")
                node_a.succeed("ping -c 1 node-b")
                node_b.succeed("ping -c 1 node-a")

                node_b.succeed("""
                  set -euo pipefail
                  cd '${sourceForConfigChecks}'
                  root=/var/lib/molten-cross/node-b
                  evidence=/var/lib/molten/vm-evidence/live-control
                  rm -rf "$root" "$evidence"
                  mkdir -p "$evidence"
                  molten node init \
                    --state-root "$root" \
                    --node-id node:node-b \
                    --config-out "$evidence/node-config.preserves" \
                    --identity-receipt-out "$evidence/identity.preserves" \
                    > "$evidence/init.txt"
                  molten node run \
                    --state-root "$root" \
                    --startup-out "$evidence/startup.preserves" \
                    > "$evidence/run.txt"
                  molten node serve \
                    --state-root "$root" \
                    --live-iroh \
                    --live-max-events 1 \
                    --live-event-timeout-ms 1 \
                    --max-requests-per-tick 1 \
                    --service-receipt-out "$evidence/live-service.preserves" \
                    --live-ticket-out "$evidence/ticket.preserves" \
                    --receipt-out "$evidence/listener.preserves" \
                    > "$evidence/live-serve.txt"
                  molten node live-peer-admit \
                    --state-root "$root" \
                    --peer node:node-a \
                    --receipt-out "$evidence/peer-admission.preserves" \
                    "$evidence/ticket.preserves" \
                    > "$evidence/admit.txt"
                  molten node authority-grant-fixture \
                    --state-root "$root" \
                    --peer node:node-a \
                    --node node:node-b \
                    --operation status \
                    --target-scope '*' \
                    --resource-scope '*' \
                    --out "$evidence/authority-grant.preserves" \
                    > "$evidence/authority.txt"
                  peer_summary=$(molten test nixos-vm show "$evidence/peer-admission.preserves")
                  peer_ref=''${peer_summary#* ref=}
                  peer_ref=''${peer_ref%% *}
                  authority_summary=$(molten test nixos-vm show "$evidence/authority-grant.preserves")
                  authority_ref=''${authority_summary#* ref=}
                  authority_ref=''${authority_ref%% *}
                  printf '%s\n' "$peer_ref" > "$evidence/peer.ref"
                  printf '%s\n' "$authority_ref" > "$evidence/authority.ref"
                  molten node control-request \
                    --operation status \
                    --authority "$authority_ref" \
                    --policy "$peer_ref" \
                    --resource "$authority_ref" \
                    --out "$evidence/request.preserves" \
                    > "$evidence/request.txt"
                  molten node control-ingress-live-loopback \
                    --state-root "$root" \
                    "$evidence/request.preserves" \
                    --from-peer node:node-a \
                    --to-node node:node-b \
                    --sequence 1 \
                    --peer-bootstrap "$peer_ref" \
                    --authority "$authority_ref" \
                    --policy "$peer_ref" \
                    --resource "$authority_ref" \
                    --publish-receipt-out "$evidence/live-publish.preserves" \
                    --receive-receipt-out "$evidence/live-receive.preserves" \
                    > "$evidence/live-loopback.txt"
                  molten node run-loop \
                    --state-root "$root" \
                    --max-requests 1 \
                    --receipt-out "$evidence/live-control-loop.preserves" \
                    --heartbeat-out "$evidence/live-heartbeat.preserves" \
                    > "$evidence/live-run-loop.txt"
                  cp "$root"/control/iroh-ingress/receipts/*.deliver.receipt.preserves "$evidence/ingress.preserves"
                  cp "$root"/control/inbox/*.queue-receipt.preserves "$evidence/queue.preserves"
                  cp "$root"/control/outbox/*.control-receipt.preserves "$evidence/control.preserves"
                  molten node live-workflow-bundle-export \
                    --ticket "$evidence/ticket.preserves" \
                    --peer-admission "$evidence/peer-admission.preserves" \
                    --authority-grant "$evidence/authority-grant.preserves" \
                    --receipt "$evidence/listener.preserves" \
                    --receipt "$evidence/live-service.preserves" \
                    --receipt "$evidence/live-receive.preserves" \
                    --out "$evidence/bundle.preserves" \
                    --receipt-out "$evidence/bundle-export.preserves" \
                    > "$evidence/bundle-export.txt"
                  grep -q node-control-live-workflow-bundle-v1 "$evidence/bundle.preserves"
                  grep -q node-control-live-transport-receipt-v1 "$evidence/live-receive.preserves"
                """)
                node_a.succeed("mkdir -p /var/lib/molten/vm-evidence/live-control")
                for artifact in [
                    "ticket.preserves",
                    "peer-admission.preserves",
                    "authority-grant.preserves",
                    "bundle.preserves",
                    "request.preserves",
                    "ingress.preserves",
                    "queue.preserves",
                    "control.preserves",
                    "peer.ref",
                    "authority.ref",
                    "live-loopback.txt",
                ]:
                    content = node_b.succeed(f"cat /var/lib/molten/vm-evidence/live-control/{artifact}")
                    node_a.succeed(f"cat > /var/lib/molten/vm-evidence/live-control/{artifact} <<'EOF'\n" + content + "\nEOF")
                node_a.succeed("""
                  set -euo pipefail
                  cd '${sourceForConfigChecks}'
                  root=/var/lib/molten-cross/node-a
                  evidence=/var/lib/molten/vm-evidence/live-control
                  peer_ref=$(cat "$evidence/peer.ref")
                  authority_ref=$(cat "$evidence/authority.ref")
                  rm -rf "$root"
                  molten node init \
                    --state-root "$root" \
                    --node-id node:node-a \
                    --config-out "$evidence/sender-node-config.preserves" \
                    --identity-receipt-out "$evidence/sender-identity.preserves" \
                    > "$evidence/sender-init.txt"
                  molten node run \
                    --state-root "$root" \
                    --startup-out "$evidence/sender-startup.preserves" \
                    > "$evidence/sender-run.txt"
                  molten node live-workflow-bundle-verify \
                    "$evidence/bundle.preserves" \
                    --expected-node node:node-b \
                    --expected-peer node:node-a \
                    --operation status \
                    --receipt-out "$evidence/verify.preserves" \
                    > "$evidence/verify.txt"
                  molten node live-workflow-bundle-gate \
                    "$evidence/bundle.preserves" \
                    --verify-receipt "$evidence/verify.preserves" \
                    --require-verify-receipt \
                    --expected-node node:node-b \
                    --expected-peer node:node-a \
                    --operation status \
                    --receipt-out "$evidence/gate.preserves" \
                    > "$evidence/gate.txt"
                  molten node live-workflow-bundle-apply \
                    --state-root "$root" \
                    "$evidence/bundle.preserves" \
                    --gate-receipt "$evidence/gate.preserves" \
                    --require-gate-receipt \
                    --request "$evidence/request.preserves" \
                    --from-peer node:node-a \
                    --sequence 1 \
                    --peer-bootstrap "$peer_ref" \
                    --authority "$authority_ref" \
                    --policy "$peer_ref" \
                    --resource "$authority_ref" \
                    --expected-node node:node-b \
                    --expected-peer node:node-a \
                    --operation status \
                    --receipt-out "$evidence/apply.preserves" \
                    > "$evidence/apply.txt"
                  molten node live-workflow-bundle-reconcile \
                    "$evidence/apply.preserves" \
                    --ingress-receipt "$evidence/ingress.preserves" \
                    --queue-receipt "$evidence/queue.preserves" \
                    --control-receipt "$evidence/control.preserves" \
                    --receipt-out "$evidence/reconcile.preserves" \
                    > "$evidence/reconcile.txt"
                  molten node live-workflow-bundle-ack-export \
                    "$evidence/apply.preserves" \
                    --ingress-receipt "$evidence/ingress.preserves" \
                    --queue-receipt "$evidence/queue.preserves" \
                    --control-receipt "$evidence/control.preserves" \
                    --reconcile-receipt "$evidence/reconcile.preserves" \
                    --out "$evidence/ack.preserves" \
                    --receipt-out "$evidence/ack-export.preserves" \
                    > "$evidence/ack-export.txt"
                  molten node live-workflow-bundle-ack-import \
                    --state-root "$root" \
                    "$evidence/ack.preserves" \
                    --receipt-out "$evidence/ack-import.preserves" \
                    > "$evidence/ack-import.txt"
                  molten node live-workflow-bundle-protocol-gate \
                    "$evidence/bundle.preserves" \
                    --gate-receipt "$evidence/gate.preserves" \
                    --apply-receipt "$evidence/apply.preserves" \
                    --reconcile-receipt "$evidence/reconcile.preserves" \
                    --ack "$evidence/ack.preserves" \
                    --receipt-out "$evidence/protocol-gate.preserves" \
                    > "$evidence/protocol-gate.txt"
                  grep -q 'decision "pass"' "$evidence/apply.preserves"
                  grep -q 'decision "pass"' "$evidence/reconcile.preserves"
                  grep -q 'decision "pass"' "$evidence/ack-export.preserves"
                  grep -q 'decision "pass"' "$evidence/ack-import.preserves"
                  grep -q 'decision "pass"' "$evidence/protocol-gate.preserves"
                """)

                node_a.succeed("""
                  set -euo pipefail
                  cd '${sourceForConfigChecks}'
                  evidence=/var/lib/molten/vm-evidence/service-job
                  rm -rf "$evidence"
                  mkdir -p "$evidence"
                  make_ref() {
                    label=$1
                    file="$evidence/ref-$label.preserves"
                    printf '<vm-synthetic-ref "%s">\n' "$label" > "$file"
                    summary=$(molten test nixos-vm show "$file")
                    ref=''${summary#* ref=}
                    ref=''${ref%% *}
                    printf '%s\n' "$ref"
                  }
                  printf '<vm-service-message "node_a" "node_b">\n' > "$evidence/remote-payload.preserves"
                  remote_policy_ref=$(make_ref remote-policy)
                  remote_capability_ref=$(make_ref remote-capability)
                  remote_evidence_ref=$(make_ref remote-evidence)
                  molten test remote envelope build \
                    --from-peer node_a \
                    --from-actor service-client \
                    --to-peer node_b \
                    --topic molten.vm.service \
                    --operation message \
                    --payload "$evidence/remote-payload.preserves" \
                    --capability-ref "$remote_capability_ref" \
                    --evidence-ref "$remote_evidence_ref" \
                    --out "$evidence/remote-envelope.preserves" \
                    > "$evidence/remote-envelope.txt"
                  remote_summary=$(molten test nixos-vm show "$evidence/remote-envelope.preserves")
                  remote_ref=''${remote_summary#* ref=}
                  remote_ref=''${remote_ref%% *}
                  printf '%s\n' "$remote_ref" > "$evidence/remote-envelope.ref"
                  printf 'echo' > "$evidence/executable.bin"
                  printf 'hello' > "$evidence/input.bin"
                  exe_summary=$(molten test chunk put "$evidence/executable.bin" \
                    --store "$evidence/source-chunks" \
                    --kind job-executable \
                    --manifest-out "$evidence/executable-manifest.preserves" \
                    --receipt-out "$evidence/executable-put.preserves")
                  exe_manifest=''${exe_summary#* manifest=}
                  exe_manifest=''${exe_manifest%% *}
                  input_summary=$(molten test chunk put "$evidence/input.bin" \
                    --store "$evidence/source-chunks" \
                    --kind job-input \
                    --manifest-out "$evidence/input-manifest.preserves" \
                    --receipt-out "$evidence/input-put.preserves")
                  input_manifest=''${input_summary#* manifest=}
                  input_manifest=''${input_manifest%% *}
                  operation_ref=$(make_ref job-operation)
                  policy_ref=$(make_ref job-policy)
                  provenance_ref=$(make_ref job-provenance)
                  effect_ref=$(make_ref job-effect)
                  authority_ref=$(make_ref job-authority)
                  molten test job ref-submit \
                    --job-id job:vm-echo \
                    --operation-id "$operation_ref" \
                    --executable "$exe_manifest@4@elf-executable" \
                    --input "$input_manifest@5@bytes" \
                    --output-mode chunk-manifest \
                    --handler-profile local-echo-v1 \
                    --context-ref "$authority_ref" \
                    --policy-ref "$policy_ref" \
                    --provenance-ref "$provenance_ref" \
                    --effect-ref "$effect_ref" \
                    --evidence-ref "$remote_ref" \
                    --out "$evidence/job-submission.preserves" \
                    > "$evidence/job-ref-submit.txt"
                  coord_policy_ref=$(make_ref coordination-policy)
                  coord_resource_ref=$(make_ref coordination-resource)
                  coord_authority_ref=$(make_ref coordination-authority)
                  coord_operation_ref=$(make_ref coordination-operation)
                  printf '%s\n' "$coord_policy_ref" > "$evidence/coord-policy.ref"
                  printf '%s\n' "$coord_resource_ref" > "$evidence/coord-resource.ref"
                  printf '%s\n' "$coord_authority_ref" > "$evidence/coord-authority.ref"
                  printf '<coordination-item "vm-job-worker">\n' > "$evidence/coord-payload.preserves"
                  molten test coordination request \
                    --service queue \
                    --operation enqueue \
                    --key queue:vm-jobs \
                    --client-session node-a \
                    --operation-id-ref "$coord_operation_ref" \
                    --payload "$evidence/coord-payload.preserves" \
                    --authority-ref "$coord_authority_ref" \
                    --policy-ref "$coord_policy_ref" \
                    --resource-ref "$coord_resource_ref" \
                    --out "$evidence/coord-request.preserves" \
                    > "$evidence/coord-request.txt"
                """)
                node_b.succeed("mkdir -p /var/lib/molten/vm-evidence/service-job")
                for artifact in [
                    "remote-envelope.preserves",
                    "job-submission.preserves",
                    "coord-request.preserves",
                    "coord-policy.ref",
                    "coord-resource.ref",
                    "coord-authority.ref",
                ]:
                    content = node_a.succeed(f"cat /var/lib/molten/vm-evidence/service-job/{artifact}")
                    node_b.succeed(f"cat > /var/lib/molten/vm-evidence/service-job/{artifact} <<'EOF'\n" + content + "\nEOF")
                node_b.succeed("""
                  set -euo pipefail
                  cd '${sourceForConfigChecks}'
                  evidence=/var/lib/molten/vm-evidence/service-job
                  rm -rf "$evidence/target-chunks" "$evidence/job-ledger" "$evidence/coord-apply" "$evidence/remote-transport"
                  printf 'echo' > "$evidence/executable.bin"
                  printf 'hello' > "$evidence/input.bin"
                  molten test remote publish-local \
                    --transport-root "$evidence/remote-transport" \
                    --envelope "$evidence/remote-envelope.preserves" \
                    --node node_a \
                    --receipt-out "$evidence/remote-publish.preserves" \
                    > "$evidence/remote-publish.txt"
                  remote_summary=$(molten test nixos-vm show "$evidence/remote-envelope.preserves")
                  remote_ref=''${remote_summary#* ref=}
                  remote_ref=''${remote_ref%% *}
                  molten test remote deliver-local \
                    --transport-root "$evidence/remote-transport" \
                    --topic molten.vm.service \
                    --envelope-ref "$remote_ref" \
                    --receiver-peer node_b \
                    --out "$evidence/remote-delivered.preserves" \
                    --receipt-out "$evidence/remote-deliver.preserves" \
                    > "$evidence/remote-deliver.txt"
                  molten test chunk put "$evidence/executable.bin" \
                    --store "$evidence/target-chunks" \
                    --kind job-executable \
                    --manifest-out "$evidence/target-executable-manifest.preserves" \
                    --receipt-out "$evidence/target-executable-put.preserves" \
                    > "$evidence/target-executable-put.txt"
                  molten test chunk put "$evidence/input.bin" \
                    --store "$evidence/target-chunks" \
                    --kind job-input \
                    --manifest-out "$evidence/target-input-manifest.preserves" \
                    --receipt-out "$evidence/target-input-put.preserves" \
                    > "$evidence/target-input-put.txt"
                  molten test job ref-execute "$evidence/job-submission.preserves" \
                    --chunks "$evidence/target-chunks" \
                    --ledger "$evidence/job-ledger" \
                    --receipt-out "$evidence/job-ref.receipt.preserves" \
                    > "$evidence/job-ref-execute.txt"
                  coord_policy_ref=$(cat "$evidence/coord-policy.ref")
                  coord_resource_ref=$(cat "$evidence/coord-resource.ref")
                  molten test coordination manifest \
                    --service queue \
                    --policy-ref "$coord_policy_ref" \
                    --resource-ref "$coord_resource_ref" \
                    --out "$evidence/coord-manifest.preserves" \
                    > "$evidence/coord-manifest.txt"
                  molten test coordination apply \
                    --manifest "$evidence/coord-manifest.preserves" \
                    --request "$evidence/coord-request.preserves" \
                    --out "$evidence/coord-apply" \
                    > "$evidence/coord-apply.txt"
                  grep -q remote-dataspace-transport-receipt-v1 "$evidence/remote-deliver.preserves"
                  grep -q 'decision "pass"' "$evidence/job-ref.receipt.preserves"
                  grep -q coordination-apply-report-v1 "$evidence/coord-apply/report.preserves"
                """)
                node_a.succeed("mkdir -p /var/lib/molten/vm-evidence/service-job/coord-apply")
                for artifact in [
                    "remote-deliver.preserves",
                    "remote-deliver.txt",
                    "job-ref.receipt.preserves",
                    "job-ref-execute.txt",
                    "target-executable-put.preserves",
                    "target-input-put.preserves",
                    "coord-apply/report.preserves",
                    "coord-apply.txt",
                ]:
                    content = node_b.succeed(f"cat /var/lib/molten/vm-evidence/service-job/{artifact}")
                    node_a.succeed(f"cat > /var/lib/molten/vm-evidence/service-job/{artifact} <<'EOF'\n" + content + "\nEOF")

                node_b.succeed("""
                  molten node control-request \
                    --operation status \
                    --out /var/lib/molten/vm-evidence/restart-status-request.preserves
                  molten node control-submit \
                    --state-root /var/lib/molten \
                    /var/lib/molten/vm-evidence/restart-status-request.preserves \
                    --receipt-out /var/lib/molten/vm-evidence/restart-queue.preserves
                """)
                node_b.succeed("systemctl restart molten-node.service")
                node_b.wait_for_unit("molten-node.service")
                node_b.succeed("grep -q processed=1 /var/lib/molten/vm-evidence/run-loop.txt")
                node_b.succeed("grep -q node-control-loop-receipt-v1 /var/lib/molten/vm-evidence/control-loop.preserves")

                node_a.succeed("systemctl stop molten-node.service")
                node_b.succeed("systemctl stop molten-node.service")
                for machine in [node_a, node_b]:
                    machine.succeed("test -s /var/lib/molten/vm-evidence/shutdown.preserves")
                    machine.succeed("grep -q node-shutdown-receipt-v1 /var/lib/molten/vm-evidence/shutdown.preserves")

                node_a.succeed("""
                  molten test nixos-vm topology \
                    --node node_a \
                    --node node_b \
                    --package-ref 'store:${moltenPkg}' \
                    --package-path '${moltenPkg}' \
                    --network nixos-test-private \
                    --nix-input 'source:${sourceForConfigChecks}' \
                    --caveat 'vm evidence is platform integration evidence only' \
                    --out /var/lib/molten/vm-evidence/topology.preserves
                """)
                for machine, node_name in [(node_a, "node_a"), (node_b, "node_b")]:
                    machine.succeed(f"""
                      molten test nixos-vm node-evidence \
                        --node {node_name} \
                        --state-root /var/lib/molten \
                        --identity /var/lib/molten/vm-evidence/identity.preserves \
                        --startup /var/lib/molten/vm-evidence/startup.preserves \
                        --health /var/lib/molten/vm-evidence/health.preserves \
                        --control-loop /var/lib/molten/vm-evidence/control-loop.preserves \
                        --heartbeat /var/lib/molten/vm-evidence/heartbeat.preserves \
                        --shutdown /var/lib/molten/vm-evidence/shutdown.preserves \
                        --log /var/lib/molten/vm-evidence/init.txt \
                        --log /var/lib/molten/vm-evidence/run.txt \
                        --log /var/lib/molten/vm-evidence/status.txt \
                        --log /var/lib/molten/vm-evidence/run-loop.txt \
                        --out /var/lib/molten/vm-evidence/node-evidence.preserves
                    """)
                node_b_evidence = node_b.succeed("cat /var/lib/molten/vm-evidence/node-evidence.preserves")
                node_a.succeed("cat > /var/lib/molten/vm-evidence/node-b-evidence.preserves <<'EOF'\n" + node_b_evidence + "\nEOF")
                for artifact in [
                    "restart-queue.preserves",
                    "control-loop.preserves",
                    "heartbeat.preserves",
                    "shutdown.preserves",
                ]:
                    content = node_b.succeed(f"cat /var/lib/molten/vm-evidence/{artifact}")
                    node_a.succeed(f"cat > /var/lib/molten/vm-evidence/node-b-{artifact} <<'EOF'\n" + content + "\nEOF")
                node_a.succeed("""
                  protocol_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/protocol-gate.preserves)
                  protocol_ref=''${protocol_summary#* ref=}
                  protocol_ref=''${protocol_ref%% *}
                  reconcile_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/reconcile.preserves)
                  reconcile_ref=''${reconcile_summary#* ref=}
                  reconcile_ref=''${reconcile_ref%% *}
                  ack_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/ack-export.preserves)
                  ack_ref=''${ack_summary#* ref=}
                  ack_ref=''${ack_ref%% *}
                  remote_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/service-job/remote-deliver.preserves)
                  remote_ref=''${remote_summary#* ref=}
                  remote_ref=''${remote_ref%% *}
                  job_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/service-job/job-ref.receipt.preserves)
                  job_ref=''${job_summary#* ref=}
                  job_ref=''${job_ref%% *}
                  coordination_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/service-job/coord-apply/report.preserves)
                  coordination_ref=''${coordination_summary#* ref=}
                  coordination_ref=''${coordination_ref%% *}
                  ticket_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/ticket.preserves)
                  ticket_ref=''${ticket_summary#* ref=}
                  ticket_ref=''${ticket_ref%% *}
                  peer_admission_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/peer-admission.preserves)
                  peer_admission_ref=''${peer_admission_summary#* ref=}
                  peer_admission_ref=''${peer_admission_ref%% *}
                  authority_summary=$(molten test nixos-vm show /var/lib/molten/vm-evidence/live-control/authority-grant.preserves)
                  authority_ref=''${authority_summary#* ref=}
                  authority_ref=''${authority_ref%% *}
                  fault_dir=/var/lib/molten/vm-evidence/prod-soak-faults
                  mkdir -p "$fault_dir"
                  molten node live-workflow-bundle-gate \
                    /var/lib/molten/vm-evidence/live-control/bundle.preserves \
                    --expected-peer node:stale-peer \
                    --receipt-out "$fault_dir/stale-ticket-gate.preserves" \
                    > "$fault_dir/stale-ticket-gate.txt"
                  grep -q 'decision "deny"' "$fault_dir/stale-ticket-gate.preserves"
                  stale_summary=$(molten test prod-soak show "$fault_dir/stale-ticket-gate.preserves")
                  stale_denial_ref=''${stale_summary#* ref=}
                  stale_denial_ref=''${stale_denial_ref%% *}
                  molten node live-workflow-bundle-gate \
                    /var/lib/molten/vm-evidence/live-control/bundle.preserves \
                    --expected-node node:wrong-authority \
                    --receipt-out "$fault_dir/wrong-authority-gate.preserves" \
                    > "$fault_dir/wrong-authority-gate.txt"
                  grep -q 'decision "deny"' "$fault_dir/wrong-authority-gate.preserves"
                  wrong_authority_summary=$(molten test prod-soak show "$fault_dir/wrong-authority-gate.preserves")
                  wrong_authority_denial_ref=''${wrong_authority_summary#* ref=}
                  wrong_authority_denial_ref=''${wrong_authority_denial_ref%% *}
                  fault_case() {
                    kind=$1
                    expected=$2
                    denial_ref=''${3:-}
                    out="$fault_dir/$kind.preserves"
                    if [ -n "$denial_ref" ]; then
                      molten test prod-soak fault-case \
                        --scenario network-transport-fault-matrix \
                        --fault-kind "$kind" \
                        --injection simulated-vm-fault \
                        --expected-outcome "$expected" \
                        --evidence-ref "$protocol_ref" \
                        --evidence-ref "$remote_ref" \
                        --denial-ref "$denial_ref" \
                        --decision pass \
                        --replay-status simulated-fault \
                        --caveat 'fault evidence is simulated diagnostic evidence for pilot scoping' \
                        --out "$out"
                    else
                      molten test prod-soak fault-case \
                        --scenario network-transport-fault-matrix \
                        --fault-kind "$kind" \
                        --injection simulated-vm-fault \
                        --expected-outcome "$expected" \
                        --evidence-ref "$protocol_ref" \
                        --evidence-ref "$remote_ref" \
                        --decision pass \
                        --replay-status simulated-fault \
                        --caveat 'fault evidence is simulated diagnostic evidence for pilot scoping' \
                        --out "$out"
                    fi
                  }
                  fault_case delay bounded-diagnostic
                  fault_case drop bounded-diagnostic
                  fault_case partition bounded-diagnostic
                  fault_case rejoin bounded-diagnostic
                  fault_case stale-ticket deny-before-side-effects "$stale_denial_ref"
                  fault_case wrong-authority deny-before-side-effects "$wrong_authority_denial_ref"
                  fault_case duplicate-operation idempotent-replay
                  fault_case conflicting-operation-id deny-before-side-effects "$wrong_authority_denial_ref"
                  fault_case corrupted-transport-receipt deny-before-side-effects "$stale_denial_ref"
                  molten test prod-soak fault-matrix \
                    --scenario network-transport-fault-matrix \
                    --fault-case "$fault_dir/delay.preserves" --fault-kind delay \
                    --fault-case "$fault_dir/drop.preserves" --fault-kind drop \
                    --fault-case "$fault_dir/partition.preserves" --fault-kind partition \
                    --fault-case "$fault_dir/rejoin.preserves" --fault-kind rejoin \
                    --fault-case "$fault_dir/stale-ticket.preserves" --fault-kind stale-ticket \
                    --fault-case "$fault_dir/wrong-authority.preserves" --fault-kind wrong-authority \
                    --fault-case "$fault_dir/duplicate-operation.preserves" --fault-kind duplicate-operation \
                    --fault-case "$fault_dir/conflicting-operation-id.preserves" --fault-kind conflicting-operation-id \
                    --fault-case "$fault_dir/corrupted-transport-receipt.preserves" --fault-kind corrupted-transport-receipt \
                    --decision pass \
                    --caveat 'fault matrix evidence is diagnostic and does not prove transport correctness beyond this topology' \
                    --out "$fault_dir/matrix.preserves"
                  fault_matrix_summary=$(molten test prod-soak show "$fault_dir/matrix.preserves")
                  fault_matrix_ref=''${fault_matrix_summary#* ref=}
                  fault_matrix_ref=''${fault_matrix_ref%% *}
                  restart_queue_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/node-b-restart-queue.preserves)
                  restart_queue_ref=''${restart_queue_summary#* ref=}
                  restart_queue_ref=''${restart_queue_ref%% *}
                  recovery_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/node-b-control-loop.preserves)
                  recovery_ref=''${recovery_summary#* ref=}
                  recovery_ref=''${recovery_ref%% *}
                  executable_put_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/service-job/target-executable-put.preserves)
                  executable_put_ref=''${executable_put_summary#* ref=}
                  executable_put_ref=''${executable_put_ref%% *}
                  input_put_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/service-job/target-input-put.preserves)
                  input_put_ref=''${input_put_summary#* ref=}
                  input_put_ref=''${input_put_ref%% *}
                  molten test prod-soak durability \
                    --scenario restart-durability \
                    --queued-control-ref "$restart_queue_ref" \
                    --recovery-ref "$recovery_ref" \
                    --ledger-ref "$job_ref" \
                    --chunk-ref "$executable_put_ref" \
                    --chunk-ref "$input_put_ref" \
                    --retention-ref "$job_ref" \
                    --retention-ref "$coordination_ref" \
                    --decision pass \
                    --caveat 'durability evidence is scoped to this VM topology and remains diagnostic' \
                    --out /var/lib/molten/vm-evidence/prod-soak-durability.preserves
                  durability_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/prod-soak-durability.preserves)
                  durability_ref=''${durability_summary#* ref=}
                  durability_ref=''${durability_ref%% *}
                  receipt_bytes=$(stat -c%s "$fault_dir/matrix.preserves")
                  store_bytes=$(du -sb /var/lib/molten/vm-evidence | cut -f1)
                  molten test prod-soak resource-envelope \
                    --scenario pilot-resource-envelope \
                    --queue-depth 1 \
                    --max-queue-depth 8 \
                    --receipt-bytes "$receipt_bytes" \
                    --max-receipt-bytes 1000000 \
                    --store-bytes "$store_bytes" \
                    --max-store-bytes 100000000 \
                    --delivery-latency-ms 0 \
                    --max-delivery-latency-ms 60000 \
                    --recovery-time-ms 0 \
                    --max-recovery-time-ms 60000 \
                    --pressure-ref "$fault_matrix_ref" \
                    --denial-ref "$stale_denial_ref" \
                    --denial-ref "$wrong_authority_denial_ref" \
                    --decision pass \
                    --caveat 'resource metrics are single-VM pilot bounds and are not SLO evidence' \
                    --out /var/lib/molten/vm-evidence/prod-soak-resource-envelope.preserves
                  resource_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/prod-soak-resource-envelope.preserves)
                  resource_ref=''${resource_summary#* ref=}
                  resource_ref=''${resource_ref%% *}
                  vm_fault_dir=/var/lib/molten/vm-evidence/vm-faults
                  mkdir -p "$vm_fault_dir"
                  fault_duration_millis=1000
                  printf '<vm-fault-injection "network-control-unavailable">\n' > "$vm_fault_dir/network-unavailable.preserves"
                  printf '%s\n' 'network control command unavailable in this VM image' > "$vm_fault_dir/network-unavailable.log"
                  molten test nixos-vm fault-descriptor \
                    --fault-id network-partition-node-a-node-b \
                    --topology /var/lib/molten/vm-evidence/topology.preserves \
                    --target-node node_a \
                    --target-link node_a-to-node_b \
                    --fault-kind network-partition \
                    --command-profile nixos-test-driver-network-control \
                    --expected-outcome unavailable \
                    --duration-millis "$fault_duration_millis" \
                    --trigger network-control-preflight \
                    --preflight /var/lib/molten/vm-evidence/topology.preserves \
                    --caveat 'network fault execution is unavailable unless VM image exposes network-control tools' \
                    --out "$vm_fault_dir/network-partition.descriptor.preserves"
                  molten test nixos-vm fault-receipt \
                    --descriptor "$vm_fault_dir/network-partition.descriptor.preserves" \
                    --decision unavailable \
                    --host-support unavailable \
                    --pre-fault /var/lib/molten/vm-evidence/topology.preserves \
                    --injection "$vm_fault_dir/network-unavailable.preserves" \
                    --post-fault /var/lib/molten/vm-evidence/node-evidence.preserves \
                    --replay-status unavailable-network-control \
                    --diagnostic 'network-control support unavailable; no pass evidence minted' \
                    --log "$vm_fault_dir/network-unavailable.log" \
                    --caveat 'unavailable VM fault evidence cannot satisfy pass claims' \
                    --out "$vm_fault_dir/network-partition.receipt.preserves"
                  molten test nixos-vm fault-descriptor \
                    --fault-id duplicate-send-after-restart \
                    --topology /var/lib/molten/vm-evidence/topology.preserves \
                    --target-node node_b \
                    --fault-kind duplicate-send-after-restart \
                    --command-profile systemd-restart-window \
                    --expected-outcome idempotent-recovery \
                    --duration-millis "$fault_duration_millis" \
                    --trigger queued-control-restart \
                    --preflight /var/lib/molten/vm-evidence/node-b-restart-queue.preserves \
                    --caveat 'restart fault evidence is scoped to this VM topology' \
                    --out "$vm_fault_dir/restart.descriptor.preserves"
                  molten test nixos-vm fault-receipt \
                    --descriptor "$vm_fault_dir/restart.descriptor.preserves" \
                    --decision pass \
                    --host-support supported \
                    --pre-fault /var/lib/molten/vm-evidence/node-b-restart-queue.preserves \
                    --injection /var/lib/molten/vm-evidence/node-b-control-loop.preserves \
                    --child /var/lib/molten/vm-evidence/node-b-control-loop.preserves \
                    --post-fault /var/lib/molten/vm-evidence/node-b-heartbeat.preserves \
                    --replay-status restart-window-observed \
                    --log /var/lib/molten/vm-evidence/run-loop.txt \
                    --caveat 'restart evidence demonstrates bounded idempotent recovery only' \
                    --out "$vm_fault_dir/restart.receipt.preserves"
                  printf '<vm-fault-injection "permission-denied-state-root-denial">\n' > "$vm_fault_dir/storage-denial.preserves"
                  printf '%s\n' 'permission denied before mutation fixture' > "$vm_fault_dir/storage-denial.log"
                  molten test nixos-vm fault-descriptor \
                    --fault-id permission-denied-state-root \
                    --topology /var/lib/molten/vm-evidence/topology.preserves \
                    --target-node node_b \
                    --fault-kind permission-denied-state-root \
                    --command-profile filesystem-state-root \
                    --expected-outcome deny-before-side-effects \
                    --duration-millis "$fault_duration_millis" \
                    --trigger state-root-write-preflight \
                    --preflight /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                    --caveat 'storage fault evidence is bounded and diagnostic outside canonical receipts' \
                    --out "$vm_fault_dir/storage.descriptor.preserves"
                  molten test nixos-vm fault-receipt \
                    --descriptor "$vm_fault_dir/storage.descriptor.preserves" \
                    --decision deny \
                    --host-support supported \
                    --pre-fault /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                    --injection "$vm_fault_dir/storage-denial.preserves" \
                    --post-fault /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                    --replay-status denial-observed-before-mutation \
                    --diagnostic 'permission denied before mutation' \
                    --log "$vm_fault_dir/storage-denial.log" \
                    --caveat 'deny receipt is authoritative; logs are diagnostic only' \
                    --out "$vm_fault_dir/storage.receipt.preserves"
                  molten test nixos-vm fault-validate \
                    --topology /var/lib/molten/vm-evidence/topology.preserves \
                    --descriptor "$vm_fault_dir/network-partition.descriptor.preserves" \
                    --descriptor "$vm_fault_dir/restart.descriptor.preserves" \
                    --descriptor "$vm_fault_dir/storage.descriptor.preserves" \
                    --receipt "$vm_fault_dir/network-partition.receipt.preserves" \
                    --receipt "$vm_fault_dir/restart.receipt.preserves" \
                    --receipt "$vm_fault_dir/storage.receipt.preserves" \
                    --out "$vm_fault_dir/validation.preserves"
                  vm_fault_validation_summary=$(molten test nixos-vm show "$vm_fault_dir/validation.preserves")
                  vm_fault_validation_ref=''${vm_fault_validation_summary#* ref=}
                  vm_fault_validation_ref=''${vm_fault_validation_ref%% *}
                  molten test prod-soak evidence-export \
                    --node node_a \
                    --node-evidence /var/lib/molten/vm-evidence/node-evidence.preserves \
                    --artifact /var/lib/molten/vm-evidence/live-control/protocol-gate.preserves \
                    --artifact /var/lib/molten/vm-evidence/live-control/ack-export.preserves \
                    --artifact /var/lib/molten/vm-evidence/service-job/remote-deliver.preserves \
                    --artifact /var/lib/molten/vm-evidence/prod-soak-durability.preserves \
                    --artifact /var/lib/molten/vm-evidence/prod-soak-resource-envelope.preserves \
                    --log /var/lib/molten/vm-evidence/live-control/protocol-gate.txt \
                    --out /var/lib/molten/vm-evidence/prod-soak-node-a-export.preserves
                  molten test prod-soak evidence-export \
                    --node node_b \
                    --node-evidence /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                    --artifact /var/lib/molten/vm-evidence/service-job/job-ref.receipt.preserves \
                    --artifact /var/lib/molten/vm-evidence/service-job/coord-apply/report.preserves \
                    --artifact /var/lib/molten/vm-evidence/live-control/reconcile.preserves \
                    --log /var/lib/molten/vm-evidence/service-job/job-ref-execute.txt \
                    --out /var/lib/molten/vm-evidence/prod-soak-node-b-export.preserves
                  molten test prod-soak run-receipt \
                    --topology /var/lib/molten/vm-evidence/topology.preserves \
                    --node-evidence /var/lib/molten/vm-evidence/node-evidence.preserves \
                    --node-evidence /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                    --scenario production-shaped-vm-live-workflow \
                    --fault-profile network-transport-fault-matrix \
                    --peer-ticket-ref "$ticket_ref" \
                    --peer-ticket-ref "$peer_admission_ref" \
                    --peer-ticket-ref "$authority_ref" \
                    --node-control-ref "$protocol_ref" \
                    --node-control-ref "$reconcile_ref" \
                    --node-control-ref "$ack_ref" \
                    --remote-service-ref "$remote_ref" \
                    --job-ref "$job_ref" \
                    --coordination-ref "$coordination_ref" \
                    --fault-ref "$fault_matrix_ref" \
                    --durability-ref "$durability_ref" \
                    --resource-ref "$resource_ref" \
                    --evidence-export /var/lib/molten/vm-evidence/prod-soak-node-a-export.preserves \
                    --evidence-export /var/lib/molten/vm-evidence/prod-soak-node-b-export.preserves \
                    --log /var/lib/molten/vm-evidence/live-control/protocol-gate.txt \
                    --log /var/lib/molten/vm-evidence/service-job/remote-deliver.txt \
                    --log /var/lib/molten/vm-evidence/service-job/job-ref-execute.txt \
                    --log /var/lib/molten/vm-evidence/service-job/coord-apply.txt \
                    --log /var/lib/molten/vm-evidence/prod-soak-faults/stale-ticket-gate.txt \
                    --decision pass \
                    --replay-status non-replayable-live-observations \
                    --caveat 'soak evidence is pilot-scoped and diagnostic unless separately replayed' \
                    --caveat 'soak evidence does not grant authority, policy, resource, provenance, or source-gate trust' \
                    --out /var/lib/molten/vm-evidence/prod-soak-run.preserves
                  soak_summary=$(molten test prod-soak show /var/lib/molten/vm-evidence/prod-soak-run.preserves)
                  soak_ref=''${soak_summary#* ref=}
                  soak_ref=''${soak_ref%% *}
                  molten test nixos-vm run-receipt \
                    --topology /var/lib/molten/vm-evidence/topology.preserves \
                    --node-evidence /var/lib/molten/vm-evidence/node-evidence.preserves \
                    --node-evidence /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                    --scenario phase2-live-control-service-job-restart \
                    --fault-profile executable-vm-faults \
                    --child-ref "$protocol_ref" \
                    --child-ref "$reconcile_ref" \
                    --child-ref "$ack_ref" \
                    --child-ref "$remote_ref" \
                    --child-ref "$job_ref" \
                    --child-ref "$coordination_ref" \
                    --child-ref "$soak_ref" \
                    --child-ref "$vm_fault_validation_ref" \
                    --log /var/lib/molten/vm-evidence/live-control/protocol-gate.txt \
                    --log /var/lib/molten/vm-evidence/live-control/reconcile.txt \
                    --log /var/lib/molten/vm-evidence/live-control/live-loopback.txt \
                    --log /var/lib/molten/vm-evidence/service-job/remote-deliver.txt \
                    --log /var/lib/molten/vm-evidence/service-job/job-ref-execute.txt \
                    --log /var/lib/molten/vm-evidence/service-job/coord-apply.txt \
                    --decision pass \
                    --replay-status non-replayable-vm-observations \
                    --caveat 'vm observations are diagnostic unless separately replayed' \
                    --caveat 'vm evidence does not grant authority or policy trust' \
                    --out /var/lib/molten/vm-evidence/vm-test-run.preserves
                """)
                node_a.succeed("grep -q nixos-vm-topology-v1 /var/lib/molten/vm-evidence/topology.preserves")
                node_a.succeed("grep -q nixos-vm-node-evidence-v1 /var/lib/molten/vm-evidence/node-evidence.preserves")
                node_a.succeed("grep -q nixos-vm-test-run-v1 /var/lib/molten/vm-evidence/vm-test-run.preserves")
                node_a.succeed("grep -q prod-soak-run-v1 /var/lib/molten/vm-evidence/prod-soak-run.preserves")
                node_a.succeed("""
                  molten test nixos-vm validate \
                    --topology /var/lib/molten/vm-evidence/topology.preserves \
                    --node-evidence /var/lib/molten/vm-evidence/node-evidence.preserves \
                    --node-evidence /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                    --test-run /var/lib/molten/vm-evidence/vm-test-run.preserves \
                    --prod-soak /var/lib/molten/vm-evidence/prod-soak-run.preserves \
                    --expected-node node_a \
                    --expected-node node_b \
                    --out /var/lib/molten/vm-evidence/vm-evidence-validation.preserves
                  molten test nixos-vm manifest \
                    --root /var/lib/molten/vm-evidence \
                    --artifact /var/lib/molten/vm-evidence/topology.preserves \
                    --artifact /var/lib/molten/vm-evidence/node-evidence.preserves \
                    --artifact /var/lib/molten/vm-evidence/node-b-evidence.preserves \
                    --artifact /var/lib/molten/vm-evidence/vm-test-run.preserves \
                    --artifact /var/lib/molten/vm-evidence/prod-soak-run.preserves \
                    --artifact /var/lib/molten/vm-evidence/vm-evidence-validation.preserves \
                    --artifact /var/lib/molten/vm-evidence/vm-faults/network-partition.descriptor.preserves \
                    --artifact /var/lib/molten/vm-evidence/vm-faults/network-partition.receipt.preserves \
                    --artifact /var/lib/molten/vm-evidence/vm-faults/restart.descriptor.preserves \
                    --artifact /var/lib/molten/vm-evidence/vm-faults/restart.receipt.preserves \
                    --artifact /var/lib/molten/vm-evidence/vm-faults/storage.descriptor.preserves \
                    --artifact /var/lib/molten/vm-evidence/vm-faults/storage.receipt.preserves \
                    --artifact /var/lib/molten/vm-evidence/vm-faults/validation.preserves \
                    --log /var/lib/molten/vm-evidence/run.txt \
                    --log /var/lib/molten/vm-evidence/status.txt \
                    --log /var/lib/molten/vm-evidence/live-control/protocol-gate.txt \
                    --log /var/lib/molten/vm-evidence/service-job/job-ref-execute.txt \
                    --log /var/lib/molten/vm-evidence/vm-faults/network-unavailable.log \
                    --log /var/lib/molten/vm-evidence/vm-faults/storage-denial.log \
                    --caveat 'VM canonical receipts are authoritative; preserved logs are diagnostic only' \
                    --out /var/lib/molten/vm-evidence/vm-evidence-manifest.preserves
                  grep -q nixos-vm-evidence-validation-v1 /var/lib/molten/vm-evidence/vm-evidence-validation.preserves
                  grep -q nixos-vm-evidence-manifest-v1 /var/lib/molten/vm-evidence/vm-evidence-manifest.preserves
                  mkdir -p /tmp/molten-vm-output
                  cp -R /var/lib/molten/vm-evidence /tmp/molten-vm-output/
                """)
                import os
                out_dir = os.environ["out"]
                os.makedirs(out_dir, exist_ok=True)
                node_a.copy_from_vm("/tmp/molten-vm-output/vm-evidence", os.path.join(out_dir, "vm-evidence"))
              '';
            };

            nextest-config =
              pkgs.runCommand "molten-nextest-config-check"
                {
                  nativeBuildInputs = [
                    rustToolchain
                    pkgs.cargo-nextest
                    pkgs.gnugrep
                  ];
                  src = sourceForConfigChecks;
                }
                ''
                  set -euo pipefail
                  cp -R $src source
                  chmod -R u+w source
                  cd source
                  cargo nextest show-config version --user-config-file none --profile default > default.txt
                  cargo nextest show-config version --user-config-file none --profile ci > ci.txt
                  cargo nextest show-config version --user-config-file none --profile deterministic > deterministic.txt
                  cargo nextest show-config version --user-config-file none --profile exploratory > exploratory.txt
                  cargo nextest show-config version --user-config-file none --profile fast-core > fast-core.txt
                  cargo nextest show-config version --user-config-file none --profile harness > harness.txt
                  cargo nextest show-config version --user-config-file none --profile cli > cli.txt
                  cargo nextest show-config version --user-config-file none --profile distributed-simulation > distributed-simulation.txt
                  cargo nextest show-config version --user-config-file none --profile vm-platform > vm-platform.txt
                  cargo nextest show-config version --user-config-file none --profile dogfood-soak > dogfood-soak.txt
                  mkdir -p $out
                  cp default.txt ci.txt deterministic.txt exploratory.txt fast-core.txt harness.txt cli.txt distributed-simulation.txt vm-platform.txt dogfood-soak.txt .config/nextest.toml $out/
                  profiles='ci deterministic exploratory fast-core harness cli distributed-simulation vm-platform dogfood-soak'
                  profile_block_context=16
                  for profile in $profiles; do
                    grep -q "^\[profile.$profile\]" .config/nextest.toml
                    grep -A $profile_block_context "^\[profile.$profile\]" .config/nextest.toml | grep '^default-filter = ' > "$out/$profile-filter.txt"
                    grep -A $profile_block_context "^\[profile.$profile\]" .config/nextest.toml | grep '^junit = ' > "$out/$profile-junit-config.txt"
                  done
                  grep -A $profile_block_context '^\[profile.exploratory\]' .config/nextest.toml | grep '^retries = 1' > $out/exploratory-retry-policy.txt
                  grep -A $profile_block_context '^\[profile.exploratory\]' .config/nextest.toml | grep '^flaky-result = "pass"' > $out/exploratory-flaky-result.txt
                  printf 'nextest-profile-matrix decision=pass diagnostics=none\n' > $out/nextest-profile-matrix.txt
                  printf 'cargo nextest run --profile ci\n' > $out/ci-command.txt
                  printf 'cargo nextest run --profile deterministic\n' > $out/deterministic-command.txt
                  printf 'cargo nextest run --profile exploratory\n' > $out/exploratory-command.txt
                  printf 'cargo nextest run --profile fast-core\n' > $out/fast-core-command.txt
                  printf 'cargo nextest run --profile harness\n' > $out/harness-command.txt
                  printf 'cargo nextest run --profile cli\n' > $out/cli-command.txt
                  printf 'cargo nextest run --profile distributed-simulation\n' > $out/distributed-simulation-command.txt
                  printf 'cargo nextest run --profile vm-platform\n' > $out/vm-platform-command.txt
                  printf 'cargo nextest run --profile dogfood-soak\n' > $out/dogfood-soak-command.txt
                  printf 'target/nextest/ci/junit.xml\n' > $out/ci-junit-path.txt
                  printf 'target/nextest/deterministic/junit.xml\n' > $out/deterministic-junit-path.txt
                  printf 'target/nextest/exploratory/junit.xml\n' > $out/exploratory-junit-path.txt
                  printf 'target/nextest/fast-core/junit.xml\n' > $out/fast-core-junit-path.txt
                  printf 'target/nextest/harness/junit.xml\n' > $out/harness-junit-path.txt
                  printf 'target/nextest/cli/junit.xml\n' > $out/cli-junit-path.txt
                  printf 'target/nextest/distributed-simulation/junit.xml\n' > $out/distributed-simulation-junit-path.txt
                  printf 'target/nextest/vm-platform/junit.xml\n' > $out/vm-platform-junit-path.txt
                  printf 'target/nextest/dogfood-soak/junit.xml\n' > $out/dogfood-soak-junit-path.txt
                '';

            fmt =
              pkgs.runCommand "cargo-fmt-check"
                {
                  nativeBuildInputs = [ rustToolchain ];
                  src = ./.;
                }
                ''
                  cp -R $src source
                  chmod -R u+w source
                  cd source
                  cargo fmt --check
                  touch $out
                '';
          }
          // pkgs.lib.optionalAttrs (system == "x86_64-linux") {
            dev-function-profiling = fluxProfilerCli;
            verified-node-replication-pilot = verifiedNodeReplicationPilot.check;
          };

        apps = {
          default = {
            type = "app";
            program = "${moltenPkg}/bin/molten";
            meta = {
              description = "Run the Molten CLI";
            };
          };
          molten = {
            type = "app";
            program = "${moltenPkg}/bin/molten";
            meta = {
              description = "Run the Molten CLI";
            };
          };
          nextest-ci = {
            type = "app";
            program = "${nextestCi}/bin/molten-nextest-ci";
            meta = {
              description = "Run Molten's cargo-nextest CI profile";
            };
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            rustToolchain
            pkgs.pkg-config
            pkgs.clang
            pkgs.mold
          ];

          # CLI tools for the embedded languages/protocols used by molten.
          packages = [
            pkgs.steel
            pkgs.nickel
            pkgs.wasmtime
            pkgs.cargo-nextest
            pkgs.cargo-watch
            pkgs.rust-analyzer
            unit2nix.packages.${system}.unit2nix
          ]
          ++ pkgs.lib.optional (system == "x86_64-linux") fluxProfilerCli;

        };

        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
