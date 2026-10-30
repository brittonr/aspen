# Product-owned profile over Nix build facts; signing remains an independent operator effect.
{ pkgs, source, buildCandidate, buildGate, rustToolchain, gateCompiler }:
let
  candidate = buildCandidate source;
  gates = buildGate source;
  policy = source + "/dylint.toml";
  profile = source + "/config/startup-source-profile.ncl";
in
pkgs.runCommand "molten-node-source-binding-v1"
  {
    nativeBuildInputs = [ pkgs.python3 pkgs.b3sum pkgs.nix ];
    exportReferencesGraph = [
      "candidate-build-closure" candidate.drvPath
      "compiler-closure" rustToolchain
      "runtime-closure" candidate
      "gate-build-closure" gates.drvPath
      "gate-compiler-closure" gateCompiler
      "gate-runtime-closure" gates
    ];
    # An input-addressed output is required: CA trust must not bypass an actual signature.
    preferLocalBuild = true;
    allowSubstitutes = false;
  }
  ''
    mkdir -p retained
    cp -R ${gates}/primary retained/primary
    cp -R ${gates}/library retained/library
    chmod -R u+w retained
    cd ${source}
    ${candidate}/bin/molten test octet gate \
      --artifacts "$NIX_BUILD_TOP/retained/primary" --profile strict-ci \
      --receipt-out "$NIX_BUILD_TOP/retained/gate-receipt.preserves"
    ${candidate}/bin/molten test octet remediation plan \
      --artifacts "$NIX_BUILD_TOP/retained/primary" \
      --lib-artifacts "$NIX_BUILD_TOP/retained/library" \
      --focused-object-corpus "$NIX_BUILD_TOP/retained/primary/object-corpus-receipt.json" \
      --receipt-out "$NIX_BUILD_TOP/retained/remediation-plan.preserves"
    python3 ${./../scripts/produce-startup-source-binding.py} \
      --candidate ${candidate} --source ${source} --gates "$NIX_BUILD_TOP/retained" \
      --toolchain ${rustToolchain} --closure "$NIX_BUILD_TOP/candidate-build-closure" \
      --compiler-closure "$NIX_BUILD_TOP/compiler-closure" --runtime-closure "$NIX_BUILD_TOP/runtime-closure" \
      --gate-closure "$NIX_BUILD_TOP/gate-build-closure" \
      --gate-compiler-closure "$NIX_BUILD_TOP/gate-compiler-closure" \
      --gate-runtime-closure "$NIX_BUILD_TOP/gate-runtime-closure" \
      --derivation ${candidate.drvPath} --policy ${policy} --profile ${profile}
  ''
