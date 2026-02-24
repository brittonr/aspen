{
  description = "Aspen - Foundational orchestration layer for distributed systems";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-25.11";

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    # SNIX - Nix store implementation in Rust
    # Used for content-addressed storage and Nix binary cache integration
    snix-src = {
      url = "git+https://git.snix.dev/snix/snix.git?rev=8fe3bade2013befd5ca98aa42224fa2a23551559";
      flake = false;
    };

    # MicroVM.nix - Declarative NixOS microVMs for isolated dogfood testing
    # Uses Cloud Hypervisor for fast boot times (~125ms) with full kernel isolation
    microvm = {
      url = "github:astro/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Verus - Formal verification for Rust
    # Built from source to avoid rustup dependency
    verus-src = {
      url = "github:verus-lang/verus/release/0.2026.01.30.44ebdee";
      flake = false;
    };
  };

  nixConfig = {
    keepOutputs = true;
    max-jobs = "auto";
    builders = "";
    # ── Binary Caches ─────────────────────────────────────────────────
    # Cargo dependency artifacts (cargoArtifacts, nodeCargoArtifacts,
    # cliCargoArtifacts) are the most expensive derivations (~2000 crates).
    # Push them to a binary cache to save 10-30 min per CI run:
    #
    #   Option A — Cachix (hosted):
    #     $ cachix create aspen
    #     $ nix build .#cargoArtifacts .#nodeCargoArtifacts .#cliCargoArtifacts
    #     $ cachix push aspen ./result
    #
    #   Option B — Attic (self-hosted):
    #     $ attic cache create aspen
    #     $ nix build .#cargoArtifacts .#nodeCargoArtifacts .#cliCargoArtifacts
    #     $ attic push aspen ./result
    #     Then uncomment the substituter/key lines below.
    #
    extra-substituters = [
      "https://cache.nixos.org"
      # microvm.nix provides pre-built Cloud Hypervisor kernels and VM components
      "https://microvm.cachix.org"
      # Uncomment after setting up your project cache:
      # "https://aspen.cachix.org"           # Cachix
      # "https://cache.yourserver.com/aspen"  # Attic / Harmonia
    ];
    extra-trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "microvm.cachix.org-1:oXnBc6hRE3eX5rSYdRyMYXnfzcCxC7yKPTbZXALsqys="
      # Uncomment after setting up your project cache:
      # "aspen.cachix.org-1:XXXX..."
    ];
    # Network reliability
    connect-timeout = 30;
    download-attempts = 3;
    fallback = true;
    # Automatic garbage collection
    min-free = 5368709120; # 5GB - trigger GC when less than this free
    max-free = 10737418240; # 10GB - stop GC when this much free
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    advisory-db,
    rust-overlay,
    snix-src,
    microvm,
    verus-src,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pname = "aspen";
        lib = nixpkgs.lib;
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };

        rustToolChain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolChain;

        netwatchSrc = pkgs.fetchFromGitHub {
          owner = "n0-computer";
          repo = "net-tools";
          rev = "cd7aba545996781786b8168d49b876f0844ad3d7";
          hash = "sha256-FMuab/Disrd/l9nJFOZ7vQxE2KE98ZLvQvocbZELPPY=";
        };

        netwatchCommon = {
          pname = "netwatch";
          version = "0.12.0";
          src = netwatchSrc;
          cargoToml = "${netwatchSrc}/netwatch/Cargo.toml";
          cargoLock = "${netwatchSrc}/Cargo.lock";
        };

        netwatchCargoArtifacts = craneLib.buildDepsOnly (
          netwatchCommon
          // {
            doCheck = false;
          }
        );

        netwatch = craneLib.buildPackage (
          netwatchCommon
          // {
            cargoArtifacts = netwatchCargoArtifacts;
            cargoExtraArgs = "--bin netwatch";
            doCheck = false;
            postPatch = ''
                          mkdir -p netwatch/src/bin
                          cat <<'RS' > netwatch/src/bin/netwatch.rs
                          use netwatch::netmon::Monitor;
                          use n0_watcher::Watcher as _;

                          #[tokio::main(flavor = "current_thread")]
                          async fn main() -> Result<(), netwatch::netmon::Error> {
                              let monitor = Monitor::new().await?;
                              let mut subscriber = monitor.interface_state();
                              println!("netwatch: initial interface state -> {:#?}", subscriber.get());

                              loop {
                                  tokio::select! {
                                      update = subscriber.updated() => {
                                          match update {
                                              Ok(state) => println!("netwatch: interface state updated -> {:#?}", state),
                                              Err(_) => {
                                                  eprintln!("netwatch: watcher closed, exiting");
                                                  break;
                                              }
                                          }
                                      }
                                      res = tokio::signal::ctrl_c() => {
                                          if let Err(err) = res {
                                              eprintln!("netwatch: ctrl-c handler error: {err}");
                                          }
                                          break;
                                      }
                                  }
                              }
                              Ok(())
                          }
              RS
            '';
          }
        );

        # Use lib.fileset for precise source inclusion — only Rust/Cargo files
        # and vendored sources. Changes to docs, scripts, .agent/, etc. won't
        # trigger rebuilds.
        rawSrc = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./.cargo
            ./.config
            ./build.rs
            ./deny.toml
            ./rustfmt.toml
            ./rust-toolchain.toml
            ./.config
            ./src
            ./crates
            # ./openraft  # Extracted to ~/git/aspen-raft
            ./vendor
            ./tests
            ./benches
            ./examples
            ./scripts/tutorial-verify
          ];
        };

        # ── Full Source Assembly ──────────────────────────────────────
        # For VM integration tests we need real sibling repo sources
        # instead of stubs. builtins.path copies each repo to the Nix
        # store, filtering out .git/ and target/ to keep it lean.
        siblingFilter = path: _type:
          let
            baseName = builtins.baseNameOf path;
          in
            !(baseName == ".git" || baseName == "target" || baseName == ".direnv" || baseName == "result");

        # Resolve sibling repo root. Requires --impure since sibling
        # repos live outside the flake source tree. Set ASPEN_REPOS_DIR
        # to override the default (parent of CWD, assuming CWD is the
        # aspen checkout).
        # In pure evaluation (nix flake check without --impure),
        # builtins.getEnv always returns "".  We use this to gate
        # everything that touches sibling repos outside the flake tree.
        hasImpure = builtins.getEnv "PWD" != "";

        reposDir = let
          env = builtins.getEnv "ASPEN_REPOS_DIR";
          cwd = builtins.getEnv "PWD";
        in
          if env != ""
          then env
          else builtins.dirOf cwd;

        # Whether sibling repos are actually reachable.  True only in
        # impure mode AND when the repos directory exists on disk.
        hasSiblingRepos =
          hasImpure
          && builtins.pathExists (/. + "${reposDir}/aspen-cli");

        siblingRepo = name:
          builtins.path {
            path = /. + "${reposDir}/${name}";
            filter = siblingFilter;
            name = "aspen-sibling-${name}";
          };

        siblingRepoNames = [
          "aspen-automerge"
          "aspen-ci"
          "aspen-cli"
          "aspen-client-api"
          "aspen-cluster-types"
          "aspen-constants"
          "aspen-coordination"
          "aspen-dht-discovery"
          "aspen-disk"
          "aspen-dns"
          "aspen-docs"
          "aspen-federation"
          "aspen-forge"
          "aspen-fuse"
          "aspen-hlc"
          "aspen-hooks"
          "aspen-jobs"
          "aspen-kv-types"
          "aspen-layer"
          "aspen-nix"
          "aspen-pijul"
          "aspen-plugin-api"
          "aspen-plugins"
          "aspen-proxy"
          "aspen-raft"
          "aspen-rpc"
          "aspen-s3"
          "aspen-secrets"
          "aspen-sql"
          "aspen-storage-types"
          "aspen-testing-network"
          "aspen-ticket"
          "aspen-time"
          "aspen-traits"
          "aspen-tui"
          "aspen-verus-metrics"
          "aspen-wasm-guest-sdk"
          "aspen-wasm-plugin"
        ];

        # Shell snippet that copies all sibling repos into $DEPS/
        copySiblingRepos = lib.concatMapStringsSep "\n" (name:
          "cp -r ${siblingRepo name} \"$DEPS/${name}\""
        ) siblingRepoNames;

        # Sed expressions to rewrite ../aspen-X/ → .nix-deps/aspen-X/ (workspace level)
        # and ../../../aspen-X/ → ../../.nix-deps/aspen-X/ (crate level)
        rewriteWorkspacePaths = lib.concatMapStringsSep " \\\n            " (name:
          "-e 's|path = \"\\.\\./${name}/|path = \".nix-deps/${name}/|g'"
        ) siblingRepoNames;

        rewriteCratePaths = lib.concatMapStringsSep " \\\n            " (name:
          "-e 's|path = \"\\.\\./\\.\\./\\.\\./\\.\\./openraft/|path = \"../../.nix-deps/aspen-raft/openraft/|g' -e 's|path = \"\\.\\./\\.\\./\\.\\./${name}/|path = \"../../.nix-deps/${name}/|g'"
        ) siblingRepoNames;

        snixGitSource = ''source = "git+https://git.snix.dev/snix/snix.git?rev=8fe3bade2013befd5ca98aa42224fa2a23551559#8fe3bade2013befd5ca98aa42224fa2a23551559"'';
        src = pkgs.runCommand "aspen-src-patched" {} ''
          cp -r ${rawSrc} $out
          chmod -R u+w $out

          # Remove [patch.*] sections from cargo config for Nix builds
          if [ -f $out/.cargo/config.toml ]; then
            ${pkgs.gnused}/bin/sed -i '/^\[patch\./,$d' $out/.cargo/config.toml
          fi

          # Add git source lines to snix packages in Cargo.lock (idempotent)
          for pkg in nix-compat nix-compat-derive snix-castore snix-cli snix-store snix-tracing; do
            ${pkgs.gawk}/bin/awk -v pkg="$pkg" -v src='${snixGitSource}' '
              /^name = "/ && $0 ~ "\"" pkg "\"" { found=1 }
              found && /^version = "0.1.0"$/ {
                print
                if ((getline nextline) > 0) {
                  if (nextline !~ /^source = /) {
                    print src
                  }
                  print nextline
                } else {
                  print src
                }
                found=0
                next
              }
              { print }
            ' $out/Cargo.lock > $out/Cargo.lock.tmp && mv $out/Cargo.lock.tmp $out/Cargo.lock
          done

          # ── Stub crates for extracted sibling repos ─────────────────
          # These repos live at ../aspen-*/ locally but don't exist in the
          # Nix sandbox. Create minimal stub crates inside the source tree
          # at .nix-stubs/ and rewrite all path references to point there.
          stub_crate() {
            local dir="$1" name="$2"
            shift 2
            mkdir -p "$dir/src"
            {
              echo '[package]'
              echo "name = \"$name\""
              echo 'version = "0.1.0"'
              echo 'edition = "2024"'
              echo 'license = "AGPL-3.0-or-later"'
              echo '[features]'
              for feat in "$@"; do
                echo "$feat = []"
              done
            } > "$dir/Cargo.toml"
            echo "// stub for Nix builds" > "$dir/src/lib.rs"
          }

          STUBS="$out/.nix-stubs"

          # Round 1 extractions (ci, nix, tui, automerge)
          stub_crate "$STUBS/aspen-tui" "aspen-tui"
          stub_crate "$STUBS/aspen-automerge" "aspen-automerge"
          stub_crate "$STUBS/aspen-nickel" "aspen-nickel"
          stub_crate "$STUBS/aspen-ci" "aspen-ci" nickel shell-executor plugins-vm
          stub_crate "$STUBS/aspen-ci-core" "aspen-ci-core"
          stub_crate "$STUBS/aspen-ci-executor-shell" "aspen-ci-executor-shell"
          stub_crate "$STUBS/aspen-ci-executor-nix" "aspen-ci-executor-nix"
          stub_crate "$STUBS/aspen-ci-executor-vm" "aspen-ci-executor-vm"
          stub_crate "$STUBS/aspen-cache" "aspen-cache"
          stub_crate "$STUBS/aspen-snix" "aspen-snix"
          stub_crate "$STUBS/aspen-nix-cache-gateway" "aspen-nix-cache-gateway"
          stub_crate "$STUBS/aspen-nix-handler" "aspen-nix-handler" cache snix

          # Rounds 2-4 extractions
          stub_crate "$STUBS/aspen-layer" "aspen-layer"
          stub_crate "$STUBS/aspen-sql" "aspen-sql"
          stub_crate "$STUBS/aspen-dht-discovery" "aspen-dht-discovery" global-discovery
          stub_crate "$STUBS/aspen-coordination" "aspen-coordination" verus bolero
          stub_crate "$STUBS/aspen-coordination-protocol" "aspen-coordination-protocol"
          stub_crate "$STUBS/aspen-forge" "aspen-forge" git-bridge
          stub_crate "$STUBS/aspen-forge-protocol" "aspen-forge-protocol"
          stub_crate "$STUBS/aspen-secrets" "aspen-secrets"
          stub_crate "$STUBS/aspen-docs" "aspen-docs"
          stub_crate "$STUBS/aspen-hooks" "aspen-hooks"
          stub_crate "$STUBS/aspen-federation" "aspen-federation" global-discovery
          stub_crate "$STUBS/aspen-jobs" "aspen-jobs" blob plugins-vm plugins-wasm
          stub_crate "$STUBS/aspen-jobs-protocol" "aspen-jobs-protocol"
          stub_crate "$STUBS/aspen-jobs-worker-blob" "aspen-jobs-worker-blob"
          stub_crate "$STUBS/aspen-jobs-worker-maintenance" "aspen-jobs-worker-maintenance"
          stub_crate "$STUBS/aspen-jobs-worker-replication" "aspen-jobs-worker-replication"
          stub_crate "$STUBS/aspen-jobs-worker-shell" "aspen-jobs-worker-shell"
          stub_crate "$STUBS/aspen-jobs-worker-sql" "aspen-jobs-worker-sql"

          # Rewrite workspace-level path deps from sibling repos to .nix-stubs/
          ${pkgs.gnused}/bin/sed -i \
            -e 's|path = "\.\./aspen-tui/crates/aspen-tui"|path = ".nix-stubs/aspen-tui"|g' \
            -e 's|path = "\.\./aspen-automerge/crates/aspen-automerge"|path = ".nix-stubs/aspen-automerge"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-nickel"|path = ".nix-stubs/aspen-nickel"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci"|path = ".nix-stubs/aspen-ci"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci-core"|path = ".nix-stubs/aspen-ci-core"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci-executor-shell"|path = ".nix-stubs/aspen-ci-executor-shell"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci-executor-nix"|path = ".nix-stubs/aspen-ci-executor-nix"|g' \
            -e 's|path = "\.\./aspen-ci/crates/aspen-ci-executor-vm"|path = ".nix-stubs/aspen-ci-executor-vm"|g' \
            -e 's|path = "\.\./aspen-nix/crates/aspen-cache"|path = ".nix-stubs/aspen-cache"|g' \
            -e 's|path = "\.\./aspen-nix/crates/aspen-snix"|path = ".nix-stubs/aspen-snix"|g' \
            -e 's|path = "\.\./aspen-nix/crates/aspen-nix-cache-gateway"|path = ".nix-stubs/aspen-nix-cache-gateway"|g' \
            -e 's|path = "\.\./aspen-nix/crates/aspen-nix-handler"|path = ".nix-stubs/aspen-nix-handler"|g' \
            -e 's|path = "\.\./aspen-layer/crates/aspen-layer"|path = ".nix-stubs/aspen-layer"|g' \
            -e 's|path = "\.\./aspen-sql/crates/aspen-sql"|path = ".nix-stubs/aspen-sql"|g' \
            -e 's|path = "\.\./aspen-dht-discovery/crates/aspen-dht-discovery"|path = ".nix-stubs/aspen-dht-discovery"|g' \
            -e 's|path = "\.\./aspen-coordination/crates/aspen-coordination"|path = ".nix-stubs/aspen-coordination"|g' \
            -e 's|path = "\.\./aspen-coordination/crates/aspen-coordination-protocol"|path = ".nix-stubs/aspen-coordination-protocol"|g' \
            -e 's|path = "\.\./aspen-forge/crates/aspen-forge"|path = ".nix-stubs/aspen-forge"|g' \
            -e 's|path = "\.\./aspen-forge/crates/aspen-forge-protocol"|path = ".nix-stubs/aspen-forge-protocol"|g' \
            -e 's|path = "\.\./aspen-secrets/crates/aspen-secrets"|path = ".nix-stubs/aspen-secrets"|g' \
            -e 's|path = "\.\./aspen-docs/crates/aspen-docs"|path = ".nix-stubs/aspen-docs"|g' \
            -e 's|path = "\.\./aspen-hooks/crates/aspen-hooks"|path = ".nix-stubs/aspen-hooks"|g' \
            -e 's|path = "\.\./aspen-federation/crates/aspen-federation"|path = ".nix-stubs/aspen-federation"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs"|path = ".nix-stubs/aspen-jobs"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-protocol"|path = ".nix-stubs/aspen-jobs-protocol"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-blob"|path = ".nix-stubs/aspen-jobs-worker-blob"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-maintenance"|path = ".nix-stubs/aspen-jobs-worker-maintenance"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-replication"|path = ".nix-stubs/aspen-jobs-worker-replication"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-shell"|path = ".nix-stubs/aspen-jobs-worker-shell"|g' \
            -e 's|path = "\.\./aspen-jobs/crates/aspen-jobs-worker-sql"|path = ".nix-stubs/aspen-jobs-worker-sql"|g' \
            $out/Cargo.toml

          # Remove [patch."https://github.com/..."] from Cargo.toml
          # Keep [patch.crates-io] for cargo-hyperlight.
          ${pkgs.gnused}/bin/sed -i '/^\[patch\."https/,$d' $out/Cargo.toml

          # Strip 'layer' feature from aspen-core deps (aspen-layer is a stub in Nix).
          ${pkgs.gnused}/bin/sed -i 's|, features = \["layer"\]||g' \
            $out/Cargo.toml
            # $out/crates/aspen-cli/Cargo.toml  # Extracted to ~/git/aspen-cli
            # $out/crates/aspen-raft/Cargo.toml  # Extracted to ~/git/aspen-raft

          # Also rewrite crate-level path deps that point to sibling repos.
          # From crates/X/, "../../../repo/crates/Y" → "../../.nix-stubs/Y"
          find $out/crates -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
            -e 's|path = "\.\./\.\./\.\./aspen-ci/crates/aspen-ci"|path = "../../.nix-stubs/aspen-ci"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-ci/crates/aspen-nickel"|path = "../../.nix-stubs/aspen-nickel"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-nix/crates/aspen-cache"|path = "../../.nix-stubs/aspen-cache"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-nix/crates/aspen-nix-cache-gateway"|path = "../../.nix-stubs/aspen-nix-cache-gateway"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-nix/crates/aspen-nix-handler"|path = "../../.nix-stubs/aspen-nix-handler"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-layer/crates/aspen-layer"|path = "../../.nix-stubs/aspen-layer"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-sql/crates/aspen-sql"|path = "../../.nix-stubs/aspen-sql"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-dht-discovery/crates/aspen-dht-discovery"|path = "../../.nix-stubs/aspen-dht-discovery"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-coordination/crates/aspen-coordination"|path = "../../.nix-stubs/aspen-coordination"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-coordination/crates/aspen-coordination-protocol"|path = "../../.nix-stubs/aspen-coordination-protocol"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-forge/crates/aspen-forge"|path = "../../.nix-stubs/aspen-forge"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-forge/crates/aspen-forge-protocol"|path = "../../.nix-stubs/aspen-forge-protocol"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-secrets/crates/aspen-secrets"|path = "../../.nix-stubs/aspen-secrets"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-docs/crates/aspen-docs"|path = "../../.nix-stubs/aspen-docs"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-hooks/crates/aspen-hooks"|path = "../../.nix-stubs/aspen-hooks"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-federation/crates/aspen-federation"|path = "../../.nix-stubs/aspen-federation"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs"|path = "../../.nix-stubs/aspen-jobs"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-protocol"|path = "../../.nix-stubs/aspen-jobs-protocol"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-blob"|path = "../../.nix-stubs/aspen-jobs-worker-blob"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-maintenance"|path = "../../.nix-stubs/aspen-jobs-worker-maintenance"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-replication"|path = "../../.nix-stubs/aspen-jobs-worker-replication"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-shell"|path = "../../.nix-stubs/aspen-jobs-worker-shell"|g' \
            -e 's|path = "\.\./\.\./\.\./aspen-jobs/crates/aspen-jobs-worker-sql"|path = "../../.nix-stubs/aspen-jobs-worker-sql"|g' \
            {} \;

        '';

        basicArgs = {
          inherit src;
          inherit pname;
          strictDeps = true;

          # Use --offline because stub crates for extracted sibling repos
          # (.nix-stubs/) cause Cargo.lock drift. Cargo can update the
          # lockfile offline since all deps are vendored.
          cargoExtraArgs = "--offline";

          nativeBuildInputs = with pkgs; [
            git
            pkg-config
            lld # Linker for WASM targets
            protobuf # Protocol Buffers compiler for snix crates
            stdenv.cc.cc
            clang # Required by .cargo/config.toml linker setting
            mold # Fast linker used with clang
            autoPatchelfHook # Patch build scripts to find shared libs
          ];

          buildInputs = with pkgs; [
            openssl
            zlib # Required by hyperlight-host build script
            stdenv.cc.cc.lib # Provides libgcc_s.so.1
          ];

          # Ensure libraries are available for build scripts
          LD_LIBRARY_PATH = lib.makeLibraryPath [pkgs.zlib pkgs.stdenv.cc.cc.lib];

          # Set environment variable required by snix-build at compile time
          SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";

          # Set PROTO_ROOT for snix-castore build.rs to find proto files
          # Points to the snix source fetched as a flake input
          PROTO_ROOT = "${snix-src}";
        };

        # Vendor cargo dependencies with snix override
        # This substitutes git.snix.dev fetches with our snix-src flake input
        # Must use vendorCargoDeps directly to pass overrideVendorGitCheckout
        cargoVendorDir = craneLib.vendorCargoDeps {
          src = rawSrc;
          overrideVendorGitCheckout = ps: drv: let
            isSnixRepo =
              builtins.any (
                p:
                  builtins.isString (p.source or null)
                  && lib.hasPrefix "git+https://git.snix.dev/snix/snix.git" (p.source or "")
              )
              ps;
          in
            if isSnixRepo
            then drv.overrideAttrs (_old: {src = snix-src;})
            else drv;
        };

        # ── Cargo Dependency Artifacts ───────────────────────────────
        # Split by feature set so each build variant hits a warm cache.
        # Push all three to your binary cache for fastest CI.

        # Base cargo artifacts — deps only, no feature flags.
        # Used by builds without special features (TUI, CI agent).
        cargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            inherit cargoVendorDir;
          }
        );

        # Node-specific cargo artifacts — pre-compiles deps with the full
        # node feature set so aspen-node builds hit warm cache.
        nodeCargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            inherit cargoVendorDir;
            pnameSuffix = "-deps-node";
            cargoExtraArgs = "--offline --features ci,docs,hooks,shell-worker,automerge,secrets";
          }
        );

        # CLI-specific cargo artifacts — pre-compiles deps with the union
        # of all CLI feature flags so every CLI variant hits warm cache.
        cliCargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            inherit cargoVendorDir;
            pnameSuffix = "-deps-cli";
            cargoExtraArgs = "--offline -p aspen-cli --features forge,secrets,ci,automerge,proxy,sql";
          }
        );

        # Development-specific cargo artifacts that preserve incremental compilation
        # This maintains the Cargo target directory structure for faster rebuilds
        devCargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            inherit cargoVendorDir;
            # Keep intermediate artifacts for incremental compilation
            doInstallCargoArtifacts = true;
            # Enable incremental compilation in the dependency build
            CARGO_INCREMENTAL = "1";
          }
        );

        # Common arguments can be set here to avoid repeating them later
        commonArgs =
          basicArgs
          // {
            inherit cargoArtifacts cargoVendorDir;

            nativeBuildInputs =
              basicArgs.nativeBuildInputs
              ++ (with pkgs; [
                autoPatchelfHook
              ]);

            buildInputs =
              basicArgs.buildInputs
              ++ (lib.optionals pkgs.stdenv.buildPlatform.isDarwin (
                with pkgs; [
                  darwin.apple_sdk.frameworks.Security
                ]
              ));
          };

        # Development-specific arguments with incremental compilation enabled
        devArgs =
          commonArgs
          // {
            cargoArtifacts = devCargoArtifacts;
            # Enable incremental compilation
            CARGO_INCREMENTAL = "1";
            # Use more aggressive caching
            CARGO_BUILD_INCREMENTAL = "true";
          };

        # Node-specific arguments — uses nodeCargoArtifacts for faster node builds
        nodeCommonArgs =
          commonArgs
          // {
            cargoArtifacts = nodeCargoArtifacts;
          };

        # CLI-specific arguments — uses cliCommonArgs for faster CLI builds
        cliCommonArgs =
          commonArgs
          // {
            cargoArtifacts = cliCargoArtifacts;
          };

        # ── Full Source Builds (for VM integration tests) ───────────
        # Assembles the real workspace + all sibling repos into one tree.
        # Unlike the stub src, this produces working binaries because every
        # dep has real source code. The layout:
        #
        #   $out/               ← main aspen workspace
        #   $out/.nix-deps/     ← all sibling repos
        #   $out/.nix-deps/aspen → ..   (symlink back to workspace root)
        #
        # Path rewrites:
        #   Cargo.toml:  ../aspen-X/  → .nix-deps/aspen-X/
        #   crates/*/Cargo.toml:  ../../../aspen-X/  → ../../.nix-deps/aspen-X/
        fullSrc = pkgs.runCommand "aspen-full-src" {} ''
          cp -r ${rawSrc} $out
          chmod -R u+w $out

          # Remove [patch.*] sections from cargo config for Nix builds
          if [ -f $out/.cargo/config.toml ]; then
            ${pkgs.gnused}/bin/sed -i '/^\[patch\./,$d' $out/.cargo/config.toml
          fi

          # Add git source lines to snix packages in Cargo.lock
          for pkg in nix-compat nix-compat-derive snix-castore snix-cli snix-store snix-tracing; do
            ${pkgs.gawk}/bin/awk -v pkg="$pkg" -v src='${snixGitSource}' '
              /^name = "/ && $0 ~ "\"" pkg "\"" { found=1 }
              found && /^version = "0.1.0"$/ {
                print
                if ((getline nextline) > 0) {
                  if (nextline !~ /^source = /) {
                    print src
                  }
                  print nextline
                } else {
                  print src
                }
                found=0
                next
              }
              { print }
            ' $out/Cargo.lock > $out/Cargo.lock.tmp && mv $out/Cargo.lock.tmp $out/Cargo.lock
          done

          # ── Real sibling repos ──────────────────────────────────────
          DEPS="$out/.nix-deps"
          mkdir -p "$DEPS"

          ${copySiblingRepos}

          chmod -R u+w "$DEPS"

          # ── Add aspen-cli as a workspace member ─────────────────────
          cp -r "$DEPS/aspen-cli/crates/aspen-cli" "$out/crates/aspen-cli"
          chmod -R u+w "$out/crates/aspen-cli"
          ${pkgs.gnused}/bin/sed -i \
            '/^members = \[/a\    "crates/aspen-cli",' \
            $out/Cargo.toml
          # Rewrite aspen-cli's path deps
          ${pkgs.gnused}/bin/sed -i \
            -e 's|path = "\.\./\.\./\.\./aspen/|path = "../../|g' \
            ${rewriteCratePaths} \
            $out/crates/aspen-cli/Cargo.toml

          # Use pre-generated unified Cargo.lock that includes all sibling
          # repo deps. Regenerate with: cd $(nix build .#_debug-full-src --print-out-paths --impure)
          # then: cargo generate-lockfile && cp Cargo.lock <repo>/nix/full-build-Cargo.lock
          cp ${./nix/full-build-Cargo.lock} $out/Cargo.lock

          # ── Rewrite workspace-level paths ───────────────────────────
          ${pkgs.gnused}/bin/sed -i \
            ${rewriteWorkspacePaths} \
            $out/Cargo.toml

          # Remove [patch."https://github.com/..."] (keep [patch.crates-io])
          ${pkgs.gnused}/bin/sed -i '/^\[patch\."https/,$d' $out/Cargo.toml

          # ── Rewrite crate-level paths ───────────────────────────────
          find $out/crates -name Cargo.toml ! -path '*/aspen-cli/*' -exec ${pkgs.gnused}/bin/sed -i \
            ${rewriteCratePaths} \
            {} \;

          # ── Inline ALL workspace = true references in sibling repos ───
          # When the main workspace resolves a path dep from a sibling repo,
          # cargo resolves `workspace = true` against the MAIN workspace (not
          # the sibling's own). We must inline every `workspace = true` by
          # reading values from each sibling's workspace root.
          #
          # This handles both package-level fields (edition, version, etc.)
          # and dependency-level fields (dep = { workspace = true }).
          ${pkgs.bash}/bin/bash ${./nix/inline-workspace-deps.sh} "$DEPS"

          # ── Convert git deps to path deps ─────────────────────────────
          # After workspace inlining, some crates have git deps pointing at
          # the aspen GitHub repo. Convert these to path deps since the
          # actual crate source is already in the unified workspace.
          ${pkgs.python3}/bin/python3 ${./nix/git-to-path-deps.py} "$out"

          # ── Fix version mismatches between sibling repos ─────────────
          # Some sibling repos have outdated dep versions; bump to match
          # the main workspace's versions.
          find "$DEPS" -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
            -e 's|bolero = { version = "0.11"|bolero = { version = "0.13"|g' \
            -e 's|bolero-generator = { version = "0.11"|bolero-generator = { version = "0.13"|g' \
            -e 's|bolero = "0.11"|bolero = "0.13"|g' \
            -e 's|bolero-generator = "0.11"|bolero-generator = "0.13"|g' \
            {} \;

          # ── Rewrite sibling repos' back-references to the main workspace ─
          # Sibling→sibling paths (../aspen-X/) already resolve correctly
          # because they're all siblings under .nix-deps/. Only back-refs
          # to the main aspen workspace need fixing:
          #   workspace level: ../aspen/ → ../../       (up from .nix-deps/X/ to root)
          #   crate level:     ../../../aspen/ → ../../../../   (up from .nix-deps/X/crates/Y/ to root)
          find "$DEPS" -maxdepth 2 -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
            -e 's|path = "\.\./aspen/|path = "../../|g' \
            {} \;
          find "$DEPS" -mindepth 3 -name Cargo.toml -exec ${pkgs.gnused}/bin/sed -i \
            -e 's|path = "\.\./\.\./\.\./aspen/|path = "../../../../|g' \
            {} \;
        '';

        fullBasicArgs =
          basicArgs
          // {
            src = fullSrc;
            cargoToml = ./Cargo.toml;
            # No --offline: real sources match Cargo.lock accurately
            cargoExtraArgs = "";
          };

        fullCargoVendorDir = craneLib.vendorCargoDeps {
          src = fullSrc;
          overrideVendorGitCheckout = ps: drv: let
            isSnixRepo =
              builtins.any (
                p:
                  builtins.isString (p.source or null)
                  && lib.hasPrefix "git+https://git.snix.dev/snix/snix.git" (p.source or "")
              )
              ps;
          in
            if isSnixRepo
            then drv.overrideAttrs (_old: {src = snix-src;})
            else drv;
        };

        fullNodeCargoArtifacts = craneLib.buildDepsOnly (
          fullBasicArgs
          // {
            cargoVendorDir = fullCargoVendorDir;
            pnameSuffix = "-full-deps-node";
            # Build deps for the union of all VM-test feature sets
            # (excludes plugins-rpc which needs special hyperlight handling)
            cargoExtraArgs = "--features ci,docs,hooks,shell-worker,automerge,secrets,proxy";
          }
        );

        fullCommonArgs =
          fullBasicArgs
          // {
            cargoArtifacts = fullNodeCargoArtifacts;
            cargoVendorDir = fullCargoVendorDir;

            nativeBuildInputs =
              basicArgs.nativeBuildInputs
              ++ (with pkgs; [
                autoPatchelfHook
              ]);

            buildInputs =
              basicArgs.buildInputs
              ++ (lib.optionals pkgs.stdenv.buildPlatform.isDarwin (
                with pkgs; [
                  darwin.apple_sdk.frameworks.Security
                ]
              ));
          };

        fullBin = {
          name,
          features ? [],
        }:
          craneLib.buildPackage (
            fullCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
              cargoExtraArgs =
                "--bin ${name}"
                + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
              doCheck = false;
            }
          );

        # ── Full CLI builds ────────────────────────────────────────
        # Build aspen-cli from the unified fullSrc workspace. aspen-cli is
        # added as a workspace member so all workspace = true deps resolve
        # correctly against the main workspace.
        fullCliBin = features:
          craneLib.buildPackage (
            fullCommonArgs
            // {
              pname = "aspen-cli";
              version = "0.1.0";
              cargoExtraArgs =
                "-p aspen-cli --bin aspen-cli"
                + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
              doCheck = false;
            }
          );

        # ── Hyperlight WASM Plugin Support ──────────────────────────────
        # The plugins-rpc feature requires hyperlight-wasm, whose build.rs
        # runs a nested cargo build that fails in Nix's sandbox (no network).
        # We work around this by:
        # 1. Pre-building the wasm_runtime binary in a separate derivation
        # 2. Patching hyperlight-wasm's build.rs to use the pre-built binary
        # 3. Building aspen-node with the patched vendor dir

        # Pre-build the hyperlight wasm_runtime ELF binary
        hyperlight-wasm-runtime = import ./nix/hyperlight-wasm-runtime.nix {
          inherit pkgs lib;
          rustToolChain = rustToolChain;
        };

        # Patched cargoVendorDir: modify hyperlight-wasm's build.rs to
        # accept HYPERLIGHT_WASM_RUNTIME env var (skips nested cargo build)
        pluginsCargoVendorDir = pkgs.runCommand "patched-vendor-for-plugins" {} ''
                    # Deep-copy vendor dir, resolving symlinks so we can modify files
                    cp -rL --no-preserve=mode ${cargoVendorDir} $out

                    # Update config.toml to point to our local copies instead of
                    # the original Nix store paths (which have unpatched files)
                    ${pkgs.gnused}/bin/sed -i "s|${cargoVendorDir}|$out|g" $out/config.toml

                    # Find the hyperlight-wasm crate in the vendor registry
                    HLW_DIR=$(find $out -maxdepth 3 -type d -name "hyperlight-wasm-0.12.0" | head -1)
                    if [ -z "$HLW_DIR" ]; then
                      echo "ERROR: hyperlight-wasm-0.12.0 not found in vendor dir"
                      echo "Contents:"
                      find $out -maxdepth 3 -type d | head -20
                      exit 1
                    fi
                    echo "Found hyperlight-wasm at: $HLW_DIR"

                    # Replace the entire main() function with one that uses a pre-built
                    # wasm_runtime binary (avoiding the nested cargo build entirely, which
                    # also avoids the cargo-hyperlight dependency issue)
                    cat > "$HLW_DIR/build.rs" << 'BUILDRS'
          use std::path::{Path, PathBuf};
          use std::{env, fs};
          use anyhow::Result;
          use built::write_built_file;

          fn main() -> Result<()> {
              let wasm_runtime_resource = PathBuf::from(
                  env::var("HYPERLIGHT_WASM_RUNTIME")
                      .expect("HYPERLIGHT_WASM_RUNTIME must be set for Nix builds")
              );

              println!("cargo:warning=Using pre-built wasm_runtime from {}", wasm_runtime_resource.display());

              let out_dir = env::var_os("OUT_DIR").unwrap();
              let dest_path = Path::new(&out_dir).join("wasm_runtime_resource.rs");
              let contents = format!(
                  "pub (super) static WASM_RUNTIME: [u8; include_bytes!({name:?}).len()] = *include_bytes!({name:?});",
                  name = wasm_runtime_resource.as_os_str()
              );
              fs::write(dest_path, contents).unwrap();

              let wasm_runtime_bytes = fs::read(&wasm_runtime_resource).unwrap();
              let elf = goblin::elf::Elf::parse(&wasm_runtime_bytes).unwrap();

              let section_name = ".note_hyperlight_metadata";
              let wasmtime_version_number = if let Some(header) = elf.section_headers.iter().find(|hdr| {
                  elf.shdr_strtab.get_at(hdr.sh_name).map_or(false, |name| name == section_name)
              }) {
                  let start = header.sh_offset as usize;
                  let size = header.sh_size as usize;
                  let metadata_bytes = &wasm_runtime_bytes[start..start + size];
                  if let Some(null_pos) = metadata_bytes.iter().position(|&b| b == 0) {
                      std::str::from_utf8(&metadata_bytes[..null_pos]).unwrap()
                  } else {
                      std::str::from_utf8(metadata_bytes).unwrap()
                  }
              } else {
                  panic!(".note_hyperlight_metadata section not found in wasm_runtime binary");
              };

              write_built_file()?;

              let built_path = Path::new(&out_dir).join("built.rs");
              let mut file = std::fs::OpenOptions::new().create(false).append(true).open(built_path).unwrap();
              use std::io::Write;

              let metadata = fs::metadata(&wasm_runtime_resource).unwrap();
              let created = metadata.modified().unwrap();
              let created_datetime: chrono::DateTime<chrono::Local> = created.into();
              writeln!(file, "static WASM_RUNTIME_CREATED: &str = \"{}\";", created_datetime).unwrap();
              writeln!(file, "static WASM_RUNTIME_SIZE: &str = \"{}\";", metadata.len()).unwrap();
              writeln!(file, "static WASM_RUNTIME_WASMTIME_VERSION: &str = \"{}\";", wasmtime_version_number).unwrap();

              let hash = blake3::hash(&wasm_runtime_bytes);
              writeln!(file, "static WASM_RUNTIME_BLAKE3_HASH: &str = \"{}\";", hash).unwrap();

              println!("cargo:rerun-if-changed=build.rs");

              cfg_aliases::cfg_aliases! {
                  gdb: { all(feature = "gdb", debug_assertions) },
              }

              Ok(())
          }
          BUILDRS

                    # Verify the patch applied
                    grep -q "HYPERLIGHT_WASM_RUNTIME" "$HLW_DIR/build.rs" || {
                      echo "ERROR: build.rs patch failed"
                      cat "$HLW_DIR/build.rs" | head -20
                      exit 1
                    }

                    # Verify config.toml points to our local copy
                    grep -q "$out" "$out/config.toml" || {
                      echo "ERROR: config.toml not updated"
                      exit 1
                    }

                    echo "Patch applied successfully"
        '';

        # Cargo artifacts for plugin builds (uses patched vendor dir)
        # Must NOT inherit from cargoArtifacts — the patched build.rs
        # must be compiled fresh, not reused from the original cache.
        pluginsCargoArtifacts = craneLib.buildDepsOnly (
          basicArgs
          // {
            pname = "aspen-plugins";
            cargoVendorDir = pluginsCargoVendorDir;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
            cargoExtraArgs = "--features plugins-rpc";
          }
        );

        # Common args for plugins-enabled builds
        pluginsCommonArgs =
          basicArgs
          // {
            cargoVendorDir = pluginsCargoVendorDir;
            cargoArtifacts = pluginsCargoArtifacts;
            HYPERLIGHT_WASM_RUNTIME = "${hyperlight-wasm-runtime}/wasm_runtime";
            nativeBuildInputs =
              basicArgs.nativeBuildInputs
              ++ (with pkgs; [
                autoPatchelfHook
              ]);
            buildInputs =
              basicArgs.buildInputs
              ++ (lib.optionals pkgs.stdenv.buildPlatform.isDarwin (
                with pkgs; [
                  darwin.apple_sdk.frameworks.Security
                ]
              ));
          };

        # WASM plugins moved to ~/git/aspen-plugins repo
        # TODO: Add aspen-plugins flake input to restore VM test plugin builds

        # Build the main package
        aspen = craneLib.buildPackage (
          commonArgs
          // {
            inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
            doCheck = false;
          }
        );

        # Minimal busybox initramfs for VirtioFS integration testing.
        # Mounts virtiofs tag "testfs", writes/reads a file, prints PASS/FAIL,
        # then powers off. Used by cloud_hypervisor_virtiofs_test.rs.
        virtiofsTestInitrd = let
          kmod = pkgs.linuxPackages.kernel.modules;
          modDir = "${kmod}/lib/modules/${pkgs.linuxPackages.kernel.modDirVersion}/kernel";
          fuseDir = "${modDir}/fs/fuse";
          virtioDir = "${modDir}/drivers/virtio";
        in
          pkgs.runCommand "virtiofs-test-initrd" {
            nativeBuildInputs = with pkgs; [cpio gzip];
          } ''
                      mkdir -p root/{bin,dev,proc,sys,mnt,lib/modules}
                      cp ${pkgs.pkgsStatic.busybox}/bin/busybox root/bin/busybox
                      for cmd in sh mount umount echo insmod poweroff cat mkdir; do
                        ln -sf busybox root/bin/$cmd
                      done
                      # FUSE modules
                      cp ${fuseDir}/fuse.ko.xz root/lib/modules/
                      cp ${fuseDir}/virtiofs.ko.xz root/lib/modules/
                      # Virtio transport modules (virtiofs depends on virtio_ring, virtio, virtio_pci)
                      cp ${virtioDir}/virtio.ko.xz root/lib/modules/
                      cp ${virtioDir}/virtio_ring.ko.xz root/lib/modules/
                      cp ${virtioDir}/virtio_pci_modern_dev.ko.xz root/lib/modules/
                      cp ${virtioDir}/virtio_pci_legacy_dev.ko.xz root/lib/modules/
                      cp ${virtioDir}/virtio_pci.ko.xz root/lib/modules/
                      cat > root/init << 'INITEOF'
            #!/bin/sh
            mount -t proc none /proc
            mount -t sysfs none /sys
            mount -t devtmpfs none /dev
            # Load virtio transport stack (order matters: deps first)
            insmod /lib/modules/virtio.ko.xz
            insmod /lib/modules/virtio_ring.ko.xz
            insmod /lib/modules/virtio_pci_modern_dev.ko.xz
            insmod /lib/modules/virtio_pci_legacy_dev.ko.xz
            insmod /lib/modules/virtio_pci.ko.xz
            # Load FUSE + virtiofs
            insmod /lib/modules/fuse.ko.xz
            insmod /lib/modules/virtiofs.ko.xz
            # Wait briefly for virtio-fs PCI device to be probed
            sleep 1
            mount -t virtiofs testfs /mnt
            if [ $? -ne 0 ]; then
              echo "VIRTIOFS_TEST_FAIL: mount failed"
              poweroff -f
            fi
            echo "hello from guest" > /mnt/result.txt
            READBACK=$(cat /mnt/result.txt)
            if [ "$READBACK" = "hello from guest" ]; then
              echo "VIRTIOFS_TEST_PASS"
            else
              echo "VIRTIOFS_TEST_FAIL: got '$READBACK'"
            fi
            poweroff -f
            INITEOF
                      chmod +x root/init
                      cd root && find . | cpio --quiet -H newc -o | gzip -9 > $out
          '';

        # Helper script for setting up VM test environment
        vm-test-setup = pkgs.writeShellScriptBin "aspen-vm-setup" ''
          #!/usr/bin/env bash
          set -e

          echo "Setting up Cloud Hypervisor test environment..."

          # Check for KVM support
          if [ ! -e /dev/kvm ]; then
            echo "ERROR: KVM not available. Please ensure KVM is enabled."
            exit 1
          fi

          # Create network bridge if it doesn't exist
          if ! ip link show aspen-br0 &>/dev/null; then
            echo "Creating network bridge aspen-br0..."
            sudo ip link add aspen-br0 type bridge
            sudo ip addr add 10.100.0.1/24 dev aspen-br0
            sudo ip link set aspen-br0 up
            echo "Bridge created successfully."
          else
            echo "Bridge aspen-br0 already exists."
          fi

          # Create TAP devices for testing (up to 10)
          for i in {0..9}; do
            if ! ip link show aspen-tap$i &>/dev/null; then
              echo "Creating TAP device aspen-tap$i..."
              sudo ip tuntap add aspen-tap$i mode tap
              sudo ip link set aspen-tap$i master aspen-br0
              sudo ip link set aspen-tap$i up
            fi
          done

          echo "Setup complete! You can now run VM tests."
          echo ""
          echo "Quick start:"
          echo "  cloud-hypervisor --api-socket /tmp/ch.sock"
          echo ""
          echo "Or use the aspen-vm-run helper to launch a test VM."
        '';

        # Helper script for running a test VM
        vm-test-run = pkgs.writeShellScriptBin "aspen-vm-run" ''
          #!/usr/bin/env bash
          set -e

          NODE_ID=''${1:-1}

          # Default paths (can be overridden with env vars)
          KERNEL=''${CH_KERNEL:-${pkgs.linuxPackages.kernel}/bzImage}
          INITRD=''${CH_INITRD:-}
          DISK=''${CH_DISK:-/tmp/aspen-node-$NODE_ID.img}

          # Create a simple disk if it doesn't exist
          if [ ! -f "$DISK" ]; then
            echo "Creating disk image at $DISK..."
            qemu-img create -f raw "$DISK" 1G
          fi

          # Calculate TAP device and MAC address based on node ID
          TAP_NUM=$((NODE_ID - 1))
          TAP_NAME="aspen-tap$TAP_NUM"
          MAC=$(printf "52:54:00:00:00:%02x" $NODE_ID)

          # Cloud Hypervisor creates its own TAP devices
          # If a TAP with the same name exists, it will fail
          # So we either remove it first or use a unique name

          # Option 1: Remove existing TAP if it exists (requires sudo)
          if ip link show "$TAP_NAME" &>/dev/null; then
            echo "Note: TAP device $TAP_NAME already exists. Removing it..."
            sudo ip link del "$TAP_NAME" 2>/dev/null || true
          fi

          echo "Launching Cloud Hypervisor VM for node $NODE_ID..."
          echo "  Kernel: $KERNEL"
          echo "  Disk: $DISK"
          echo "  Network: TAP $TAP_NAME (will be created), MAC $MAC"
          echo ""
          echo "Starting VM (requires sudo for TAP creation)..."
          echo "Press Ctrl+C to stop the VM."
          echo ""

          # Cloud Hypervisor will create the TAP device
          sudo cloud-hypervisor \
            --kernel "$KERNEL" \
            --disk path="$DISK" \
            --cmdline "console=hvc0 root=/dev/vda1 rw init=/bin/sh" \
            --cpus boot=2 \
            --memory size=512M \
            --net "tap=$TAP_NAME,mac=$MAC,ip=10.100.0.$((NODE_ID+1))/24" \
            --serial tty \
            --console off \
            --api-socket /tmp/ch-node-$NODE_ID.sock
        '';

        # ==================================================================
        # Verus Formal Verification Tool
        # ==================================================================
        # Build Z3 4.12.5 from source (Verus requires this exact version)
        z3_4_12_5 = pkgs.z3.overrideAttrs (old: rec {
          version = "4.12.5";
          src = pkgs.fetchFromGitHub {
            owner = "Z3Prover";
            repo = "z3";
            rev = "z3-${version}";
            hash = "sha256-Qj9w5s02OSMQ2qA7HG7xNqQGaUacA1d4zbOHynq5k+A=";
          };
        });

        # Rust toolchain for Verus (requires Rust 1.93.0 with rustc-dev and llvm-tools)
        verusRustToolchain = pkgs.rust-bin.stable."1.93.0".default.override {
          extensions = ["rustc-dev" "llvm-tools" "rust-src"];
        };

        # Verus development shell - provides all dependencies for building/running Verus
        # Verus has a complex build system (vargo) with git dependencies that makes
        # direct Nix packaging difficult. This shell provides the environment
        # needed to build Verus from source or run pre-built binaries.
        verusDevShell = pkgs.mkShell {
          name = "verus-dev";
          nativeBuildInputs = with pkgs; [
            verusRustToolchain
            z3_4_12_5
            pkg-config
            openssl
            git
          ];
          shellHook = ''
            export VERUS_Z3_PATH="${z3_4_12_5}/bin/z3"
            echo "Verus development environment"
            echo "  Z3: ${z3_4_12_5}/bin/z3"
            echo "  Rust: 1.93.0 with rustc-dev, llvm-tools, rust-src"
            echo ""
            echo "To build Verus from source:"
            echo "  git clone https://github.com/verus-lang/verus"
            echo "  cd verus"
            echo "  source tools/activate"
            echo "  cd source && vargo build --release"
          '';
        };

        # Fetch pre-built Verus binaries from GitHub releases
        # Verus has a complex build system (vargo) that makes direct Nix packaging difficult,
        # but the binary releases work perfectly with proper library path setup.
        verusRoot = pkgs.stdenv.mkDerivation {
          pname = "verus-root";
          version = "0.2026.01.30";

          src = pkgs.fetchzip {
            url = "https://github.com/verus-lang/verus/releases/download/release%2F0.2026.01.30.44ebdee/verus-0.2026.01.30.44ebdee-x86-linux.zip";
            hash = "sha256-oRStgYjHmQS4zZ4iBpSRrpaS1/dMV65RdFKOeVqejUo=";
            stripRoot = false;
          };

          nativeBuildInputs = [pkgs.autoPatchelfHook];
          buildInputs = [pkgs.stdenv.cc.cc.lib pkgs.zlib];

          # Rust libs come from verusRustToolchain at runtime, so ignore these
          autoPatchelfIgnoreMissingDeps = ["librustc_driver*" "libLLVM*" "libstd-*"];

          installPhase = ''
            mkdir -p $out/bin $out/lib
            # The zip extracts with verus-x86-linux subdirectory - copy its contents to root
            cp -r verus-x86-linux/* $out/
            # Ensure binaries are executable
            chmod +x $out/rust_verify $out/verus $out/cargo-verus $out/z3 2>/dev/null || true
          '';
        };

        # Verus wrapper that sets up the environment correctly
        # The rust_verify binary needs librustc_driver from Rust 1.93.0
        # We use rust_verify directly since the 'verus' binary expects rustup
        verus = pkgs.writeShellScriptBin "verus" ''
          # Set library path for Rust 1.93.0 compiler libs (provides librustc_driver)
          export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

          # Set Z3 path to use bundled Z3 from the release
          export VERUS_Z3_PATH="${verusRoot}/z3"

          # Set VERUS_ROOT so rust_verify can find vstd and builtin libraries
          export VERUS_ROOT="${verusRoot}"

          # Call rust_verify directly (the verus binary expects rustup, which we don't have)
          exec "${verusRoot}/rust_verify" "$@"
        '';

        bins = let
          bin = {
            name,
            features ? [],
          }:
            craneLib.buildPackage (
              nodeCommonArgs
              // {
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs =
                  "--bin ${name}"
                  + lib.optionalString (features != []) " --features ${lib.concatStringsSep "," features}";
                doCheck = false;
              }
            );

          # aspen-tui extracted to ~/git/aspen-tui

          # aspen-cli extracted to ~/git/aspen-cli - builds commented out
          # Build aspen-cli from its own crate
          # aspen-cli-crate = craneLib.buildPackage (
          #   cliCommonArgs
          #   // {
          #     inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
          #     cargoExtraArgs = "--package aspen-cli --bin aspen-cli";
          #     doCheck = false;
          #   }
          # );

          # Build aspen-cli with forge features (push, signed tags, export-key)
          aspen-cli-forge-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features forge";
              doCheck = false;
            }
          );

          # Build aspen-cli with secrets features (KV, Transit, PKI)
          # Must include ci,automerge to match node's aspen-client-api feature set,
          # otherwise postcard enum discriminants are misaligned and responses
          # deserialize to wrong variants.
          aspen-cli-secrets-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features secrets,ci,automerge";
              doCheck = false;
            }
          );

          # Build aspen-cli with plugin management features
          # Must include ci,automerge to match node's aspen-client-api enum layout
          # (postcard uses variant indices; cfg-gated variants shift indices)
          aspen-cli-plugins-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features plugins-rpc,ci,automerge";
              doCheck = false;
            }
          );

          # Build aspen-cli with full features for comprehensive testing
          # Must include ci to match node's aspen-client-api enum layout
          aspen-cli-full-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features automerge,sql,ci";
              doCheck = false;
            }
          );

          # Build aspen-cli with CI features (CI pipelines + Nix cache)
          aspen-cli-ci-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features ci";
              doCheck = false;
            }
          );

          # Build aspen-cli with proxy features (TCP tunnel + HTTP forward proxy)
          aspen-cli-proxy-crate = craneLib.buildPackage (
            cliCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./crates/aspen-cli/Cargo.toml;}) pname version;
              cargoExtraArgs = "--package aspen-cli --bin aspen-cli --features proxy";
              doCheck = false;
            }
          );

          # Build aspen-node with proxy support (default features + proxy)
          aspen-node-proxy = bin {
            name = "aspen-node";
            features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets" "proxy"];
          };

          # Build aspen-node with WASM plugin support (plugins-rpc)
          # Uses pre-built hyperlight wasm_runtime to avoid nested cargo build
          aspen-node-plugins = craneLib.buildPackage (
            pluginsCommonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
              cargoExtraArgs = "--bin aspen-node --features ci,docs,hooks,shell-worker,automerge,secrets,plugins-rpc";
              doCheck = false;
            }
          );

          # aspen-ci extracted to ~/git/aspen-ci

          # aspen-verus-metrics moved to ~/git/aspen-verus-metrics repo

          bins =
            builtins.listToAttrs (
              map ({name, ...} @ package: lib.nameValuePair name (bin package)) [
                {
                  name = "aspen-node";
                  # Required features: jobs, docs, blob, hooks (see Cargo.toml required-features)
                  # ci: Includes jobs+blob via ci-basic, adds SNIX for artifact upload
                  # docs: iroh-docs CRDT document operations
                  # hooks: Event-driven automation (requires jobs)
                  # shell-worker: Execute shell commands for CI shell jobs
                  # Note: "plugins" excluded — hyperlight-wasm build.rs needs network access
                  # which is unavailable in Nix's sandbox.
                  features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets"];
                }
                {
                  name = "git-remote-aspen";
                  features = ["git-bridge"];
                }
              ]
            )
            // {
              # aspen-tui extracted to ~/git/aspen-tui
              aspen-cli = pkgs.writeShellScriptBin "aspen-cli" "exit 1";
              aspen-cli-forge = pkgs.writeShellScriptBin "aspen-cli" "exit 1";
              aspen-cli-plugins = pkgs.writeShellScriptBin "aspen-cli" "exit 1";
              aspen-cli-secrets = pkgs.writeShellScriptBin "aspen-cli" "exit 1";
              aspen-cli-full = pkgs.writeShellScriptBin "aspen-cli" "exit 1";
              aspen-cli-ci = pkgs.writeShellScriptBin "aspen-cli" "exit 1";
              aspen-cli-proxy = pkgs.writeShellScriptBin "aspen-cli" "exit 1";
              # aspen-cli extracted to ~/git/aspen-cli - all variants commented out
              # aspen-cli = aspen-cli-crate;
              # aspen-cli-forge = aspen-cli-forge-crate;
              # aspen-cli-plugins = aspen-cli-plugins-crate;
              # aspen-cli-secrets = aspen-cli-secrets-crate;
              # aspen-cli-full = aspen-cli-full-crate;
              # aspen-cli-ci = aspen-cli-ci-crate;  # Extracted
              # aspen-cli-proxy = aspen-cli-proxy-crate;  # Extracted
              inherit aspen-node-proxy aspen-node-plugins;
              # aspen-ci-agent extracted to ~/git/aspen-ci
              inherit hyperlight-wasm-runtime;

            }
            # Full-source builds for VM integration tests (real sibling deps).
            # Only available with --impure since they need sibling repos.
            // lib.optionalAttrs hasSiblingRepos {
              full-aspen-node = fullBin {
                name = "aspen-node";
                features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets"];
              };
              full-aspen-node-proxy = fullBin {
                name = "aspen-node";
                features = ["ci" "docs" "hooks" "shell-worker" "automerge" "secrets" "proxy"];
              };
              full-aspen-cli = fullCliBin [];
              full-aspen-cli-forge = fullCliBin ["forge"];
              full-aspen-cli-plugins = fullCliBin ["plugins-rpc" "ci" "automerge"];
              full-aspen-cli-ci = fullCliBin ["ci"];
              full-aspen-cli-proxy = fullCliBin ["proxy"];
            };
        in
          bins
          // rec {
            default = bins.aspen-node;

            # Development builds with incremental compilation enabled
            # Use these for faster iteration during development
            dev-aspen-node = craneLib.buildPackage (
              devArgs
              // {
                inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
                cargoExtraArgs = "--bin aspen-node --features full";
                doCheck = false;
              }
            );

            # dev-aspen-tui extracted to ~/git/aspen-tui

            # Convenience alias for the most commonly used dev build
            dev = dev-aspen-node;
          };
      in
        {
          # Formatter
          formatter = pkgs.alejandra;

          # Set of checks that are run: `nix flake check`
          checks =
            {
              # NOTE: clippy and doc checks disabled — they compile the full
              # workspace which depends on extracted sibling repos (aspen-layer,
              # aspen-coordination, etc.) that are stubs in the Nix sandbox.
              # Run locally: cargo clippy --workspace -- -D warnings
              clippy = pkgs.runCommand "aspen-clippy-stub" {} ''
                echo "SKIPPED: clippy requires extracted sibling repo sources"
                echo "Run locally: cargo clippy --workspace -- -D warnings"
                touch $out
              '';

              doc = pkgs.runCommand "aspen-doc-stub" {} ''
                echo "SKIPPED: doc check requires extracted sibling repo sources"
                echo "Run locally: cargo doc --workspace --no-deps"
                touch $out
              '';

              # Custom fmt check using nightly rustfmt to support unstable features
              # in rustfmt.toml (imports_granularity, overflow_delimited_expr, etc.)
              fmt =
                pkgs.runCommand "aspen-fmt-check" {
                  nativeBuildInputs = [pkgs.rust-bin.nightly.latest.rustfmt pkgs.findutils];
                } ''
                  cd ${src}
                  find . -name '*.rs' \
                    -not -path './target/*' \
                    -not -path './openraft/*' \
                    -not -path './vendor/*' \
                    -exec rustfmt --edition 2024 --check {} +
                  touch $out
                '';

              # Deny check stubbed out — requires extracted sibling repo sources
              # Run locally: cargo deny check
              deny = pkgs.runCommand "aspen-deny-stub" {} ''
                echo "SKIPPED: cargo deny requires extracted sibling repo sources"
                echo "Run locally: cargo deny check"
                touch $out
              '';

              audit = craneLib.cargoAudit {
                inherit src;
                # Patch advisory-db: strip CVSS 4.0 lines which cargo-audit can't parse
                advisory-db = pkgs.runCommand "advisory-db-patched" {} ''
                  cp -r ${advisory-db} $out
                  chmod -R u+w $out
                  find $out -name '*.md' -exec \
                    ${pkgs.gnused}/bin/sed -i '/^cvss = "CVSS:4\./d' {} +
                '';
                # RUSTSEC-2023-0071: rsa crate timing sidechannel — no fix available,
                # transitive dep via ssh-key → aspen-forge
                cargoAuditExtraArgs = "--ignore RUSTSEC-2023-0071";
              };

              # Verus formal verification check
              # Verifies all standalone Verus specifications (Core + Raft + Coordination + Cluster + Transport)
              verus-check =
                pkgs.runCommand "aspen-verus-check" {
                  nativeBuildInputs = [verus z3_4_12_5];
                } ''
                  export VERUS_Z3_PATH="${z3_4_12_5}/bin/z3"
                  export VERUS_ROOT="${verusRoot}"
                  export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

                  echo "=== Verifying All Aspen Verus Specifications ==="

                  CORE_SPEC_DIR="${src}/crates/aspen-core/verus"
                  RAFT_SPEC_DIR="${src}/crates/aspen-raft/verus"
                  CLUSTER_SPEC_DIR="${src}/crates/aspen-cluster/verus"
                  TRANSPORT_SPEC_DIR="${src}/crates/aspen-transport/verus"

                  echo "[1/4] Verifying Core specifications..."
                  ${verusRoot}/rust_verify \
                    --crate-type=lib \
                    --rlimit 30 \
                    --time \
                    "$CORE_SPEC_DIR/lib.rs" \
                    || { echo "FAILED: Core Verus verification"; exit 1; }
                  echo "[PASS] Core specifications verified"

                  # NOTE: Raft verus specs extracted to ~/git/aspen-raft
                  # Verify via: cd ~/git/aspen-raft && nix run .#verify-verus
                  echo "[2/4] Raft specifications — SKIPPED (extracted to aspen-raft repo)"

                  echo "[3/4] Verifying Cluster specifications..."
                  ${verusRoot}/rust_verify \
                    --crate-type=lib \
                    --rlimit 30 \
                    --time \
                    "$CLUSTER_SPEC_DIR/lib.rs" \
                    || { echo "FAILED: Cluster Verus verification"; exit 1; }
                  echo "[PASS] Cluster specifications verified"

                  echo "[4/4] Verifying Transport specifications..."
                  ${verusRoot}/rust_verify \
                    --crate-type=lib \
                    --rlimit 30 \
                    --time \
                    "$TRANSPORT_SPEC_DIR/lib.rs" \
                    || { echo "FAILED: Transport Verus verification"; exit 1; }
                  echo "[PASS] Transport specifications verified"

                  echo "=== All specifications verified ==="
                  touch $out
                '';

              # Verus inline ghost code check
              # Verifies that production code with ghost annotations compiles with verus feature
              # NOTE: verus-inline-check disabled — requires aspen-layer (extracted
              # to sibling repo). Run locally: cargo check -p aspen-raft --features verus
              verus-inline-check = pkgs.runCommand "aspen-verus-inline-check" {} ''
                echo "SKIPPED: verus-inline-check requires extracted sibling repos (aspen-layer)"
                echo "Run locally: cargo check -p aspen-raft --features verus"
                touch $out
              '';

              # verus-sync-check: moved to ~/git/aspen-verus-metrics repo
            }
            // {
              # Run quick tests with cargo-nextest (for CI)
              # NOTE: nextest checks disabled — they compile the full workspace
              # which depends on extracted sibling repos that are stubs in Nix.
              # Run locally: cargo nextest run -P quick
              nextest-quick = pkgs.runCommand "aspen-nextest-quick-stub" {} ''
                echo "SKIPPED: nextest requires extracted sibling repo sources"
                echo "Run locally: cargo nextest run -P quick"
                touch $out
              '';

              nextest = pkgs.runCommand "aspen-nextest-stub" {} ''
                echo "SKIPPED: nextest requires extracted sibling repo sources"
                echo "Run locally: cargo nextest run"
                touch $out
              '';
            }
            // lib.optionalAttrs (system == "x86_64-linux" && hasSiblingRepos) {
              # NixOS VM integration tests — uses full-source builds with
              # real sibling repo sources (not stubs). Requires --impure
              # since sibling repos live outside the flake source tree.
              # Run individually: nix build .#checks.x86_64-linux.forge-cluster-test --impure
              # NixOS VM integration test for the Forge.
              # Spins up a cluster with real networking and exercises
              # every forge CLI command end-to-end.
              # Build: nix build .#checks.x86_64-linux.forge-cluster-test
              forge-cluster-test = import ./nix/tests/forge-cluster.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-forge;
              };

              # Multi-node cluster test: 3-node Raft consensus, replication,
              # leader failover, cross-node operations, and node rejoin.
              # Build: nix build .#checks.x86_64-linux.multi-node-cluster-test
              multi-node-cluster-test = import ./nix/tests/multi-node-cluster.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-forge;
              };

              # Multi-node KV test: write/read replication, CAS across nodes,
              # batch write replication, scan consistency, NOT_LEADER routing,
              # delete propagation, large value replication, failover survival.
              # Build: nix build .#checks.x86_64-linux.multi-node-kv-test
              multi-node-kv-test = import ./nix/tests/multi-node-kv.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # multi-node-coordination-test: disabled — WASM plugins moved to aspen-plugins repo
              # TODO: restore via aspen-plugins flake input

              # Multi-node blob test: cross-node blob retrieval, blobs from
              # different nodes, replication status, large blob replication,
              # protection across nodes, repair cycle, failover survival.
              # Build: nix build .#checks.x86_64-linux.multi-node-blob-test
              multi-node-blob-test = import ./nix/tests/multi-node-blob.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # KV operations test: CRUD, CAS/CAD, scan/pagination, batch ops,
              # binary data, large values, special characters.
              # Build: nix build .#checks.x86_64-linux.kv-operations-test
              kv-operations-test = import ./nix/tests/kv-operations.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # Blob operations test: add, get, has, list, protect/unprotect,
              # status, replication-status, delete, repair-cycle.
              # Build: nix build .#checks.x86_64-linux.blob-operations-test
              blob-operations-test = import ./nix/tests/blob-operations.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # coordination-primitives-test: disabled — WASM plugins moved to aspen-plugins repo
              # TODO: restore via aspen-plugins flake input

              # hooks-services-test: disabled — WASM plugins moved to aspen-plugins repo
              # TODO: restore via aspen-plugins flake input

              # ratelimit-verify-test: disabled — WASM plugins moved to aspen-plugins repo
              # TODO: restore via aspen-plugins flake input

              # Cluster management, docs namespace, peer, and verify test:
              # cluster (status, health, metrics, ticket, prometheus);
              # docs CRDT (set, get, delete, list); peer list;
              # verify (kv, blob, docs, all).
              # Build: nix build .#checks.x86_64-linux.cluster-docs-peer-test
              cluster-docs-peer-test = import ./nix/tests/cluster-docs-peer.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # Job queue and index management test: job submit, status,
              # list, cancel, purge; index list, show (built-in indexes).
              # Build: nix build .#checks.x86_64-linux.job-index-test
              job-index-test = import ./nix/tests/job-index.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli;
              };

              # automerge-sql-test: disabled — WASM plugins moved to aspen-plugins repo
              # TODO: restore via aspen-plugins flake input

              # Plugin CLI test: install, list, info, enable, disable, remove,
              # reload, manifest-based install, flag overrides, resource limits,
              # KV prefix configuration, reinstall/overwrite.
              # Build: nix build .#checks.x86_64-linux.plugin-cli-test
              plugin-cli-test = import ./nix/tests/plugin-cli.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-plugins;
              };

              # secrets-engine-test: disabled — WASM plugins moved to aspen-plugins repo
              # TODO: restore via aspen-plugins flake input

              # CI pipeline and Nix binary cache test: CI lifecycle
              # (run, status, list, cancel, watch, unwatch, output) and
              # cache operations (stats, query, download).
              # Build: nix build .#checks.x86_64-linux.ci-cache-test
              ci-cache-test = import ./nix/tests/ci-cache.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node;
                aspenCliPackage = bins.full-aspen-cli-ci;
              };

              # Pijul version control test: repository management
              # (init, list, info), channels (create, list, fork, delete,
              # info), working directory (init, add, status, record),
              # and checkout.
              # HTTP proxy test: TCP tunnel and HTTP forward proxy over
              # iroh QUIC. Two nodes — server (aspen-node with proxy) and
              # client (aspen-cli proxy commands). Tests tunnel creation,
              # multiple/concurrent requests, forward proxy, and restart.
              # Build: nix build .#checks.x86_64-linux.proxy-tunnel-test
              proxy-tunnel-test = import ./nix/tests/proxy-tunnel.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node-proxy;
                aspenCliPackage = bins.full-aspen-cli-proxy;
              };

              # Multi-node proxy test: 3-node Raft cluster with proxy
              # enabled + external client. Tests TCP tunneling through
              # each node, cross-node tunneling, proxy during KV ops,
              # HTTP forward proxy, concurrent streams through multiple
              # nodes, proxy surviving leader failover, restarted node
              # proxy, large payload transfer, and multi-target tunnels.
              # Build: nix build .#checks.x86_64-linux.multi-node-proxy-test
              multi-node-proxy-test = import ./nix/tests/multi-node-proxy.nix {
                inherit pkgs;
                aspenNodePackage = bins.full-aspen-node-proxy;
                aspenCliPackage = bins.full-aspen-cli-proxy;
              };
            };

          # Base apps available on all systems
          apps = {
            aspen-node = flake-utils.lib.mkApp {
              drv = bins.aspen-node;
              exePath = "/bin/aspen-node";
            };

            # aspen-tui extracted to ~/git/aspen-tui
            # aspen-cli extracted to ~/git/aspen-cli

            # aspen-cli app stubbed out (extracted to separate repo)
            aspen-cli = {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-cli-stub" ''
                echo "aspen-cli has been extracted to ~/git/aspen-cli"
                echo "Build it from that repository or use: nix run ~/git/aspen-cli"
                exit 1
              ''}";
            };

            # Verus formal verification (all specs: Core + Raft + Coordination + Cluster + Transport)
            # Usage: nix run .#verify-verus [command]
            # Commands: all (default), quick, core, raft, coordination, cluster, transport
            verify-verus = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus" ''
                set -e
                export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
                export VERUS_Z3_PATH="${verusRoot}/z3"
                export VERUS_ROOT="${verusRoot}"

                CORE_SPEC_DIR="crates/aspen-core/verus"
                RAFT_SPEC_DIR="crates/aspen-raft/verus"
                COORD_SPEC_DIR="crates/aspen-coordination/verus"
                CLUSTER_SPEC_DIR="crates/aspen-cluster/verus"
                TRANSPORT_SPEC_DIR="crates/aspen-transport/verus"
                CMD="''${1:-all}"

                case "$CMD" in
                  all|"")
                    echo "=== Verus Formal Verification (All) ==="
                    echo ""
                    echo "[1/5] Verifying Core specifications..."
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$CORE_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Core specifications verified"
                    echo ""
                    echo "[2/5] Verifying Raft specifications..."
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$RAFT_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Raft specifications verified"
                    echo ""
                    echo "[3/5] Verifying Coordination specifications..."
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$COORD_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Coordination specifications verified"
                    echo ""
                    echo "[4/5] Verifying Cluster specifications..."
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$CLUSTER_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Cluster specifications verified"
                    echo ""
                    echo "[5/5] Verifying Transport specifications..."
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$TRANSPORT_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Transport specifications verified"
                    echo ""
                    echo "=== All specifications verified ==="
                    ;;
                  core)
                    echo "=== Verus Formal Verification (Core) ==="
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$CORE_SPEC_DIR/lib.rs" || exit 1
                    echo "=== Core specifications verified ==="
                    ;;
                  raft)
                    echo "=== Verus Formal Verification (Raft) ==="
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$RAFT_SPEC_DIR/lib.rs" || exit 1
                    echo "=== Raft specifications verified ==="
                    ;;
                  coordination|coord)
                    echo "=== Verus Formal Verification (Coordination) ==="
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$COORD_SPEC_DIR/lib.rs" || exit 1
                    echo "=== Coordination specifications verified ==="
                    ;;
                  cluster)
                    echo "=== Verus Formal Verification (Cluster) ==="
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$CLUSTER_SPEC_DIR/lib.rs" || exit 1
                    echo "=== Cluster specifications verified ==="
                    ;;
                  transport)
                    echo "=== Verus Formal Verification (Transport) ==="
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$TRANSPORT_SPEC_DIR/lib.rs" || exit 1
                    echo "=== Transport specifications verified ==="
                    ;;
                  quick|check)
                    echo "Quick syntax check..."
                    ${verusRoot}/rust_verify --crate-type=lib --no-verify "$CORE_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Core syntax check"
                    ${verusRoot}/rust_verify --crate-type=lib --no-verify "$RAFT_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Raft syntax check"
                    ${verusRoot}/rust_verify --crate-type=lib --no-verify "$COORD_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Coordination syntax check"
                    ${verusRoot}/rust_verify --crate-type=lib --no-verify "$CLUSTER_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Cluster syntax check"
                    ${verusRoot}/rust_verify --crate-type=lib --no-verify "$TRANSPORT_SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Transport syntax check"
                    ;;
                  help|--help|-h)
                    echo "Usage: nix run .#verify-verus [command]"
                    echo ""
                    echo "Commands:"
                    echo "  all (default)    Verify all specifications (Core + Raft + Coordination + Cluster + Transport)"
                    echo "  core             Verify only Core specifications"
                    echo "  raft             Verify only Raft specifications"
                    echo "  coordination     Verify only Coordination specifications"
                    echo "  cluster          Verify only Cluster specifications"
                    echo "  transport        Verify only Transport specifications"
                    echo "  quick            Syntax check only (no proofs)"
                    echo "  help             Show this help message"
                    ;;
                  *)
                    echo "Unknown command: $CMD"
                    echo "Run 'nix run .#verify-verus help' for usage"
                    exit 1
                    ;;
                esac
              ''}";
            };

            # Verus formal verification for coordination lock only
            # Usage: nix run .#verify-verus-coordination [command]
            # Commands: all (default), quick
            verify-verus-coordination = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus-coordination" ''
                set -e
                export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
                export VERUS_Z3_PATH="${verusRoot}/z3"
                export VERUS_ROOT="${verusRoot}"

                SPEC_DIR="crates/aspen-coordination/verus"
                CMD="''${1:-all}"

                case "$CMD" in
                  all|"")
                    echo "=== Verus Formal Verification (Coordination Lock) ==="
                    echo "Verifying distributed lock specifications..."
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$SPEC_DIR/lib.rs" || exit 1
                    echo "=== Coordination specifications verified ==="
                    ;;
                  quick|check)
                    echo "Quick syntax check..."
                    ${verusRoot}/rust_verify --crate-type=lib --no-verify "$SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Syntax check"
                    ;;
                  help|--help|-h)
                    echo "Usage: nix run .#verify-verus-coordination [command]"
                    echo ""
                    echo "Commands:"
                    echo "  all (default)    Verify all coordination specifications"
                    echo "  quick            Syntax check only (no proofs)"
                    echo "  help             Show this help message"
                    ;;
                  *)
                    echo "Unknown command: $CMD"
                    echo "Run 'nix run .#verify-verus-coordination help' for usage"
                    exit 1
                    ;;
                esac
              ''}";
            };

            # Verus formal verification for core primitives only
            # Usage: nix run .#verify-verus-core [command]
            # Commands: all (default), quick
            verify-verus-core = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus-core" ''
                set -e
                export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
                export VERUS_Z3_PATH="${verusRoot}/z3"
                export VERUS_ROOT="${verusRoot}"

                SPEC_DIR="crates/aspen-core/verus"
                CMD="''${1:-all}"

                case "$CMD" in
                  all|"")
                    echo "=== Verus Formal Verification (Core Primitives) ==="
                    echo "Verifying core specifications (HLC, Allocator, Tuple, Directory, Index)..."
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$SPEC_DIR/lib.rs" || exit 1
                    echo "=== Core specifications verified ==="
                    ;;
                  quick|check)
                    echo "Quick syntax check..."
                    ${verusRoot}/rust_verify --crate-type=lib --no-verify "$SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Syntax check"
                    ;;
                  help|--help|-h)
                    echo "Usage: nix run .#verify-verus-core [command]"
                    echo ""
                    echo "Commands:"
                    echo "  all (default)    Verify all core specifications"
                    echo "  quick            Syntax check only (no proofs)"
                    echo "  help             Show this help message"
                    ;;
                  *)
                    echo "Unknown command: $CMD"
                    echo "Run 'nix run .#verify-verus-core help' for usage"
                    exit 1
                    ;;
                esac
              ''}";
            };

            # Verus formal verification for cluster specs only
            # Usage: nix run .#verify-verus-cluster [command]
            # Commands: all (default), quick
            verify-verus-cluster = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus-cluster" ''
                set -e
                export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
                export VERUS_Z3_PATH="${verusRoot}/z3"
                export VERUS_ROOT="${verusRoot}"

                SPEC_DIR="crates/aspen-cluster/verus"
                CMD="''${1:-all}"

                case "$CMD" in
                  all|"")
                    echo "=== Verus Formal Verification (Cluster) ==="
                    echo "Verifying cluster specifications (gossip, identity, trust, announcements)..."
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$SPEC_DIR/lib.rs" || exit 1
                    echo "=== Cluster specifications verified ==="
                    ;;
                  quick|check)
                    echo "Quick syntax check..."
                    ${verusRoot}/rust_verify --crate-type=lib --no-verify "$SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Syntax check"
                    ;;
                  help|--help|-h)
                    echo "Usage: nix run .#verify-verus-cluster [command]"
                    echo ""
                    echo "Commands:"
                    echo "  all (default)    Verify all cluster specifications"
                    echo "  quick            Syntax check only (no proofs)"
                    echo "  help             Show this help message"
                    ;;
                  *)
                    echo "Unknown command: $CMD"
                    echo "Run 'nix run .#verify-verus-cluster help' for usage"
                    exit 1
                    ;;
                esac
              ''}";
            };

            # Verus formal verification for transport specs only
            # Usage: nix run .#verify-verus-transport [command]
            # Commands: all (default), quick
            verify-verus-transport = {
              type = "app";
              program = "${pkgs.writeShellScript "verify-verus-transport" ''
                set -e
                export LD_LIBRARY_PATH="${verusRustToolchain}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
                export VERUS_Z3_PATH="${verusRoot}/z3"
                export VERUS_ROOT="${verusRoot}"

                SPEC_DIR="crates/aspen-transport/verus"
                CMD="''${1:-all}"

                case "$CMD" in
                  all|"")
                    echo "=== Verus Formal Verification (Transport) ==="
                    echo "Verifying transport specifications (connections, streams)..."
                    ${verusRoot}/rust_verify --crate-type=lib --rlimit 30 --time "$SPEC_DIR/lib.rs" || exit 1
                    echo "=== Transport specifications verified ==="
                    ;;
                  quick|check)
                    echo "Quick syntax check..."
                    ${verusRoot}/rust_verify --crate-type=lib --no-verify "$SPEC_DIR/lib.rs" || exit 1
                    echo "[PASS] Syntax check"
                    ;;
                  help|--help|-h)
                    echo "Usage: nix run .#verify-verus-transport [command]"
                    echo ""
                    echo "Commands:"
                    echo "  all (default)    Verify all transport specifications"
                    echo "  quick            Syntax check only (no proofs)"
                    echo "  help             Show this help message"
                    ;;
                  *)
                    echo "Unknown command: $CMD"
                    echo "Run 'nix run .#verify-verus-transport help' for usage"
                    exit 1
                    ;;
                esac
              ''}";
            };

            # Verus inline ghost code check
            # Verifies that ghost code in production files compiles correctly
            # Usage: nix run .#verus-inline-check
            verus-inline-check = {
              type = "app";
              program = "${pkgs.writeShellScript "verus-inline-check" ''
                set -e
                echo "=== Verus Inline Ghost Code Check ==="
                echo "Checking that ghost code compiles with verus feature..."
                echo ""

                # Check aspen-raft compiles with verus feature
                echo "[1/4] Checking aspen-raft (verus feature disabled)..."
                ${rustToolChain}/bin/cargo check -p aspen-raft --quiet || {
                  echo "[FAIL] aspen-raft failed to compile without verus feature"
                  exit 1
                }
                echo "[PASS] aspen-raft compiles without verus feature"

                echo "[2/4] Checking aspen-raft (verus feature enabled)..."
                ${rustToolChain}/bin/cargo check -p aspen-raft --features verus --quiet || {
                  echo "[FAIL] aspen-raft failed to compile with verus feature"
                  exit 1
                }
                echo "[PASS] aspen-raft compiles with verus feature"

                # Check aspen-coordination compiles with verus feature
                echo "[3/4] Checking aspen-coordination (verus feature disabled)..."
                ${rustToolChain}/bin/cargo check -p aspen-coordination --quiet || {
                  echo "[FAIL] aspen-coordination failed to compile without verus feature"
                  exit 1
                }
                echo "[PASS] aspen-coordination compiles without verus feature"

                echo "[4/4] Checking aspen-coordination (verus feature enabled)..."
                ${rustToolChain}/bin/cargo check -p aspen-coordination --features verus --quiet || {
                  echo "[FAIL] aspen-coordination failed to compile with verus feature"
                  exit 1
                }
                echo "[PASS] aspen-coordination compiles with verus feature"

                echo ""
                echo "=== All inline ghost code checks passed ==="
              ''}";
            };

            # verify-verus-sync and verus-coverage: moved to ~/git/aspen-verus-metrics repo

            # 3-node cluster launcher
            # Usage: nix run .#cluster
            # Environment variables:
            #   ASPEN_NODE_COUNT  - Number of nodes (default: 3)
            #   ASPEN_BASE_HTTP   - Base HTTP port (default: 21001)
            #   ASPEN_STORAGE     - Storage backend: inmemory, sqlite, redb (default: sqlite)
            cluster = let
              # Bundle scripts directory so lib/cluster-common.sh is available
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-cluster" ''
                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    # bins.aspen-cli  # Extracted to ~/git/aspen-cli
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.curl
                    pkgs.netcat
                    pkgs.gnugrep
                    pkgs.gnused
                  ]
                }:$PATH"
                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                exec ${scriptsDir}/cluster.sh "$@"
              ''}";
            };

            # kitty-cluster removed (script deleted, aspen-tui extracted)

            # Default: single development node with sensible defaults
            # Usage: nix run
            # This starts a single-node cluster ready for experimentation.
            # For production, use: nix run .#aspen-node -- --node-id 1 --cookie <secret>
            # For multi-node cluster: nix run .#cluster
            default = {
              type = "app";
              program = "${pkgs.writeShellScript "aspen-dev" ''
                set -e

                # Generate a unique data directory for this session
                DATA_DIR="''${ASPEN_DATA_DIR:-/tmp/aspen-dev-$$}"
                mkdir -p "$DATA_DIR"

                echo "Starting Aspen development node..."
                echo ""
                echo "  Node ID:    1"
                echo "  Cookie:     dev-cookie-$USER"
                echo "  Data dir:   $DATA_DIR"
                echo ""
                echo "This is a single-node development cluster."
                echo "For production, use: nix run .#aspen-node -- --node-id 1 --cookie <secret>"
                echo "For multi-node:      nix run .#cluster"
                echo ""

                exec ${bins.aspen-node}/bin/aspen-node \
                  --node-id 1 \
                  --cookie "dev-cookie-$USER" \
                  --data-dir "$DATA_DIR" \
                  "$@"
              ''}";
            };

            # Fuzzing commands - run with nix run .#fuzz, .#fuzz-quick, .#fuzz-intensive
            # NOTE: Must be run from the project root directory
            fuzz = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-all" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Starting parallel fuzzing campaign on high-risk targets (1 hour each)..."
                echo ""

                mkdir -p fuzz/dictionaries

                # Enter the fuzzing devShell environment
                nix develop .#fuzz --command bash -c '
                  # Function to run fuzzer and capture dictionary
                  run_and_capture() {
                    local target=$1
                    local output_file=$(mktemp)

                    cargo fuzz run "$target" -- -max_total_time=3600 2>&1 | tee "$output_file"

                    # Extract and save recommended dictionary
                    if grep -q "Recommended dictionary" "$output_file"; then
                      local dict_file="fuzz/dictionaries/$target-auto.dict"
                      echo "# Auto-generated dictionary from fuzzing run on $(date -I)" > "$dict_file"
                      sed -n "/^###### Recommended dictionary/,/^###### End of recommended dictionary/p" "$output_file" \
                        | grep -v "^######" >> "$dict_file"
                      echo "Saved dictionary to $dict_file"
                    fi
                    rm -f "$output_file"
                  }

                  # Run high-risk targets in parallel with dictionary capture
                  run_and_capture fuzz_raft_rpc &
                  run_and_capture fuzz_tui_rpc &
                  run_and_capture fuzz_protocol_handler &
                  run_and_capture fuzz_snapshot_json &
                  wait

                  echo ""
                  echo "Fuzzing complete. Dictionaries saved to fuzz/dictionaries/"
                '
              ''}";
            };

            fuzz-quick = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-quick" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Starting quick fuzzing smoke test (5 min per target)..."
                echo ""

                mkdir -p fuzz/dictionaries

                nix develop .#fuzz --command bash -c '
                  for target in fuzz_raft_rpc fuzz_tui_rpc fuzz_snapshot_json; do
                    echo "Fuzzing $target..."

                    # Capture output to extract dictionary
                    output_file=$(mktemp)
                    cargo fuzz run "$target" --sanitizer none -- -max_total_time=300 2>&1 | tee "$output_file"

                    # Extract and save recommended dictionary
                    if grep -q "Recommended dictionary" "$output_file"; then
                      dict_file="fuzz/dictionaries/$target-auto.dict"
                      echo "# Auto-generated dictionary from fuzzing run on $(date -I)" > "$dict_file"
                      sed -n "/^###### Recommended dictionary/,/^###### End of recommended dictionary/p" "$output_file" \
                        | grep -v "^######" >> "$dict_file"
                      echo "Saved dictionary to $dict_file ($(wc -l < "$dict_file") entries)"
                    fi
                    rm -f "$output_file"
                    echo ""
                  done
                '
              ''}";
            };

            fuzz-intensive = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-intensive" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Starting intensive fuzzing campaign (6 hours per target)..."
                echo ""

                mkdir -p fuzz/dictionaries

                nix develop .#fuzz --command bash -c '
                  targets=(fuzz_raft_rpc fuzz_tui_rpc fuzz_protocol_handler fuzz_snapshot_json
                           fuzz_log_entries fuzz_gossip fuzz_differential fuzz_http_api)

                  for target in "''${targets[@]}"; do
                    echo "Starting $target (6 hours)..."

                    # Capture output to extract dictionary
                    output_file=$(mktemp)
                    cargo fuzz run "$target" -- -max_total_time=21600 -jobs=4 2>&1 | tee "$output_file"

                    # Extract and save recommended dictionary
                    if grep -q "Recommended dictionary" "$output_file"; then
                      dict_file="fuzz/dictionaries/$target-auto.dict"
                      echo "# Auto-generated dictionary from fuzzing run on $(date -I)" > "$dict_file"
                      sed -n "/^###### Recommended dictionary/,/^###### End of recommended dictionary/p" "$output_file" \
                        | grep -v "^######" >> "$dict_file"
                      echo "Saved dictionary to $dict_file ($(wc -l < "$dict_file") entries)"
                    fi
                    rm -f "$output_file"

                    echo ""
                    echo "Minimizing corpus for $target..."
                    cargo fuzz cmin "$target" || true
                    echo ""
                  done
                '
              ''}";
            };

            fuzz-overnight = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-overnight" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Starting overnight fuzzing campaign (8 hours, 4 targets in parallel)..."
                echo "Estimated completion: $(date -d '+8 hours' '+%Y-%m-%d %H:%M')"
                echo ""

                mkdir -p fuzz/dictionaries

                nix develop .#fuzz --command bash -c '
                  # Function to run fuzzer and capture dictionary
                  run_and_capture() {
                    local target=$1
                    local output_file=$(mktemp)

                    echo "[$(date +%H:%M)] Starting $target..."
                    cargo fuzz run "$target" --sanitizer none -- -max_total_time=28800 2>&1 | tee "$output_file"

                    # Extract and save recommended dictionary
                    if grep -q "Recommended dictionary" "$output_file"; then
                      local dict_file="fuzz/dictionaries/$target-auto.dict"
                      echo "# Auto-generated dictionary from overnight fuzzing on $(date -I)" > "$dict_file"
                      sed -n "/^###### Recommended dictionary/,/^###### End of recommended dictionary/p" "$output_file" \
                        | grep -v "^######" >> "$dict_file"
                      echo "[$(date +%H:%M)] Saved dictionary to $dict_file"
                    fi
                    rm -f "$output_file"
                    echo "[$(date +%H:%M)] Completed $target"
                  }

                  # Run 4 high-priority targets in parallel (8 hours each)
                  run_and_capture fuzz_raft_rpc &
                  run_and_capture fuzz_tui_rpc &
                  run_and_capture fuzz_snapshot_json &
                  run_and_capture fuzz_http_api &
                  wait

                  echo ""
                  echo "=== Overnight fuzzing complete ==="
                  echo "Dictionaries saved to fuzz/dictionaries/"
                  echo "Corpus entries:"
                  for target in fuzz_raft_rpc fuzz_tui_rpc fuzz_snapshot_json fuzz_http_api; do
                    count=$(find "fuzz/corpus/$target" -type f 2>/dev/null | wc -l)
                    printf "  %-25s %d entries\n" "$target" "$count"
                  done
                '
              ''}";
            };

            fuzz-corpus = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-corpus" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                echo "Generating fuzz corpus seeds..."
                nix develop .#fuzz --command cargo run --bin generate_fuzz_corpus --features fuzzing
                echo ""
                echo "Corpus generation complete!"
              ''}";
            };

            # Benchmarking command
            # Usage: nix run .#bench [suite] [filter]
            #   nix run .#bench                    - Run all benchmarks
            #   nix run .#bench production        - Run production benchmarks only
            #   nix run .#bench kv_operations     - Run kv_operations benchmarks only
            #   nix run .#bench storage           - Run storage benchmarks only
            #   nix run .#bench workloads         - Run workload benchmarks only
            #   nix run .#bench concurrency       - Run concurrency benchmarks only
            #   nix run .#bench -- kv_write       - Run all benchmarks matching "kv_write"
            bench = {
              type = "app";
              program = "${pkgs.writeShellScript "bench" ''
                set -e
                SUITE="''${1:-}"
                FILTER="''${2:-}"

                case "$SUITE" in
                  production|kv_operations|storage|workloads|concurrency)
                    if [ -n "$FILTER" ]; then
                      echo "Running $SUITE benchmarks matching: $FILTER"
                      nix develop -c cargo bench --bench "$SUITE" -- "$FILTER"
                    else
                      echo "Running $SUITE benchmarks..."
                      nix develop -c cargo bench --bench "$SUITE"
                    fi
                    ;;
                  "")
                    echo "Running all benchmarks..."
                    nix develop -c cargo bench
                    ;;
                  *)
                    # Treat as filter for all benchmarks
                    echo "Running all benchmarks matching: $SUITE"
                    nix develop -c cargo bench -- "$SUITE"
                    ;;
                esac

                echo ""
                echo "HTML reports available at: target/criterion/report/index.html"
              ''}";
            };

            # Production benchmarks (realistic latencies with SQLite + Iroh)
            # Usage: nix run .#bench-production [filter]
            bench-production = {
              type = "app";
              program = "${pkgs.writeShellScript "bench-production" ''
                set -e
                FILTER="''${1:-}"

                echo "Running production benchmarks (SQLite + Iroh networking)..."
                echo "These show realistic distributed system latencies."
                echo ""

                if [ -n "$FILTER" ]; then
                  nix develop -c cargo bench --bench production -- "$FILTER"
                else
                  nix develop -c cargo bench --bench production
                fi

                echo ""
                echo "HTML reports available at: target/criterion/production/report/index.html"
              ''}";
            };

            # Rust formatter with nightly rustfmt for unstable features
            rustfmt = {
              type = "app";
              program = "${pkgs.writeShellScript "rustfmt" ''
                set -e
                MODE="''${1:-}"
                if [ "$MODE" = "check" ]; then
                  echo "Checking Rust formatting..."
                  ${pkgs.rust-bin.nightly.latest.rustfmt}/bin/rustfmt --edition 2024 --check $(find . -name '*.rs' -not -path './target/*' -not -path './openraft/*' -not -path './vendor/*')
                else
                  echo "Formatting Rust files..."
                  ${pkgs.rust-bin.nightly.latest.rustfmt}/bin/rustfmt --edition 2024 $(find . -name '*.rs' -not -path './target/*' -not -path './openraft/*' -not -path './vendor/*')
                fi
              ''}";
            };

            # Code coverage command
            # Usage: nix run .#coverage [subcommand]
            #   nix run .#coverage        - Show summary (default)
            #   nix run .#coverage html   - Generate HTML report and open browser
            #   nix run .#coverage ci     - Generate lcov.info for CI
            #   nix run .#coverage update - Update .coverage-baseline.toml
            coverage = {
              type = "app";
              program = "${pkgs.writeShellScript "coverage" ''
                set -e

                SUBCMD="''${1:-summary}"

                case "$SUBCMD" in
                  html|open)
                    echo "Generating HTML coverage report..."
                    nix develop -c cargo llvm-cov --html --open --lib \
                      --ignore-filename-regex '(tests/|examples/|openraft/)'
                    ;;

                  ci|lcov)
                    echo "Generating lcov.info for CI..."
                    nix develop -c cargo llvm-cov --lib \
                      --lcov \
                      --output-path lcov.info \
                      --ignore-filename-regex '(tests/|examples/|openraft/)'
                    echo "Coverage report written to lcov.info"
                    ;;

                  update|baseline)
                    echo "Generating coverage data..."
                    nix develop -c cargo llvm-cov --lib \
                      --ignore-filename-regex '(tests/|examples/|openraft/)' \
                      2>&1 | tee /tmp/coverage-output.txt

                    echo ""
                    echo "=== Updating .coverage-baseline.toml ==="

                    TOTAL=$(grep "^TOTAL" /tmp/coverage-output.txt | awk '{print $NF}' | tr -d '%')
                    TOTAL_LINES=$(grep "^TOTAL" /tmp/coverage-output.txt | awk '{print $8}')
                    COVERED_LINES=$(grep "^TOTAL" /tmp/coverage-output.txt | awk '{print $10}')

                    echo "Total coverage: $TOTAL%"
                    echo "Lines: $COVERED_LINES / $TOTAL_LINES"

                    if [ -f .coverage-baseline.toml ]; then
                      sed -i "s/^generated = .*/generated = \"$(date +%Y-%m-%d)\"/" .coverage-baseline.toml
                      sed -i "s/^total_coverage = .*/total_coverage = $TOTAL/" .coverage-baseline.toml
                      echo ""
                      echo "Updated .coverage-baseline.toml"
                      echo "Review changes with: git diff .coverage-baseline.toml"
                    fi
                    ;;

                  summary|"")
                    echo "=== Code Coverage Summary ==="
                    nix develop -c cargo llvm-cov --lib \
                      --ignore-filename-regex '(tests/|examples/|openraft/)'
                    echo ""
                    echo "Commands: nix run .#coverage [html|ci|update|summary]"
                    ;;

                  help|--help|-h)
                    echo "Usage: nix run .#coverage [subcommand]"
                    echo ""
                    echo "Subcommands:"
                    echo "  summary  Show coverage summary to stdout (default)"
                    echo "  html     Generate HTML report and open in browser"
                    echo "  ci       Generate lcov.info for CI/Codecov"
                    echo "  update   Update .coverage-baseline.toml with current coverage"
                    echo "  help     Show this help message"
                    ;;

                  *)
                    echo "Unknown subcommand: $SUBCMD"
                    echo "Run 'nix run .#coverage help' for usage"
                    exit 1
                    ;;
                esac
              ''}";
            };

            fuzz-coverage = {
              type = "app";
              program = "${pkgs.writeShellScript "fuzz-coverage" ''
                set -e

                if [ ! -f "fuzz/Cargo.toml" ]; then
                  echo "Error: Must be run from the aspen project root directory"
                  exit 1
                fi

                mkdir -p fuzz/coverage

                # Parse target from args or use default
                target="''${1:-fuzz_raft_rpc}"

                echo "Generating coverage report for $target..."
                echo ""

                nix develop .#fuzz --command bash -c "
                  # Generate coverage data
                  cargo fuzz coverage \"$target\"

                  # Find the profdata file
                  profdata_dir=\"fuzz/coverage/$target/coverage\"
                  if [ -d \"\$profdata_dir\" ]; then
                    # Merge profile data
                    \$LLVM_PROFDATA merge -sparse \"\$profdata_dir\"/*.profraw -o \"fuzz/coverage/$target.profdata\"

                    # Generate HTML report
                    target_bin=\"fuzz/target/x86_64-unknown-linux-gnu/coverage/x86_64-unknown-linux-gnu/release/$target\"
                    if [ -f \"\$target_bin\" ]; then
                      \$LLVM_COV show \"\$target_bin\" \
                        --instr-profile=\"fuzz/coverage/$target.profdata\" \
                        --format=html \
                        --output-dir=\"fuzz/coverage/$target-html\" \
                        --ignore-filename-regex='/.cargo/|/rustc/'

                      # Also generate summary
                      \$LLVM_COV report \"\$target_bin\" \
                        --instr-profile=\"fuzz/coverage/$target.profdata\" \
                        --ignore-filename-regex='/.cargo/|/rustc/' \
                        > \"fuzz/coverage/$target-summary.txt\"

                      echo \"\"
                      echo \"Coverage report generated:\"
                      echo \"  HTML: fuzz/coverage/$target-html/index.html\"
                      echo \"  Summary: fuzz/coverage/$target-summary.txt\"
                      echo \"\"
                      cat \"fuzz/coverage/$target-summary.txt\"
                    else
                      echo \"Warning: Could not find coverage binary at \$target_bin\"
                    fi
                  else
                    echo \"Warning: No coverage data found. Run fuzzing first.\"
                  fi
                "
              ''}";
            };

            # VM-isolated dogfood cluster
            # Usage: ASPEN_NODE_COUNT=3 nix run .#dogfood-vm
            # Launches N Cloud Hypervisor microVMs (1-10) for isolated CI testing
            dogfood-vm = let
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "dogfood-vm" ''
                set -e

                NODE_COUNT=''${ASPEN_NODE_COUNT:-3}
                VM_DIR="$PWD/.aspen/vms"

                # Validate node count (1-10)
                if [ "$NODE_COUNT" -lt 1 ] || [ "$NODE_COUNT" -gt 10 ]; then
                  echo "Error: ASPEN_NODE_COUNT must be 1-10 (got: $NODE_COUNT)"
                  exit 1
                fi

                # Check for KVM
                if [ ! -e /dev/kvm ]; then
                  echo "Error: KVM not available. Please ensure KVM is enabled."
                  echo "  - Check if virtualization is enabled in BIOS"
                  echo "  - Verify with: ls -la /dev/kvm"
                  exit 1
                fi

                mkdir -p "$VM_DIR"

                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    bins.git-remote-aspen
                    pkgs.cloud-hypervisor
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.curl
                    pkgs.netcat
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.iproute2
                    pkgs.bridge-utils
                    pkgs.procps # provides pgrep for process detection
                  ]
                }:$PATH"

                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                export GIT_REMOTE_ASPEN_BIN="${bins.git-remote-aspen}/bin/git-remote-aspen"
                export ASPEN_NODE_COUNT="$NODE_COUNT"
                export ASPEN_VM_DIR="$VM_DIR"
                # Override PROJECT_DIR since script is in Nix store, not the actual project
                export PROJECT_DIR="$PWD"

                exec ${scriptsDir}/dogfood-vm.sh "$@"
              ''}";
            };

            # Local dogfood cluster (no VMs)
            # Usage: nix run .#dogfood-local
            # Runs nodes as local processes for quick testing
            dogfood-local = let
              scriptsDir = pkgs.runCommand "aspen-scripts" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "dogfood-local" ''
                set -e

                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    bins.git-remote-aspen
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.git
                    pkgs.nix # Required for CI jobs that run nix build/fmt
                  ]
                }:$PATH"

                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                export GIT_REMOTE_ASPEN_BIN="${bins.git-remote-aspen}/bin/git-remote-aspen"
                export PROJECT_DIR="$PWD"

                exec ${scriptsDir}/dogfood-local.sh "$@"
              ''}";
            };

            # Local nodes with VM-isolated CI
            # Usage: nix run .#dogfood-local-vmci
            # Runs nodes as local processes, CI jobs in Cloud Hypervisor VMs
            dogfood-local-vmci = let
              scriptsDir = pkgs.runCommand "aspen-scripts-vmci" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "dogfood-local-vmci" ''
                set -e

                export PATH="${
                  pkgs.lib.makeBinPath [
                    bins.aspen-node
                    bins.aspen-cli
                    bins.git-remote-aspen
                    pkgs.bash
                    pkgs.coreutils
                    pkgs.gnugrep
                    pkgs.gnused
                    pkgs.git
                    pkgs.nix
                    pkgs.cloud-hypervisor
                    pkgs.virtiofsd
                  ]
                }:$PATH"

                export ASPEN_NODE_BIN="${bins.aspen-node}/bin/aspen-node"
                export ASPEN_CLI_BIN="${bins.aspen-cli}/bin/aspen-cli"
                export GIT_REMOTE_ASPEN_BIN="${bins.git-remote-aspen}/bin/git-remote-aspen"
                export CLOUD_HYPERVISOR_BIN="${pkgs.cloud-hypervisor}/bin/cloud-hypervisor"
                export VIRTIOFSD_BIN="${pkgs.virtiofsd}/bin/virtiofsd"
                export PROJECT_DIR="$PWD"

                exec ${scriptsDir}/dogfood-local-vmci.sh "$@"
              ''}";
            };

            # Network setup for CI VMs (run with sudo before dogfood-local-vmci)
            # Usage: sudo nix run .#setup-ci-network
            setup-ci-network = let
              scriptsDir = pkgs.runCommand "aspen-scripts-network" {} ''
                mkdir -p $out
                cp -r ${./scripts}/* $out/
                chmod -R +w $out
              '';
            in {
              type = "app";
              program = "${pkgs.writeShellScript "setup-ci-network" ''
                set -e
                export PATH="${
                  pkgs.lib.makeBinPath [
                    pkgs.iproute2
                    pkgs.nftables
                    pkgs.iptables
                    pkgs.coreutils
                  ]
                }:$PATH"
                exec ${scriptsDir}/setup-ci-network.sh "$@"
              ''}";
            };

            # build-plugins and deploy-plugin: moved to ~/git/aspen-plugins repo
          };
        }
        // {
          # Packages exposed by the flake
          packages =
            {
              default = bins.aspen-node;
              aspen-node = bins.aspen-node;
              # Full-source builds for inspection/debugging
              # _debug-full-src = fullSrc;
              # aspen-tui extracted to ~/git/aspen-tui
              aspen-cli = bins.aspen-cli;
              aspen-cli-forge = bins.aspen-cli-forge;
              # aspen-ci-agent extracted to ~/git/aspen-ci
              git-remote-aspen = bins.git-remote-aspen;
              netwatch = netwatch;
              vm-test-setup = vm-test-setup;
              vm-test-run = vm-test-run;
              # Pre-built cargo dependencies for CI and binary cache.
              # Push these to your cache first — they're the most expensive derivations.
              #   nix build .#cargoArtifacts .#nodeCargoArtifacts .#cliCargoArtifacts
              inherit cargoArtifacts nodeCargoArtifacts cliCargoArtifacts;
              # Verus formal verification tool (binary release with proper library setup)
              inherit verus;
              # Direct access to all Verus binaries (rust_verify, cargo-verus, z3, etc.)
              verus-root = verusRoot;
              # Z3 4.12.5 (required by Verus)
              z3 = z3_4_12_5;
            }
            // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") (
              let
                # Build the CI worker VM configuration
                ciVmConfig = nixpkgs.lib.nixosSystem {
                  system = "x86_64-linux";
                  modules = [
                    microvm.nixosModules.microvm
                    (import ./nix/vms/ci-worker-node.nix {
                      inherit pkgs;
                      lib = nixpkgs.lib;
                      vmId = "aspen-ci-vm";
                      aspenNodePackage = bins.aspen-node;
                    })
                  ];
                };
              in {
                # CI VM kernel for Cloud Hypervisor worker
                ci-vm-kernel = ciVmConfig.config.microvm.kernel;
                # CI VM initrd for Cloud Hypervisor worker
                ci-vm-initrd = ciVmConfig.config.system.build.initialRamdisk;
                # CI VM toplevel (NixOS system with init script)
                # The kernel cmdline needs init=${toplevel}/init to boot NixOS properly
                ci-vm-toplevel = ciVmConfig.config.system.build.toplevel;
                # Full CI VM runner (includes cloud-hypervisor command)
                ci-vm-runner = ciVmConfig.config.microvm.runner.cloud-hypervisor;
                # VirtioFS test initramfs
                inherit virtiofsTestInitrd;
              }
            );
        }
        // {
          # Fuzzing development shell with nightly Rust
          # Usage: nix develop .#fuzz
          devShells.fuzz = let
            rustNightly = pkgs.rust-bin.nightly.latest.default.override {
              extensions = ["rust-src" "llvm-tools-preview"];
            };
          in
            pkgs.mkShell {
              buildInputs = with pkgs; [
                rustNightly
                cargo-fuzz
                llvmPackages_latest.llvm
                llvmPackages_latest.libclang
                pkg-config
                openssl.dev
              ];

              LIBCLANG_PATH = "${pkgs.llvmPackages_latest.libclang.lib}/lib";
              LLVM_COV = "${pkgs.llvmPackages_latest.llvm}/bin/llvm-cov";
              LLVM_PROFDATA = "${pkgs.llvmPackages_latest.llvm}/bin/llvm-profdata";

              shellHook = ''
                echo "Fuzzing development environment ready!"
                echo ""
                echo "Nix apps (from project root):"
                echo "  nix run .#fuzz           # Parallel fuzzing (1hr/target)"
                echo "  nix run .#fuzz-quick     # Quick smoke test (5min/target)"
                echo "  nix run .#fuzz-overnight # Overnight run (8hr, 4 parallel targets)"
                echo "  nix run .#fuzz-intensive # Full campaign (6hr/target, sequential)"
                echo "  nix run .#fuzz-coverage  # Generate coverage report"
                echo "  nix run .#fuzz-corpus    # Generate seed corpus"
                echo ""
                echo "Manual commands (in this shell):"
                echo "  cargo fuzz list"
                echo "  cargo fuzz run fuzz_raft_rpc --sanitizer none -- -max_total_time=60"
                echo "  cargo fuzz coverage fuzz_raft_rpc"
                echo ""
                echo "Outputs saved to:"
                echo "  fuzz/corpus/          # Coverage-increasing inputs"
                echo "  fuzz/artifacts/       # Crash-triggering inputs"
                echo "  fuzz/dictionaries/    # Auto-generated dictionaries"
                echo "  fuzz/coverage/        # Coverage reports"
              '';
            };

          # Verus formal verification development shell
          devShells.verus = verusDevShell;

          devShells.default = craneLib.devShell {
            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages = with pkgs;
              [
                netwatch
                litefs # Transparent SQLite replication via FUSE filesystem
                bash
                coreutils
                cargo-watch
                cargo-nextest
                cargo-llvm-cov # Code coverage tool
                git
                jq
                ripgrep
                rust-analyzer
                sqlite
                pkg-config
                openssl.dev
                zlib # Required by hyperlight-host build script
                codex
                lld # Linker for WASM targets
                mold # Fast linker for development builds
                clang # Used as linker driver for mold
                protobuf # Protocol Buffers compiler for snix crates
                # Pre-commit and quality tools
                pre-commit
                shellcheck
                nodePackages.markdownlint-cli
                # Cloud Hypervisor for VM-based testing
                cloud-hypervisor
                virtiofsd # VirtioFS daemon for VM filesystem sharing
                OVMF # UEFI firmware for Cloud Hypervisor
                # Helper tools for VM management
                bridge-utils # For network bridge management
                iproute2 # For ip commands (TAP devices)
                qemu-utils # For qemu-img disk operations
                # LLVM tools for coverage
                rustc.llvmPackages.llvm
              ]
              ++ [
                # Add our custom helper scripts to devShell
                vm-test-setup
                vm-test-run
                # Verus formal verification tool
                verus
                # Z3 for Verus formal verification (bundled in verus, but also available standalone)
                z3_4_12_5
              ];

            env.RUST_SRC_PATH = "${rustToolChain}/lib/rustlib/src/rust/library";
            env.SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";

            # LLVM coverage tool environment variables
            inherit (pkgs.cargo-llvm-cov) LLVM_COV LLVM_PROFDATA;

            # Incremental compilation settings for faster rebuilds
            env.CARGO_INCREMENTAL = "1";
            env.CARGO_BUILD_INCREMENTAL = "true";

            # Configure cargo to use a shared target directory for better caching
            # This prevents duplicate builds when switching between nix develop and direct cargo commands
            env.CARGO_TARGET_DIR = "target";

            # Enable cargo's new resolver for better dependency resolution
            env.CARGO_RESOLVER = "2";

            # Library path for build scripts that need dynamic libraries (e.g., hyperlight-host needs zlib)
            env.LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [pkgs.zlib pkgs.stdenv.cc.cc.lib];

            # Cloud Hypervisor environment variables
            env.CH_KERNEL = "${pkgs.linuxPackages.kernel}/bzImage";
            env.CH_FIRMWARE = "${pkgs.OVMF.fd}/FV/OVMF.fd";

            # Virtiofsd path for Cloud Hypervisor worker
            env.VIRTIOFSD_PATH = "${pkgs.virtiofsd}/bin/virtiofsd";

            # Cloud Hypervisor binary for VirtioFS integration tests
            env.CLOUD_HYPERVISOR_BIN = "${pkgs.cloud-hypervisor}/bin/cloud-hypervisor";

            # VirtioFS test initramfs for cloud_hypervisor_virtiofs_test
            env.VIRTIOFS_TEST_INITRD = "${virtiofsTestInitrd}";

            shellHook = ''
              # Auto-stage scripts and decision docs so nix flake sees them
              if [ -d .git ]; then
                git add scripts/*.sh .claude/decisions/*.md 2>/dev/null || true
              fi

              # Add result/bin to PATH if it exists (from nix build)
              if [ -d "$PWD/result/bin" ]; then
                export PATH="$PWD/result/bin:$PATH"
              fi

              # Set up CI worker environment variables on x86_64-linux
              # These enable the Cloud Hypervisor worker for isolated CI builds
              if [ "$(uname -m)" = "x86_64" ] && [ "$(uname -s)" = "Linux" ]; then
                # Only build CI VM components if not already set
                if [ -z "$ASPEN_CI_KERNEL_PATH" ]; then
                  # Check if CI VM kernel is already built (fast path)
                  CI_KERNEL_STORE=$(nix path-info .#ci-vm-kernel 2>/dev/null || true)
                  if [ -n "$CI_KERNEL_STORE" ]; then
                    export ASPEN_CI_KERNEL_PATH="$CI_KERNEL_STORE/bzImage"
                    export ASPEN_CI_INITRD_PATH=$(nix path-info .#ci-vm-initrd 2>/dev/null)/initrd
                    echo "CI worker: kernel/initrd paths set from cache"
                  else
                    echo "CI worker: Run 'nix build .#ci-vm-kernel .#ci-vm-initrd' to enable VM isolation"
                  fi
                fi
              fi

              echo "Aspen development environment"
              echo ""
              echo "Build caching:"
              echo "  incremental compilation enabled"
              echo ""
              echo "Common commands:"
              echo "  cargo build                          Build project"
              echo "  cargo nextest run                    Run all tests"
              echo "  cargo nextest run -P quick           Quick tests (~2-5 min)"
              echo ""
              echo "Nix apps:"
              echo "  nix run .#cluster                    3-node cluster"
              echo "  nix run .#bench                      Run benchmarks"
              echo "  nix run .#coverage [html|ci|update]  Code coverage"
              echo "  nix run .#fuzz-quick                 Fuzzing smoke test"
              echo ""
              echo "VM testing: aspen-vm-setup / aspen-vm-run <node-id>"
              echo "  nix run .#dogfood-vm                 Dogfood in isolated VMs"
              echo ""
              echo "CI worker setup (x86_64-linux only):"
              if [ -n "$ASPEN_CI_KERNEL_PATH" ]; then
                echo "  Status: ENABLED (kernel/initrd paths set)"
              else
                echo "  Status: DISABLED (run 'nix build .#ci-vm-kernel .#ci-vm-initrd')"
              fi
            '';
          };
        }
    )
    // {
      # System-independent outputs

      # NixOS modules for Aspen services
      nixosModules = {
        # Aspen node service module
        # Usage: services.aspen.node = { enable = true; nodeId = 1; cookie = "..."; package = ...; };
        aspen-node = import ./nix/modules/aspen-node.nix;

        # Default module includes all Aspen NixOS modules
        default = import ./nix/modules/aspen-node.nix;
      };

      # Library functions for building Aspen infrastructure
      lib = let
        # Use x86_64-linux as the default system for VM builds
        defaultSystem = "x86_64-linux";
        pkgs = import nixpkgs {system = defaultSystem;};
      in {
        # Build a dogfood VM for a specific node ID
        # Returns a NixOS system with microvm configuration
        # The runner is at: result.config.microvm.runner.<hypervisor>
        mkDogfoodVm = {
          nodeId,
          cookie ? "dogfood-vm",
          aspenPackage ? null,
        }:
          nixpkgs.lib.nixosSystem {
            system = defaultSystem;
            modules = [
              microvm.nixosModules.microvm
              ./nix/modules/aspen-node.nix
              (import ./nix/vms/dogfood-node.nix {
                inherit (pkgs) lib;
                inherit nodeId cookie aspenPackage;
              })
            ];
          };
      };
    };
}
