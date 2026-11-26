# Hermetic optional-feature rail over the vendored Cargo.lock closure (no network).
#
# 1. Every optional `molten` feature (all of `[features]` except the test-only `test-fixtures`) must be
#    listed here; an unlisted or stale feature fails closed.
# 2. Each feature compiles in isolation with every target (`cargo check --all-targets`).
# 3. The union of self-contained features builds every test target of `molten` once (`--tests`: library,
#    binaries, and integration tests). `scripts/ci-coverage.py feature-tests` runs, per target, exactly the
#    tests that only exist under those features (the set difference against that target's default-feature
#    unit2nix test binary). Every feature-gated target must execute at least one test: integration targets
#    gated by `required-features` or a crate-level `#![cfg(feature = ...)]` are discovered from
#    `cargo metadata`, and a target that cannot execute here needs an entry in
#    `config/testing/feature-gated-targets.toml`.
# 4. The DoltLite oracle integration test runs against the Nix-built remotes-disabled cohort, and its
#    output must show every ignored-test inventory entry railed to `nix:feature-matrix` passed.
#
# r[impl molten.testing.feature_and_ignore_ci_accounting]
{
  pkgs,
  rustToolchain,
  source,
  vendor,
  doltliteOracle,
  defaultTestBinaries,
  extraInputs ? [ ],
}:
let
  isolatedFeatures = [
    "chaoscontrol-conformance"
    "component-rpc"
    "component-rpc-telemetry"
    "content-store-object-storage"
    "doltlite-oracle"
    "executable-extents"
    "job-dag-construction"
    "marble-object-store"
    "profiler"
    "profiler-alloc"
    "profiler-disabled"
    "profiler-perf"
    "spacewasm-pilot"
    "world-snapshot-vm-cohort"
  ];
  # `profiler-disabled` selects flux-profiler's no-op mode and is exclusive with the recording modes.
  unionFeatures = pkgs.lib.concatStringsSep "," (
    builtins.filter (feature: feature != "profiler-disabled") isolatedFeatures
  );
  doltliteTest = "live_doltlite_oracle_covers_normalized_positive_and_negative_cases";
in
pkgs.runCommand "molten-feature-matrix"
  {
    nativeBuildInputs = [
      rustToolchain
      pkgs.pkg-config
      pkgs.clang
      pkgs.perl
      pkgs.python3
      pkgs.b3sum
    ]
    ++ extraInputs;
    CARGO_INCREMENTAL = "0";
    CARGO_NET_OFFLINE = "true";
    # reqwest's rustls platform verifier loads trust roots when a client is built, even for the loopback
    # HTTP endpoints these tests use. The sandbox has no system store, so point it at the pinned CA bundle.
    SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
    expectedFeatures = pkgs.lib.concatStringsSep " " isolatedFeatures;
  }
  ''
    set -euo pipefail
    export HOME="$TMPDIR/home"
    mkdir -p "$HOME" "$out"
    cp -r ${source} source
    chmod -R u+w source
    cd source
    export CARGO_HOME="$PWD/.cargo-home"
    mkdir -p "$CARGO_HOME"
    cp ${vendor}/config.toml "$CARGO_HOME/config.toml"
    export CARGO_TARGET_DIR="$NIX_BUILD_TOP/target"

    python3 - <<'PY'
    import os
    import sys
    import tomllib

    declared = set(tomllib.load(open("Cargo.toml", "rb"))["features"]) - {"default", "test-fixtures"}
    expected = set(os.environ["expectedFeatures"].split())
    missing, stale = sorted(declared - expected), sorted(expected - declared)
    for feature in missing:
        print(f"feature-matrix-unlisted-feature:{feature}", file=sys.stderr)
    for feature in stale:
        print(f"feature-matrix-stale-feature:{feature}", file=sys.stderr)
    sys.exit(1 if missing or stale else 0)
    PY

    for feature in $expectedFeatures; do
      echo "feature-matrix: cargo check --all-targets --features $feature"
      cargo check --locked --offline -p molten --all-targets --features "$feature"
      echo "$feature" >> "$out/compiled-features.txt"
    done

    cargo metadata --locked --offline --no-deps --format-version 1 > "$TMPDIR/metadata.json"
    cargo test --locked --offline -p molten --tests --features '${unionFeatures}' --no-run \
      --message-format=json > "$TMPDIR/union-artifacts.json"
    python3 scripts/ci-coverage.py feature-tests --metadata "$TMPDIR/metadata.json" \
      --artifacts "$TMPDIR/union-artifacts.json" --default-binaries ${defaultTestBinaries}/tests \
      --out "$out/feature-gated"

    doltlite_workspace="$TMPDIR/doltlite-workspace"
    mkdir -p "$doltlite_workspace"
    MOLTEN_DOLTLITE_ORACLE_BIN=${doltliteOracle}/bin/doltlite \
    MOLTEN_DOLTLITE_ORACLE_WORKSPACE="$doltlite_workspace" \
    MOLTEN_DOLTLITE_BUILD_REF="blake3:$(b3sum --no-names ${doltliteOracle}/bin/doltlite)" \
    MOLTEN_DOLTLITE_ADAPTER_REF="blake3:$(b3sum --no-names src/world_state_oracle/process.rs)" \
      cargo test --locked --offline -p molten --features '${unionFeatures}' --test doltlite_oracle \
        -- --ignored --exact ${doltliteTest} | tee "$out/doltlite-oracle-run.txt"
    python3 scripts/ci-coverage.py ignored-rail-evidence --rail nix:feature-matrix \
      --log "$out/doltlite-oracle-run.txt"
  ''
