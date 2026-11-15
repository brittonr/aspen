# Optional experiment only. Root imports this with its existing source/dependency
# overrides; neither the default workspace nor the Octet cohort is changed.
# r[impl molten.sample_pgo.cohort]
# r[impl molten.sample_pgo.non_claim]
{ pkgs, unit2nix, source, extraCrateOverrides }:
assert pkgs.stdenv.hostPlatform.system == "x86_64-linux";
let
  lib = pkgs.lib;
  system = pkgs.stdenv.hostPlatform.system;
  cohort = import ./sample-pgo-toolchain.nix { inherit pkgs; };
  plan = builtins.fromJSON (builtins.readFile "${source}/build-plan.json");
  moltenFeatures = plan.crates.${plan.workspaceMembers.molten}.features or [ ];
  flags = [
    "-Copt-level=3"
    "-Ccodegen-units=1"
    "-Clto=off"
    "-Ctarget-cpu=x86-64"
    "-Ctarget-feature=+sse2"
    "-Cdebuginfo=line-tables-only"
    "-Cforce-frame-pointers=yes"
    "-Clink-arg=-Wl,--build-id=sha1"
    "-Zunstable-options"
    "-Zdebuginfo-for-profiling"
  ];
  policy = ../config/sample-pgo/experiment.json;
  mkBuild = bundle:
    let
      overrides = lib.genAttrs (lib.unique (map (c: c.crateName) (builtins.attrValues plan.crates)))
        (name: attrs:
          let original = (extraCrateOverrides.${name} or (_: { })) attrs;
          in original // {
            extraRustcOpts = (original.extraRustcOpts or [ ]) ++ flags
              ++ lib.optional (bundle != null) "-Cprofile-sample-use=${bundle}/sample.prof";
            dontStrip = true;
            separateDebugInfo = false;
          });
      workspace = unit2nix.lib.${system}.buildFromUnitGraph {
        inherit pkgs;
        src = source;
        resolvedJson = "${source}/build-plan.json";
        rustToolchain = cohort.compiler;
        buildRustCrateForPkgs = p: p.buildRustCrate.override {
          rustc = cohort.compiler;
          cargo = cohort.compiler;
        };
        extraCrateOverrides = overrides;
      };
    in workspace.workspaceMembers.molten.build;
  baseline = mkBuild null;
  mkVariant = bundle:
    let
      binding = builtins.fromJSON (builtins.readFile "${bundle}/binding.json");
      valid = binding.schema == "molten.sample-pgo.profile-binding.v1"
        && binding.source == "${source}"
        && binding.compiler_revision == cohort.compilerRevision
        && binding.llvm_revision == cohort.llvmRevision
        && binding.flags == flags
        && binding.baseline == toString baseline
        && binding.baseline_sha256 == builtins.hashFile "sha256" "${baseline}/bin/molten"
        && binding.compiler_sha256 == builtins.hashFile "sha256" "${cohort.compiler}/bin/rustc"
        && binding.converter_sha256 == builtins.hashFile "sha256" "${cohort.converter}/bin/llvm-profgen"
        && binding.lock_sha256 == builtins.hashFile "sha256" "${source}/Cargo.lock"
        && binding.plan_sha256 == builtins.hashFile "sha256" "${source}/build-plan.json"
        && binding.policy_sha256 == builtins.hashFile "sha256" policy
        && binding.raw_sha256 == builtins.hashFile "sha256" "${bundle}/perf.data"
        && binding.association_sha256 == builtins.hashFile "sha256" "${bundle}/association.txt"
        && binding.profile_sha256 == builtins.hashFile "sha256" "${bundle}/sample.prof"
        && binding.application_samples >= (builtins.fromJSON (builtins.readFile policy)).minimum_application_samples;
      validatedProfile = pkgs.runCommand "molten-validated-sample-profile" {
        nativeBuildInputs = [ pkgs.python3 pkgs.time pkgs.binutils pkgs.linuxPackages.perf ];
      } ''
        python3 ${../tools/sample-pgo/experiment.py} validate-profile \
          --bundle ${bundle} --binary ${baseline}/bin/molten \
          --converter ${cohort.converter}/bin/llvm-profgen \
          --minimum-samples ${toString (builtins.fromJSON (builtins.readFile policy)).minimum_application_samples} \
          --out "$TMPDIR/profile-validation"
        mkdir "$out"
        cp ${bundle}/sample.prof "$out/sample.prof"
        cp -R "$TMPDIR/profile-validation" "$out/validation"
      '';
    in assert lib.assertMsg valid "sample-PGO profile/source/binary/compiler binding mismatch";
      mkBuild validatedProfile;
  config = pkgs.writeText "molten-sample-pgo-cohort.json" (builtins.toJSON {
    inherit flags;
    source = "${source}";
    compiler = "${cohort.compiler}/bin/rustc";
    converter = "${cohort.converter}/bin/llvm-profgen";
    compiler_revision = cohort.compilerRevision;
    llvm_revision = cohort.llvmRevision;
    baseline_expression = "(builtins.getFlake \"path:${source}\").legacyPackages.${system}.sample-pgo.baseline";
    candidate_expression = "(builtins.getFlake \"path:${source}\").legacyPackages.${system}.sample-pgo.mkVariant";
    policy = "${policy}";
  });
  app = pkgs.writeShellApplication {
    name = "molten-sample-pgo-experiment";
    runtimeInputs = [
      pkgs.python3 pkgs.nix pkgs.linuxPackages.perf pkgs.binutils
      pkgs.b3sum pkgs.time pkgs.coreutils
    ];
    text = ''
      exec python3 ${../tools/sample-pgo/experiment.py} run "$@" --cohort ${config}
    '';
  };
  consumerTests = pkgs.runCommand "molten-sample-pgo-consumer-tests" {
    nativeBuildInputs = [ pkgs.python3 pkgs.time pkgs.b3sum ];
    MOLTEN_SAMPLE_PGO_BINARY = "${baseline}/bin/molten";
  } ''
    mkdir -p tools/sample-pgo config/sample-pgo
    cp ${../tools/sample-pgo}/*.py tools/sample-pgo/
    cp ${policy} config/sample-pgo/experiment.json
    python3 tools/sample-pgo/test_experiment.py
    touch "$out"
  '';
in
assert lib.assertMsg (!(lib.any (f: lib.hasPrefix "profiler" f) moltenFeatures))
  "sample-PGO experiment requires the uninstrumented feature graph";
{
  inherit baseline mkVariant app cohort consumerTests;
}
