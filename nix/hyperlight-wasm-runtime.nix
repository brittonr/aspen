# Build the hyperlight wasm_runtime binary for x86_64-hyperlight-none.
#
# This is a bare-metal ELF binary that embeds wasmtime inside hyperlight
# micro-VMs. Normally built by hyperlight-wasm's build.rs via a nested
# cargo invocation — which breaks in Nix's sandbox (no network for the
# nested cargo to fetch deps).
#
# This derivation replicates the build:
# 1. Creates x86_64-hyperlight-none target spec from x86_64-unknown-none
# 2. Builds sysroot (core, alloc) via -Zbuild-std
# 3. Vendors wasm_runtime cargo deps from its Cargo.lock
# 4. Builds the wasm_runtime ELF binary
#
# The output binary is passed to the main aspen build via
# HYPERLIGHT_WASM_RUNTIME env var, skipping the nested cargo build.
{
  pkgs,
  lib,
  rustToolChain,
}: let
  # Extract wasm_runtime sources from hyperlight-wasm 0.12.0 crate
  hyperlightCrate = pkgs.fetchurl {
    url = "https://crates.io/api/v1/crates/hyperlight-wasm/0.12.0/download";
    sha256 = "sha256-ny6VY42o7qtfiE9hYJ3628Qbq9FhHDIirGHCyPw8+RE=";
    name = "hyperlight-wasm-0.12.0.tar.gz";
  };

  # Extracted sources with wasm_runtime and hyperlight_wasm_macro
  wasmRuntimeSrc = pkgs.runCommand "wasm-runtime-src" {} ''
    mkdir -p $out
    tar xzf ${hyperlightCrate} --strip-components=1 -C $out
    cd $out
    tar xf vendor.tar
    rm vendor.tar
    # Make the wasm_runtime Cargo.lock the top-level one so vendoring works
    cp wasm_runtime/Cargo.lock Cargo.lock
  '';

  # Vendor wasm_runtime deps using Nix's rustPlatform
  wasmRuntimeDeps = pkgs.rustPlatform.importCargoLock {
    lockFile = "${wasmRuntimeSrc}/wasm_runtime/Cargo.lock";
  };

  # Vendor standard library deps for -Zbuild-std (core, alloc, compiler_builtins)
  # Without this, -Zbuild-std tries to fetch libc etc. from crates.io
  rustSysroot = "${rustToolChain}/lib/rustlib/src/rust/library";
  stdDeps = pkgs.rustPlatform.importCargoLock {
    lockFile = "${rustSysroot}/Cargo.lock";
  };
