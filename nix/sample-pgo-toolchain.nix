# r[impl molten.sample_pgo.cohort]
{ pkgs }:
assert pkgs.stdenv.hostPlatform.system == "x86_64-linux";
let
  date = "2026-08-20";
  target = "x86_64-unknown-linux-gnu";
  components = {
    rustc = "14b7ef71e56ff722774e7b1dd89354a628f9e856f6b4e829a0d1b086450891b3";
    rust-std = "b702c297f09a7cd3dd39974ce7418996e94264770401339a051c35a25c1ce2d4";
    cargo = "d6e1d312ea3e0c82de09cb20c265a5eabaad10a36d7472ee8012eb7031229976";
  };
  archives = pkgs.lib.mapAttrsToList (name: sha256: pkgs.fetchurl {
    url = "https://static.rust-lang.org/dist/${date}/${name}-nightly-${target}.tar.xz";
    inherit sha256;
  }) components;
  compiler = pkgs.stdenv.mkDerivation {
    pname = "molten-sample-pgo-rust";
    version = "1.100.0-nightly-${date}";
    dontUnpack = true;
    dontStrip = true;
    nativeBuildInputs = [ pkgs.autoPatchelfHook ];
    buildInputs = [ pkgs.stdenv.cc.cc.lib pkgs.zlib pkgs.openssl pkgs.libxml2 ];
    installPhase = ''
      runHook preInstall
      for archive in ${pkgs.lib.concatStringsSep " " archives}; do
        mkdir component
        tar -xf "$archive" -C component --strip-components=1
        bash component/install.sh --prefix="$out" --disable-ldconfig
        rm -r component
      done
      runHook postInstall
    '';
    meta.platforms = [ "x86_64-linux" ];
  };
  llvmRevision = "21cf28432798952d942bacc6bcee3a328faa3638";
  # The immutable Git object is rustc's own llvm-project submodule, not an
  # ambient LLVM release. No invented fixed-output hash is used.
  llvmSource = builtins.fetchGit {
    url = "https://github.com/rust-lang/llvm-project.git";
    rev = llvmRevision;
    allRefs = true;
  };
  converter = pkgs.stdenv.mkDerivation {
    pname = "molten-sample-pgo-llvm-profgen";
    version = llvmRevision;
    src = llvmSource;
    sourceRoot = "source/llvm";
    nativeBuildInputs = [ pkgs.cmake pkgs.ninja pkgs.python3 ];
    buildInputs = [ pkgs.zlib ];
    cmakeFlags = [
      "-GNinja"
      "-DCMAKE_BUILD_TYPE=Release"
      "-DLLVM_TARGETS_TO_BUILD=X86"
      "-DLLVM_INCLUDE_TESTS=OFF"
      "-DLLVM_INCLUDE_BENCHMARKS=OFF"
      "-DLLVM_ENABLE_TERMINFO=OFF"
      "-DLLVM_ENABLE_ZSTD=OFF"
      "-DLLVM_ENABLE_LIBXML2=OFF"
    ];
    buildPhase = ''ninja -j "$NIX_BUILD_CORES" llvm-profgen'';
    installPhase = ''install -Dm755 bin/llvm-profgen "$out/bin/llvm-profgen"'';
    meta.platforms = [ "x86_64-linux" ];
  };
in {
  inherit compiler converter llvmRevision;
  compilerRevision = "f7d782a3be46d6bb4b9792fe69a61db389ba1769";
  manifestUrl = "https://static.rust-lang.org/dist/2026-08-20/channel-rust-nightly.toml";
}
