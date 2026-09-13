{
  pkgs,
  bootstrapToolchain,
  cargoSource,
}:
let
  cargoVersion = "1.98.0-nightly";
  patchDescription = "molten-pathless-package-id";
  bootstrapPlatform = pkgs.makeRustPlatform {
    cargo = bootstrapToolchain;
    rustc = bootstrapToolchain;
  };
  cargo = bootstrapPlatform.buildRustPackage {
    pname = "molten-cargo-pathless";
    version = cargoVersion;
    src = cargoSource;
    cargoDeps =
      (bootstrapPlatform.importCargoLock.override {
        # Change the download route, not the registry identity or locked checksum.
        fetchurl =
          args:
          let
            legacyDownloadRoot = "https://crates.io/api/v1/crates/";
            registryDownloadRoot = "https://static.crates.io/crates/";
          in
          pkgs.fetchurl (
            args
            // pkgs.lib.optionalAttrs (pkgs.lib.hasPrefix legacyDownloadRoot args.url) {
              url = registryDownloadRoot + pkgs.lib.removePrefix legacyDownloadRoot args.url;
            }
          );
      })
        {
          lockFile = cargoSource + "/Cargo.lock";
        };
    patches = [ ./pathless-package-id.patch ];
    nativeBuildInputs = [
      pkgs.pkg-config
      pkgs.cmake
      pkgs.b3sum
    ];
    buildInputs = [
      pkgs.openssl
    ]
    ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [ pkgs.libiconv ];
    cargoBuildFlags = [
      "-p"
      "cargo"
      "--bin"
      "cargo"
    ];
    cargoTestFlags = [
      "-p"
      "cargo-util-schemas"
      "--all-features"
    ];
    env = {
      CFG_RELEASE = cargoVersion;
      CFG_RELEASE_CHANNEL = "nightly";
      CFG_VER_DESCRIPTION = patchDescription;
    };
    postPatch = ''
      b3sum --check ${./patched-sources.b3}
      printf '%s\n' '${cargoSource.rev}' '${cargoSource.shortRev}' '2026-05-15' > git-commit-info
    '';
    postInstall = ''
      mkdir -p "$out/share/molten-cargo" "$out/share/licenses/cargo"
      cp LICENSE-MIT LICENSE-APACHE LICENSE-THIRD-PARTY "$out/share/licenses/cargo/"
      cp ${./pathless-package-id.patch} "$out/share/molten-cargo/pathless-package-id.patch"
      cp ${./patched-sources.b3} "$out/share/molten-cargo/patched-sources.b3"
      printf '%s\n' '${cargoSource.rev}' > "$out/share/molten-cargo/upstream-revision"
      "$out/bin/cargo" --version | grep -F '${patchDescription}'
    '';
    meta = {
      description = "Pinned Cargo with explicit pathless package-ID round-trips";
      homepage = "https://github.com/rust-lang/cargo";
      license = with pkgs.lib.licenses; [
        mit
        asl20
      ];
      mainProgram = "cargo";
    };
  };
  toolchain = pkgs.symlinkJoin {
    name = "molten-rust-with-pathless-cargo";
    paths = [
      cargo
      bootstrapToolchain
    ];
    passthru = (bootstrapToolchain.passthru or { }) // {
      inherit cargo;
      unpatchedToolchain = bootstrapToolchain;
    };
    meta = bootstrapToolchain.meta;
  };
  selectionCheck =
    pkgs.runCommand "molten-cargo-toolchain-selection"
      {
        nativeBuildInputs = [
          toolchain
          pkgs.diffutils
        ];
      }
      ''
        test "$(readlink -f "$(command -v cargo)")" = "$(readlink -f ${cargo}/bin/cargo)"
        for tool in rustc rustfmt cargo-clippy clippy-driver; do
          test "$(readlink -f "$(command -v "$tool")")" = "$(readlink -f ${bootstrapToolchain}/bin/"$tool")"
        done
        diff -qr ${bootstrapToolchain}/lib/rustlib ${toolchain}/lib/rustlib
        mkdir -p "$out"
        ${bootstrapToolchain}/bin/cargo --version > "$out/bootstrap-cargo-version.txt"
        if grep -Fq '${patchDescription}' "$out/bootstrap-cargo-version.txt"; then
          echo 'the unpatched Cargo unexpectedly has the patch identity' >&2
          exit 1
        else
          test "$?" -eq 1
        fi
        cargo -Vv > "$out/cargo-version.txt"
        rustc -Vv > "$out/rustc-version.txt"
        grep -F '${patchDescription}' "$out/cargo-version.txt"
      '';
in
{
  inherit cargo toolchain selectionCheck;
}
