{
  description = "mvm-ci";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-25.05";

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
  };

  nixConfig = {
    keepOutputs = true;
    max-jobs = "auto";
    builders = "";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    advisory-db,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pname = "mvm-ci";
      lib = nixpkgs.lib;
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      rustToolChain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolChain;
      
      flawless = pkgs.stdenv.mkDerivation {
        pname = "flawless";
        version = "1.0.0-beta.3";
        src = pkgs.fetchurl {
          url = "https://downloads.flawless.dev/1.0.0-beta.3/x64-linux/flawless";
          sha256 = "0p11baphc2s8rjhzn9v2sai52gvbn33y1xlqg2yais6dmf5mj4dm";
        };
        dontUnpack = true;
        nativeBuildInputs = [
            pkgs.autoPatchelfHook 
            pkgs.pkg-config
            pkgs.openssl.dev
            pkgs.lld  # Linker for WASM targets
            pkgs.stdenv.cc.cc
];
        installPhase = ''
          mkdir -p $out/bin
          cp $src $out/bin/flawless
          chmod +x $out/bin/flawless
'';
      }; 
      

      srcFilters = path: type:
        builtins.any (suffix: lib.hasSuffix suffix path) [
          ".subplot" # build and documentation
          ".md"
          ".yaml"
          ".css"
          ".html"  # Include askama templates
        ]
        ||
        # Include vendor directory for h3-iroh and snix proto files
        (lib.hasInfix "/vendor/" path)
        ||
        # Include templates directory for askama
        (lib.hasInfix "/templates/" path)
        ||
        # Default filter from crane (allow .rs files)
        (craneLib.filterCargoSources path type);

      src = lib.cleanSourceWith {
        src = ./.;
        filter = srcFilters;
      };

      basicArgs = {
        inherit src;
        inherit pname;
        strictDeps = true;

        nativeBuildInputs = with pkgs; [
          git
          pkg-config
          lld  # Linker for WASM targets
          protobuf  # Protocol Buffers compiler for snix crates
          stdenv.cc.cc
        ];

        buildInputs = with pkgs; [
          openssl
          stdenv.cc.cc.lib  # Provides libgcc_s.so.1
        ];

        # Set environment variable required by snix-build at compile time
        SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";

        # Set PROTO_ROOT for snix-castore build.rs to find proto files
        # The proto files are vendored in ./vendor/snix
        PROTO_ROOT = "${src}/vendor";
      };

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly basicArgs;

      # Common arguments can be set here to avoid repeating them later
      commonArgs =
        basicArgs
        // {
          inherit cargoArtifacts;

          nativeBuildInputs = basicArgs.nativeBuildInputs ++ (with pkgs; [
            autoPatchelfHook
          ]);

          buildInputs = basicArgs.buildInputs ++ (lib.optionals pkgs.stdenv.buildPlatform.isDarwin (with pkgs; [
            darwin.apple_sdk.frameworks.Security
          ]));
        };

      mvm-ci = craneLib.buildPackage (commonArgs
        // {
          inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
          doCheck = false;
        });

      bins = let
        bin = {
          name,
        }:
          craneLib.buildPackage (commonArgs
            // {
              inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
              cargoExtraArgs = "--bin ${name}";
              doCheck = false;
            });
        bins = builtins.listToAttrs (map
          ({name, ...} @ package: lib.nameValuePair name (bin package))
          [
            {
              name = "cibtool";
            }
            {
              name = "cib";
            }
            {
              name = "synthetic-events";
            }
            {
              name = "mvm-ci";
            }
            {
              name = "worker";
            }
          ]);
      in
        bins
        // rec {
          default = mvm-ci;
          ci-broker = pkgs.buildEnv {
            name = "mvm-ci";
            paths = with bins; [
              cibtool
              cib
              synthetic-events
            ];
          };
        };
    in {
      # Formatter
      formatter = pkgs.alejandra;

      # Set of checks that are run: `nix flake check`
      checks = {
        # Run clippy (and deny all warnings) on the crate source,
        # again, reusing the dependency artifacts from above.
        #
        # Note that this is done as a separate derivation so that
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        clippy = craneLib.cargoClippy (commonArgs
          // {
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        doc = craneLib.cargoDoc commonArgs;
        fmt = craneLib.cargoFmt basicArgs;
        deny = craneLib.cargoDeny commonArgs;

        audit = craneLib.cargoAudit {
          inherit src advisory-db;
        };

        # Run tests with cargo-nextest
        nextest = craneLib.cargoNextest (commonArgs
          // {
            # We skip the test since it uses the underlying .git directory,
            # which is not available in the Nix sandbox.
            # In any case, this test is slow and we expect it to be tested
            # before merges (and it can be tested in the devShell)
            cargoNextestExtraArgs = "-- --skip acceptance_criteria_for_upgrades";
            partitions = 1;
            partitionType = "count";
            nativeBuildInputs = [
              pkgs.bash
              pkgs.git
              pkgs.jq
              pkgs.sqlite
            ];

            # Ensure dev is used since we rely on env variables being
            # set in tests.
            env.CARGO_PROFILE = "dev";
          });

        # Integration tests with flawless server
        # Run with: nix run .#integration-tests
        # Or check with: nix flake check -L
        integration-tests = pkgs.stdenv.mkDerivation {
          name = "mvm-ci-integration-tests";
          src = ./.;

          nativeBuildInputs = [
            rustToolChain
            flawless
            pkgs.bash
            pkgs.curl
            pkgs.git
            pkgs.jq
            pkgs.sqlite
            pkgs.pkg-config
            pkgs.openssl.dev
            pkgs.protobuf
          ];

          buildInputs = [
            pkgs.openssl
          ];

          # Required environment variables
          env.SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";
          env.RUST_SRC_PATH = "${rustToolChain}/lib/rustlib/src/rust/library";

          # Disable sandbox for network access (flawless server needs it)
          __noChroot = true;

          buildPhase = ''
            export HOME=$TMPDIR
            export CARGO_HOME=$TMPDIR/.cargo
            mkdir -p $CARGO_HOME

            echo "Building project for tests..."
            cargo build --tests
          '';

          checkPhase = ''
            echo "Running integration test script..."
            bash ${./scripts/run-integration-tests.sh}
          '';

          installPhase = ''
            mkdir -p $out
            echo "Integration tests passed" > $out/result
          '';

          doCheck = true;
        };
      };

      packages.default = mvm-ci;

      # Docker image for cluster testing (using streamLayeredImage for better caching)
      packages.dockerImage = pkgs.dockerTools.streamLayeredImage {
        name = "mvm-ci-cluster";
        tag = "latest";

        contents = [
          mvm-ci
          flawless
          pkgs.bash
          pkgs.coreutils
          pkgs.gettext  # for envsubst
          pkgs.cacert
          # Add entrypoint scripts and config template
          (pkgs.runCommand "mvm-ci-extras" {} ''
            mkdir -p $out/bin $out/etc
            cp ${./docker-entrypoint.sh} $out/bin/docker-entrypoint.sh
            cp ${./worker-entrypoint.sh} $out/bin/worker-entrypoint.sh
            chmod +x $out/bin/docker-entrypoint.sh
            chmod +x $out/bin/worker-entrypoint.sh
            cp ${./hiqlite-cluster.toml.template} $out/etc/hiqlite.toml.template
          '')
        ];

        config = {
          Cmd = [ "${pkgs.bash}/bin/sh" "/bin/docker-entrypoint.sh" ];
          Env = [
            "PATH=/bin"
            "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
          ];
          ExposedPorts = {
            "3020/tcp" = {};
            "9000/tcp" = {};
            "9001/tcp" = {};
            "27288/tcp" = {};
          };
          WorkingDir = "/";
        };
      };

      apps.cibtool = flake-utils.lib.mkApp {
        drv = self.bins.${system}.cibtool;
      };

      apps.cib = flake-utils.lib.mkApp {
        drv = self.bins.${system}.cib;
      };

      apps.synthetic-events = flake-utils.lib.mkApp {
        drv = self.bins.${system}.synthetic-events;
      };

      # Integration tests app - starts flawless server and runs tests
      apps.integration-tests = {
        type = "app";
        program = "${pkgs.writeShellScript "run-integration-tests" ''
          export PATH="${pkgs.lib.makeBinPath [
            rustToolChain
            flawless
            pkgs.bash
            pkgs.curl
            pkgs.coreutils
            pkgs.cargo-nextest
          ]}:$PATH"
          export SNIX_BUILD_SANDBOX_SHELL="${pkgs.busybox}/bin/sh"

          echo "Running integration tests with flawless server..."
          exec ${./scripts/run-integration-tests.sh}
        ''}";
      };

      devShells.default = craneLib.devShell {
        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = with pkgs; [
          flawless
          litefs  # Transparent SQLite replication via FUSE filesystem
          bash
          coreutils
          cargo-watch
          cargo-nextest
          git
          jq
          ripgrep
          rust-analyzer
          sqlite
          pkg-config
          openssl.dev
          lld  # Linker for WASM targets
          protobuf  # Protocol Buffers compiler for snix crates
        ];

        env.RUST_SRC_PATH = "${rustToolChain}/lib/rustlib/src/rust/library";
        env.SNIX_BUILD_SANDBOX_SHELL = "${pkgs.busybox}/bin/sh";
      };
    });
}

