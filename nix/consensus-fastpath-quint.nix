{ pkgs }:
let
  version = "0.33.1";
  revision = "3eaa85fc01820ea25bd7cb7baee0bb8d5c02c8b0";
  sourceArchive = pkgs.fetchurl {
    url = "https://codeload.github.com/quint-co/quint/legacy.tar.gz/${revision}";
    hash = "sha256-Keqk2eED59K/T3zkhE9xgRA/sgSiFZuSUPwMxidLPKg=";
  };
  unpacked = pkgs.runCommand "quint-source-${revision}" {
    nativeBuildInputs = [ pkgs.gnutar pkgs.gzip ];
  } ''
    mkdir -p "$out"
    tar -xzf ${sourceArchive} --strip-components=1 -C "$out"
  '';
  raw = pkgs.buildNpmPackage {
    pname = "quint-cli";
    inherit version;
    src = "${unpacked}/quint";
    npmDeps = pkgs.importNpmLock { npmRoot = "${unpacked}/quint"; };
    npmConfigHook = pkgs.importNpmLock.npmConfigHook;
    npmBuildScript = "compile";
    npmFlags = [ "--ignore-scripts" ];
    postInstall = ''
      mkdir -p "$out/share/licenses/quint"
      cp ${unpacked}/LICENSE "$out/share/licenses/quint/LICENSE"
      if test -f ${unpacked}/NOTICE; then
        cp ${unpacked}/NOTICE "$out/share/licenses/quint/NOTICE"
      fi
    '';
    meta.license = pkgs.lib.licenses.asl20;
  };
  # quint-connect 0.1.2 does not pass --backend. Force the pinned publication's
  # TypeScript evaluator, avoiding an ambient/downloaded Rust evaluator cohort.
  wrapped = pkgs.writeShellApplication {
    name = "quint";
    text = ''
      case "''${1:-}" in
        run|test|repl) exec ${raw}/bin/quint "$@" --backend=typescript ;;
        *) exec ${raw}/bin/quint "$@" ;;
      esac
    '';
  };
in {
  quint = wrapped.overrideAttrs (old: {
    inherit version;
    passthru = (old.passthru or { }) // { sourceRevision = revision; evaluator = "typescript"; };
  });
  quintConnect = {
    version = "0.1.2";
    git = "https://github.com/quint-co/quint-connect.git";
    rev = "4f018f54fc7dd4cef341d10111427bab59d3b307";
  };
  mutateSource = src: pkgs.applyPatches {
    name = "aspen-fastpath-mixed-view-guard-mutant";
    inherit src;
    patches = [ ../verification/tla/mutants/mixed-view-guard.patch ];
  };
  mkConformanceCheck = { src, normalTests, mutantTests, rustLibDir }:
    pkgs.runCommand "aspen-consensus-fastpath-quint-conformance" {
      nativeBuildInputs = [ wrapped pkgs.nodejs ];
      inherit src;
      meta.timeout = 1200;
    } ''
      set -euo pipefail
      export HOME="$TMPDIR/home"
      mkdir -p "$HOME"
      export LD_LIBRARY_PATH="${rustLibDir}:''${LD_LIBRARY_PATH:-}"
      cp -R "$src" source
      chmod -R u+w source
      cd source
      normal=( ${normalTests}/tests/fastpathquint* )
      mutant=( ${mutantTests}/tests/fastpathquint* )
      test "''${#normal[@]}" -eq 1 && test -x "''${normal[0]}"
      test "''${#mutant[@]}" -eq 1 && test -x "''${mutant[0]}"
      node verification/tla/quint-campaign.mjs "$TMPDIR/evidence" "''${normal[0]}" "''${mutant[0]}"
      mkdir -p "$out"
      cp -R "$TMPDIR/evidence/." "$out/"
    '';
}
