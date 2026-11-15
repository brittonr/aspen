{ pkgs, rustToolchain, releasePolicyPkg }:
let
  lib = pkgs.lib;
  registry = "registry+https://github.com/rust-lang/crates.io-index";
  # Published 2026-11-10; archive checksum pins scanner source and its Cargo.lock.
  scannerSource = pkgs.fetchCrate {
    pname = "cargo-audit";
    version = "0.23.0";
    # Recursive hash independently matched the checksum-verified published archive.
    sha256 = "sha256-SePgSuYYc/5ZNgC8iKNW2wfEi9kejpOkIqVCyBWbgs4=";
  };
  platform = pkgs.makeRustPlatform { cargo = rustToolchain; rustc = rustToolchain; };
  scanner = platform.buildRustPackage {
    pname = "molten-cargo-audit";
    version = "0.23.0";
    src = scannerSource;
    cargoLock.lockFile = "${scannerSource}/Cargo.lock";
    nativeBuildInputs = [ pkgs.pkg-config pkgs.cmake pkgs.perl ];
    buildInputs = [ pkgs.openssl pkgs.zlib ];
    buildNoDefaultFeatures = true;
    # Published crate excludes tests; upstream network tests are not scan evidence.
    doCheck = false;
    meta.license = with lib.licenses; [ mit asl20 ];
  };
  # Reuse the repository's locked cohort without changing workspace locks/plans.
  # SPDX's non-optional dependency is smallvec; detection backends are not enabled.
  packages = (builtins.fromTOML (builtins.readFile ../Cargo.lock)).package ++ [{
    name = "spdx";
    version = "0.13.4";
    source = registry;
    checksum = "a8da593e30beb790fc9424502eb898320b44e5eb30367dbda1c1edde8e2f32d7";
    dependencies = [ "smallvec" ];
  }];
  normalizerPackage = {
    name = "molten-dependency-audit-normalizer";
    version = "0.1.0";
    dependencies = [ "blake3 1.8.5" "serde 1.0.228" "serde_json 1.0.149" "spdx 0.13.4" ];
  };
  resolve = dependency:
    let
      fields = lib.splitString " " dependency;
      matches = builtins.filter (p: p.name == builtins.head fields
        && (builtins.length fields == 1 || p.version == builtins.elemAt fields 1)) packages;
      package = if builtins.length matches == 1 then builtins.head matches
        else throw "dependency-audit: ambiguous/unavailable locked dependency ${dependency}";
    in assert package.source == registry; { key = "${package.name}@${package.version}"; inherit package; };
  closure = builtins.genericClosure {
    startSet = [{ key = "${normalizerPackage.name}@${normalizerPackage.version}"; package = normalizerPackage; }];
    operator = item: map resolve (item.package.dependencies or []);
  };
  renderPackage = item:
    let p = item.package; in
    "[[package]]\nname = ${builtins.toJSON p.name}\nversion = ${builtins.toJSON p.version}\n"
    + lib.optionalString (p ? source) "source = ${builtins.toJSON p.source}\n"
    + lib.optionalString (p ? checksum) "checksum = ${builtins.toJSON p.checksum}\n"
    + "dependencies = ${builtins.toJSON (p.dependencies or [])}\n";
  normalizerLock = pkgs.writeText "molten-dependency-audit-Cargo.lock"
    ("version = 4\n" + lib.concatMapStringsSep "\n" renderPackage closure);
  normalizerSource = pkgs.runCommand "molten-dependency-audit-source" {} ''
    mkdir -p "$out/src" "$out/config"
    cp ${normalizerLock} "$out/Cargo.lock"
    # Remove only cargo-script's shebang/frontmatter, retaining the same Rust core.
    ${pkgs.gawk}/bin/awk 'NR == 1 { next } /^---$/ { manifest = !manifest; next } !manifest { print }' \
      ${../tools/dependency-audit.rs} > "$out/src/main.rs"
    cp -R ${../config/dependency-audit} "$out/config/dependency-audit"
    cat > "$out/Cargo.toml" <<'MANIFEST'
    [package]
    name = "molten-dependency-audit-normalizer"
    version = "0.1.0"
    edition = "2024"
    [dependencies]
    blake3 = "=1.8.5"
    serde = { version = "=1.0.228", features = ["derive"] }
    serde_json = "=1.0.149"
    spdx = "=0.13.4"
    MANIFEST
  '';
  normalizer = platform.buildRustPackage {
    pname = normalizerPackage.name;
    version = normalizerPackage.version;
    src = normalizerSource;
    cargoLock.lockFile = normalizerLock;
    # The fixture check below runs the installed core's controls once.
    doCheck = false;
  };
  runtimeInputs = [ rustToolchain pkgs.nickel pkgs.jq pkgs.b3sum pkgs.git pkgs.coreutils pkgs.gawk ];
  app = pkgs.writeShellApplication {
    name = "molten-dependency-audit";
    inherit runtimeInputs;
    text = ''
      export MOLTEN_AUDIT_SCANNER=${scanner}/bin/cargo-audit
      export MOLTEN_AUDIT_NORMALIZER_BIN=${normalizer}/bin/molten-dependency-audit-normalizer
      export MOLTEN_RELEASE_POLICY_BIN=${releasePolicyPkg}/bin/molten-release-policy
      root="$(git rev-parse --show-toplevel)"
      cmp -s ${../tools/dependency-audit.rs} "$root/tools/dependency-audit.rs" || { echo "stale normalizer source" >&2; exit 1; }
      cmp -s ${../scripts/dependency-audit.sh} "$root/scripts/dependency-audit.sh" || { echo "stale audit capture source" >&2; exit 1; }
      exec bash "$(git rev-parse --show-toplevel)/scripts/dependency-audit.sh" "$@"
    '';
  };
  controlsApp = pkgs.writeShellApplication {
    name = "molten-dependency-audit-controls";
    inherit runtimeInputs;
    text = ''
      export MOLTEN_AUDIT_SCANNER=${scanner}/bin/cargo-audit
      export MOLTEN_AUDIT_NORMALIZER_BIN=${normalizer}/bin/molten-dependency-audit-normalizer
      root="$(git rev-parse --show-toplevel)"
      cmp -s ${../tools/dependency-audit.rs} "$root/tools/dependency-audit.rs" || { echo "stale normalizer source" >&2; exit 1; }
      cmp -s ${../config/dependency-audit/controls.rs} "$root/config/dependency-audit/controls.rs" || { echo "stale normalizer controls" >&2; exit 1; }
      cmp -s ${../config/dependency-audit/tests.sh} "$root/config/dependency-audit/tests.sh" || { echo "stale control runner" >&2; exit 1; }
      exec bash "$(git rev-parse --show-toplevel)/config/dependency-audit/tests.sh" "$@"
    '';
  };
  fixtureCheck = pkgs.runCommand "aspen-dependency-audit-scanner-fixture" {
    nativeBuildInputs = [ scanner normalizer pkgs.git pkgs.jq ];
  } ''
    set -euo pipefail
    export HOME="$TMPDIR/home" CARGO_HOME="$TMPDIR/cargo-home"
    mkdir -p "$HOME" "$CARGO_HOME" "$out" database
    cp -R ${../config/dependency-audit/fixtures/advisory-db}/. database/
    git -C database init -q
    git -C database add .
    GIT_AUTHOR_DATE=2026-11-13T00:00:00Z GIT_COMMITTER_DATE=2026-11-13T00:00:00Z \
      git -C database -c user.name=fixture -c user.email=fixture@example.invalid commit -qm fixture
    molten-dependency-audit-normalizer --self-test > "$out/normalizer-controls.txt"
    status=0
    # --stale is confined to this synthetic, frozen test DB, never the real runner.
    cargo-audit audit --db "$PWD/database" --file ${../config/dependency-audit/fixtures/affected-lockfile.toml} \
      --no-fetch --no-yanked --stale --json > "$out/scanner.json" 2> "$out/scanner.stderr" || status=$?
    test "$status" = 1
    jq -e '.vulnerabilities.list | any(.advisory.id == "RUSTSEC-2026-9999" and .package.name == "time")' \
      "$out/scanner.json" > /dev/null
  '';
in { inherit scanner normalizer app controlsApp fixtureCheck; }
