{
  lib,
  stdenv,
  fetchFromGitHub,
  rustToolchain,
  cacert,
  git,
}:

let
  expectedMarker = "ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED";
  targetTriple = "x86_64-unknown-hermit";
  sourceRev = "8ca08f93a2294ce8283f0f9226ae6b21aea3d20d";
  uhyveSrc = fetchFromGitHub {
    owner = "hermit-os";
    repo = "uhyve";
    rev = sourceRev;
    hash = "sha256-IgkU6ENznhYJLI31eZ+S5VMmv2gkVuMAuyB9mVh4fh0=";
  };
  hermitSrc = fetchFromGitHub {
    owner = "hermit-os";
    repo = "hermit-rs";
    rev = "0e68850e7b848656f8704b6c1fdc3a09685cb4de";
    fetchSubmodules = true;
    hash = "sha256-bI2dHQqavQZs2XEOtWtMfV1a+TksCW+BV+l2Yrf5bW8=";
  };
  cargoVendor = stdenv.mkDerivation {
    pname = "hermit-uhyve-marker-cargo-vendor";
    version = "0.1.0-unstable-2026-05-08";
    src = uhyveSrc;

    nativeBuildInputs = [ rustToolchain git ];

    SSL_CERT_FILE = "${cacert}/etc/ssl/certs/ca-bundle.crt";
    GIT_SSL_CAINFO = "${cacert}/etc/ssl/certs/ca-bundle.crt";

    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
    outputHash = "sha256-Kf2deIro+CwjvwdkpKvonM/5esZLbv7u4/YavFMV3tk=";

    dontFixup = true;

    postPatch = ''
      substituteInPlace tests/test-kernels/Cargo.toml \
        --replace-fail 'git = "https://github.com/hermit-os/hermit-rs.git"' 'path = "${hermitSrc}/hermit"' \
        --replace-fail 'tag = "hermit-0.13.2"' ""
      cat >> tests/test-kernels/Cargo.toml <<'TOML'

      [workspace]
      TOML
    '';

    buildPhase = ''
      runHook preBuild
      export HOME="$TMPDIR/home"
      export CARGO_HOME="$TMPDIR/cargo-home"
      mkdir -p "$HOME" "$CARGO_HOME"
      export RUSTC_WRAPPER=
      export CARGO_INCREMENTAL=0
      cargo vendor --manifest-path tests/test-kernels/Cargo.toml "$out" >/dev/null
      cargo vendor --manifest-path "$(rustc --print sysroot)/lib/rustlib/src/rust/library/Cargo.toml" "$TMPDIR/std-vendor" >/dev/null
      cargo vendor --manifest-path ${hermitSrc}/kernel/Cargo.toml "$TMPDIR/kernel-vendor" >/dev/null
      cargo vendor --manifest-path ${hermitSrc}/kernel/hermit-builtins/Cargo.toml "$TMPDIR/hermit-builtins-vendor" >/dev/null
      for vendor_dir in "$TMPDIR"/std-vendor "$TMPDIR"/kernel-vendor "$TMPDIR"/hermit-builtins-vendor; do
        for crate_dir in "$vendor_dir"/*; do
          base="$(basename "$crate_dir")"
          if [ -e "$out/$base" ]; then
            version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$crate_dir/Cargo.toml" | head -n1)"
            cp -R "$crate_dir" "$out/$base-$version"
          else
            cp -R "$crate_dir" "$out/$base"
          fi
        done
      done
      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall
      runHook postInstall
    '';
  };
in
stdenv.mkDerivation {
  pname = "hermit-uhyve-marker";
  version = "0.1.0-unstable-2026-05-08";

  src = uhyveSrc;

  nativeBuildInputs = [ rustToolchain git ];

  # Cargo.lock plus the fixed-output vendor derivation pins registry/git inputs.
  # This source-built fixture is prerequisite evidence only; it is not
  # runtime-host execution proof by itself.
  dontStrip = true;
  dontPatchELF = true;

  postPatch = ''
    substituteInPlace tests/test-kernels/Cargo.toml \
      --replace-fail 'git = "https://github.com/hermit-os/hermit-rs.git"' 'path = "../../hermit-src/hermit"' \
      --replace-fail 'tag = "hermit-0.13.2"' ""
    cp -R ${hermitSrc} hermit-src
    chmod -R u+w hermit-src
    cat >> tests/test-kernels/Cargo.toml <<'TOML'

    [workspace]
    TOML
    mkdir -p tests/test-kernels/src/bin .cargo
    cat > tests/test-kernels/src/bin/aspen_marker.rs <<'RS'
    #[cfg(target_os = "hermit")]
    use hermit as _;

    fn main() {
        println!("ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED");
    }
    RS
    cat > .cargo/config.toml <<'TOML'
    [source.crates-io]
    replace-with = "vendored-sources"

    [source."git+https://github.com/hermit-os/async-executor.git?branch=no_std"]
    git = "https://github.com/hermit-os/async-executor.git"
    branch = "no_std"
    replace-with = "vendored-sources"

    [source."git+https://github.com/hermit-os/safe-mmio?branch=be"]
    git = "https://github.com/hermit-os/safe-mmio"
    branch = "be"
    replace-with = "vendored-sources"

    [source.vendored-sources]
    directory = "${cargoVendor}"
    TOML
    mkdir -p hermit-src/.cargo
    cp .cargo/config.toml hermit-src/.cargo/config.toml
  '';

  buildPhase = ''
    runHook preBuild
    export HOME="$TMPDIR/home"
    export CARGO_HOME="$TMPDIR/cargo-home"
    mkdir -p "$HOME" "$CARGO_HOME" "$TMPDIR/bin"
    cat > "$TMPDIR/bin/rustup" <<'SH'
    #!${stdenv.shell}
    if [ "$1" = "target" ] && [ "$2" = "add" ] && [ "$3" = "x86_64-unknown-none" ]; then
      exit 0
    fi
    echo "unsupported rustup invocation: $*" >&2
    exit 1
    SH
    chmod +x "$TMPDIR/bin/rustup"
    export PATH="$TMPDIR/bin:$PATH"
    export RUSTC_WRAPPER=
    export CARGO_TARGET_X86_64_UNKNOWN_HERMIT_RUSTFLAGS="-C link-arg=--build-id=none -C metadata=aspenhermituhyvemarker"
    export CARGO_INCREMENTAL=0
    cargo build \
      --offline \
      --release \
      -Zbuild-std=std,panic_abort \
      --target ${targetTriple} \
      --bin aspen_marker \
      --manifest-path tests/test-kernels/Cargo.toml
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    image="tests/test-kernels/target/${targetTriple}/release/aspen_marker"
    test -f "$image"
    test -x "$image"

    mkdir -p "$out/bin" "$out/share/aspen/hermit-uhyve-marker"
    cp "$image" "$out/bin/aspen-hermit-uhyve-marker"
    chmod 0555 "$out/bin/aspen-hermit-uhyve-marker"

    cat > "$out/share/aspen/hermit-uhyve-marker/metadata.json" <<JSON
    {
      "schema": "aspen.hermit-uhyve-marker.v1",
      "source": "hermit-os/uhyve",
      "source_rev": "${sourceRev}",
      "target_triple": "${targetTriple}",
      "expected_marker": "${expectedMarker}",
      "image_path": "bin/aspen-hermit-uhyve-marker",
      "proof_boundary": "fixture-build-is-not-runtime-host-proof"
    }
    JSON

    runHook postInstall
  '';

  passthru = {
    inherit expectedMarker targetTriple;
    imagePath = "/bin/aspen-hermit-uhyve-marker";
    metadataPath = "/share/aspen/hermit-uhyve-marker/metadata.json";
  };

  meta = {
    description = "Reproducible Hermit marker image for Aspen Uhyve runtime-host proof reruns";
    homepage = "https://github.com/hermit-os/uhyve";
    license = lib.licenses.mit;
    platforms = [ "x86_64-linux" ];
  };
}
