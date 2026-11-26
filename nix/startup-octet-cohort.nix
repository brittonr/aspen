# Retain genuine Octet producer artifacts from the same immutable source as the executable.
# Mirrors the pinned provider's consumer-check environment; vendoring is explicit so Git
# checkouts that carry publish-reserved files can be normalized without changing crates.
# `name` and `runs` let other checks reuse this exact environment for a different Octet scope.
{
  pkgs,
  octet,
  system,
  source,
  gateCompiler,
  name ? "molten-retained-startup-octet-cohort",
  runs ? null,
}:
let
  octetPkgs = import octet.inputs.nixpkgs {
    inherit system;
    overlays = [ (import octet.inputs.rust-overlay) ];
  };
  channel = (builtins.fromTOML (builtins.readFile "${octet}/rust-toolchain.toml")).toolchain.channel;
  hostTriple = octetPkgs.stdenv.hostPlatform.rust.rustcTarget;
  rustupToolchain =
    if octetPkgs.lib.hasSuffix hostTriple channel then channel else "${channel}-${hostTriple}";
  craneLib = (octet.inputs.crane.mkLib octetPkgs).overrideToolchain gateCompiler;
  rustupShim = octetPkgs.writeShellScriptBin "rustup" ''
    toolchain="''${RUSTUP_TOOLCHAIN:-${rustupToolchain}}"
    case "$1" in
      which) which "$2" ;;
      show) echo "$toolchain (default)" ;;
      *) echo "rustup shim: unsupported: $*" >&2; exit 1 ;;
    esac
  '';
  cargoShim = octetPkgs.writeShellScriptBin "cargo" ''
    export RUSTUP_TOOLCHAIN="''${RUSTUP_TOOLCHAIN:-${rustupToolchain}}"
    exec ${gateCompiler}/bin/cargo "$@"
  '';
  provider = octet.packages.${system};
  dylintLib = octetPkgs.runCommand "octet-dylint-lib" { } ''
    mkdir -p "$out/lib"
    ln -s ${provider.default}/lib/liboctet.so "$out/lib/liboctet@${rustupToolchain}.so"
  '';
  # Every Cargo git source is bound to the metadata-free `pkgs.fetchgit` identity recorded in
  # `crate-hashes.json`, so vendoring is hermetic instead of fetching each remote at evaluation
  # time. The lock names a source by `git+URL?rev=REV#REV`; the hash file names it by
  # `URL?rev=REV#CRATE@VERSION`, so the query prefix is the join key.
  lib = octetPkgs.lib;
  crateHashes = builtins.fromJSON (builtins.readFile "${source}/crate-hashes.json");
  queryOf = value: builtins.head (lib.splitString "#" (lib.removePrefix "git+" value));
  hashByQuery = builtins.listToAttrs (
    map (key: lib.nameValuePair (queryOf key) crateHashes.${key}) (builtins.attrNames crateHashes)
  );
  lockPackages = (builtins.fromTOML (builtins.readFile "${source}/Cargo.lock")).package;
  gitPackages = builtins.filter (package: lib.hasPrefix "git+" (package.source or "")) lockPackages;
  outputHashes = builtins.listToAttrs (
    map (package: lib.nameValuePair package.source hashByQuery.${queryOf package.source}) gitPackages
  );
  vendor = craneLib.vendorCargoDeps {
    cargoLock = "${source}/Cargo.lock";
    inherit outputHashes;
    # `cargo package -l` rejects committed publish backups; they are not crate sources.
    # kamacite-core reads governed adapter data from its workspace root; vendor that data
    # beside the crate and keep the same bytes rather than dropping the include.
    overrideVendorGitCheckout = _: drv: drv.overrideAttrs (previous: {
      preInstall = (previous.preInstall or "") + ''
        find . -name Cargo.toml.orig -type f -delete
      '';
      postInstall = (previous.postInstall or "") + ''
        if [ -d "$out/kamacite-core-0.1.0" ] && [ -d adapters ]; then
          cp -r adapters "$out/kamacite-core-0.1.0/adapters"
          find "$out/kamacite-core-0.1.0/src" -name '*.rs' -exec \
            sed -i 's|"\.\./\.\./\.\./adapters/|"../adapters/|g' {} +
        fi
        # Choregraph crates (at `crates/<name>` in their repository) mount repository-root `vendor/`,
        # `library/`, and `contracts/` files by relative path (`#[path]`, `include_str!`). Carry those
        # directories beside each crate as `choregraph-root/` and rebase each reference: a file `d`
        # directories deep inside the crate climbs `d + 2` levels to the repository root.
        for crate in "$out"/choregraph-*; do
          [ -d "$crate/src" ] || continue
          for top in vendor library contracts; do
            if [ -d "$top" ] && grep -rqE "\"(\.\./)+$top/" "$crate/src"; then
              mkdir -p "$crate/choregraph-root"
              cp -r "$top" "$crate/choregraph-root/$top"
            fi
          done
          [ -d "$crate/choregraph-root" ] || continue
          find "$crate/src" -name '*.rs' | while IFS= read -r file; do
            depth=$(dirname "''${file#"$crate"/}" | awk -F/ '{ print NF }')
            up=$(printf '\\.\\./%.0s' $(seq 1 "$depth"))
            down=$(printf '../%.0s' $(seq 1 "$depth"))
            sed -i -E "s#\"$up\\.\\./\\.\\./(vendor|library|contracts)/#\"''${down}choregraph-root/\\1/#g" "$file"
          done
        done
      '';
    });
  };
  startupRuns = ''
    export CARGO_TARGET_DIR="$NIX_BUILD_TOP/primary-target"
    # Cargo subcommand dispatch records the canonical `cargo octet check` command the gate admits.
    cargo octet check --artifact-dir "$out/primary"
    export CARGO_TARGET_DIR="$NIX_BUILD_TOP/library-target"
    cargo octet check -p molten --artifact-dir "$out/library" -- --lib
    cmp Cargo.lock ${source}/Cargo.lock
    python3 - ${provider.cargo-octet}/bin/cargo-octet "$out/primary/object-corpus-receipt.json" <<'PY'
    import pathlib
    import subprocess
    import sys

    files = sorted(str(path) for path in pathlib.Path("src").rglob("*.rs"))
    if not files:
        raise SystemExit("missing actual Rust object corpus")
    subprocess.run([sys.argv[1], "object", "corpus", "receipt", "--output", sys.argv[2], *files], check=True)
    PY
  '';
in
octetPkgs.runCommand name
  {
    nativeBuildInputs = [ octetPkgs.pkg-config octetPkgs.python3 pkgs.clang cargoShim rustupShim provider.cargo-octet ];
    LD_LIBRARY_PATH = octetPkgs.lib.makeLibraryPath [ gateCompiler ];
    CARGO_INCREMENTAL = "0";
    CARGO_NET_OFFLINE = "true";
    # Vendored sources are toolchain-independent; the feature matrix reuses them.
    passthru = { inherit vendor; };
  }
  ''
    export HOME="$PWD/home"
    mkdir -p "$HOME"
    cp -r ${source} source
    chmod -R u+w source
    cd source
    export CARGO_HOME="$PWD/.cargo-home"
    mkdir -p "$CARGO_HOME"
    cp ${vendor}/config.toml "$CARGO_HOME/config.toml"
    export TIGERSTYLE_DRIVER_PATH="${provider.dylint-driver}/bin/dylint-driver"
    export TIGERSTYLE_LINT_LIB="${dylintLib}/lib/liboctet@${rustupToolchain}.so"
    export TIGERSTYLE_TOOLCHAIN="${rustupToolchain}"
    mkdir -p "$out"
    ${if runs == null then startupRuns else runs}
  ''