in
  pkgs.stdenv.mkDerivation {
    pname = "hyperlight-wasm-runtime";
    version = "0.12.0";

    src = wasmRuntimeSrc;

    nativeBuildInputs = [
      rustToolChain
      pkgs.clang
      pkgs.llvm
      pkgs.lld
      pkgs.jq
    ];

    buildInputs = [
      pkgs.zlib
      pkgs.stdenv.cc.cc.lib
    ];

    dontConfigure = true;
    dontFixup = true;

    buildPhase = ''
          runHook preBuild

          set -euxo pipefail

          export HOME="$TMPDIR"
          TARGET="x86_64-hyperlight-none"

          # ── sysroot ────────────────────────────────────────────────────
          SYSROOT_DIR="$TMPDIR/sysroot"
          TRIPLET_DIR="$SYSROOT_DIR/lib/rustlib/$TARGET"
          BUILD_DIR="$SYSROOT_DIR/target"
          CRATE_DIR="$SYSROOT_DIR/crate"
          LIB_DIR="$TRIPLET_DIR/lib"

          mkdir -p "$TRIPLET_DIR" "$CRATE_DIR" "$LIB_DIR"

          echo "=== Generating target spec ==="
          # Generate target spec from x86_64-unknown-none + hyperlight mods
          # IMPORTANT: filename must be x86_64-hyperlight-none.json so cargo/rustc
          # uses "x86_64-hyperlight-none" as the target name for sysroot lookup.
          TARGET_SPEC="$TMPDIR/x86_64-hyperlight-none.json"
          rustc -Zunstable-options --print target-spec-json \
            --target x86_64-unknown-none \
            | jq 'del(.["rustc-abi"])
              | .["code-model"] = "small"
              | .features = "-mmx,+sse,+sse2,-sse3,-ssse3,-sse4.1,-sse4.2,-avx,-avx2,-soft-float"
              | .linker = "rust-lld"
              | .["linker-flavor"] = "gnu-lld"
              | .["pre-link-args"] = {"gnu-lld": ["-znostart-stop-gc"]}' \
            > "$TARGET_SPEC"

          echo "Target spec written to $TARGET_SPEC:"
          cat "$TARGET_SPEC"

          echo "=== Creating dummy sysroot crate ==="
          CARGO_VERSION=$(cargo version --verbose | grep "release:" | awk '{print $2}')

          cat > "$CRATE_DIR/Cargo.toml" << SYSEOF
      [package]
      name = "sysroot"
      version = "$CARGO_VERSION"
      edition = "2018"
      autotests = false
      autobenches = false

      [workspace]

      [lib]
      name = "sysroot"
      path = "lib.rs"
      SYSEOF

          echo '#![feature(no_core)]' > "$CRATE_DIR/lib.rs"
          echo '#![no_core]' >> "$CRATE_DIR/lib.rs"

          echo "Sysroot crate:"
          cat "$CRATE_DIR/Cargo.toml"
          cat "$CRATE_DIR/lib.rs"

          echo "=== Building sysroot for $TARGET ==="
          export CARGO_UNSTABLE_JSON_TARGET_SPEC=true

          # Provide vendored std deps for -Zbuild-std (no network in sandbox)
          # Must be in CARGO_HOME so the implicit -Zbuild-std workspace finds it
          export CARGO_HOME="$TMPDIR/cargo-home"
          mkdir -p "$CARGO_HOME"
          cat > "$CARGO_HOME/config.toml" << STDEOF
      [source.crates-io]
      replace-with = "vendored-sources"

      [source.vendored-sources]
      directory = "${stdDeps}"

      [net]
      offline = true
      STDEOF

          RUSTFLAGS="-Zunstable-options" \
          cargo build \
            --manifest-path="$CRATE_DIR/Cargo.toml" \
            --target="$TARGET_SPEC" \
            --target-dir="$BUILD_DIR" \
            -Zbuild-std=core,alloc \
            -Zbuild-std-features=compiler_builtins/mem \
            --release \
            -Zunstable-options

          echo "=== Collecting sysroot artifacts ==="
          # When using a .json target spec, cargo names the output dir after the
          # json filename stem ("target") rather than the triple name. Find it.
          RELEASE_DEPS=$(find "$BUILD_DIR" -path "*/release/deps" -type d | head -1)
          echo "Found release deps at: $RELEASE_DEPS"
          ls -la "$RELEASE_DEPS/" || echo "No release deps dir!"

          for file in "$RELEASE_DEPS"/*.rlib "$RELEASE_DEPS"/*.rmeta; do
            [ -f "$file" ] || continue
            filename=$(basename "$file")
            case "$filename" in libsysroot-*) ;; *) cp "$file" "$LIB_DIR/" ;; esac
          done

          echo "Sysroot libs:"
          ls -la "$LIB_DIR/"

          # ── C headers from hyperlight-guest-bin ─────────────────────────
          echo "=== Extracting C headers ==="
          INCLUDE_DIR="$SYSROOT_DIR/lib/rustlib/$TARGET/include"
          mkdir -p "$INCLUDE_DIR"

          GUEST_BIN="${wasmRuntimeDeps}/hyperlight-guest-bin-0.12.0"
          if [ -d "$GUEST_BIN/third_party" ]; then
            echo "Found hyperlight-guest-bin at: $GUEST_BIN"

            # Copy musl headers (provides setjmp.h, stdint.h, etc.)
            for dir in third_party/printf third_party/musl/include \
                       third_party/musl/arch/generic third_party/musl/arch/x86_64 \
                       third_party/musl/src/internal; do
              [ -d "$GUEST_BIN/$dir" ] || continue
              find "$GUEST_BIN/$dir" -name "*.h" | while read -r src; do
                rel="''${src#$GUEST_BIN/$dir/}"
                dst="$INCLUDE_DIR/$rel"
                mkdir -p "$(dirname "$dst")"
                cp "$src" "$dst"
              done
            done

            echo "Headers extracted:"
            find "$INCLUDE_DIR" -name "*.h" | wc -l
            echo "setjmp.h: $(find "$INCLUDE_DIR" -name "setjmp.h")"
          else
            echo "ERROR: hyperlight-guest-bin not found at $GUEST_BIN"
            ls "${wasmRuntimeDeps}/" | head -10
            exit 1
          fi

          # ── build wasm_runtime ──────────────────────────────────────────
          echo "=== Building wasm_runtime ==="
          cd wasm_runtime

          # Point cargo at vendored deps for wasm_runtime (overrides CARGO_HOME)
          mkdir -p .cargo
          cat > .cargo/config.toml << CARGOEOF
      [source.crates-io]
      replace-with = "vendored-sources"

      [source.vendored-sources]
      directory = "${wasmRuntimeDeps}"

      [net]
      offline = true
      CARGOEOF

          export RUSTFLAGS="--sysroot $SYSROOT_DIR --cfg=hyperlight --check-cfg=cfg(hyperlight) -Clink-args=-eentrypoint -Zunstable-options"
          export CC_x86_64_hyperlight_none="${pkgs.clang}/bin/clang"
          export AR_x86_64_hyperlight_none="${pkgs.llvm}/bin/llvm-ar"
          export CFLAGS_x86_64_hyperlight_none="--target=x86_64-unknown-linux-none -U__linux__ -fPIC -fno-stack-protector -mno-red-zone -nostdinc -DHYPERLIGHT=1 -D__HYPERLIGHT__=1 -isystem $INCLUDE_DIR"
          export BINDGEN_EXTRA_CLANG_ARGS="$CFLAGS_x86_64_hyperlight_none"
          unset RUSTC_WORKSPACE_WRAPPER

          cargo build \
            --target="$TARGET_SPEC" \
            --target-dir="$TMPDIR/wasm-target" \
            --release \
            -Zunstable-options

          echo "=== Build complete ==="
          # Find the built binary (dir may be named "target" from json spec)
          WASM_RUNTIME=$(find "$TMPDIR/wasm-target" -name "wasm_runtime" -type f -executable | head -1)
          echo "Built wasm_runtime at: $WASM_RUNTIME"
          ls -la "$WASM_RUNTIME"

          # Stash for install phase
          cp "$WASM_RUNTIME" "$TMPDIR/wasm_runtime_final"

          runHook postBuild
    '';

    installPhase = ''
      mkdir -p $out
      cp "$TMPDIR/wasm_runtime_final" $out/wasm_runtime
    '';

    meta = with lib; {
      description = "Hyperlight WASM runtime binary for x86_64-hyperlight-none";
      license = licenses.asl20;
      platforms = ["x86_64-linux"];
    };
  }
