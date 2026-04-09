{
  lib,
  system,
  pkgs,
  bins,
  inventory,
}: let
  repoRoot = ../../..;

  packagePreset = name: value:
    if name == "aspenNodePackage"
    then
      if value == "full-aspen-node"
      then bins.full-aspen-node
      else if value == "ci-aspen-node"
      then bins.ci-aspen-node
      else throw "unsupported generated aspenNodePackage preset: ${value}"
    else if name == "aspenCliPackage"
    then
      if value == "full-aspen-cli"
      then bins.full-aspen-cli
      else if value == "ci-aspen-cli"
      then bins.ci-aspen-cli
      else throw "unsupported generated aspenCliPackage preset: ${value}"
    else throw "unsupported generated package preset key: ${name}";

  suiteImportArgs = suite: let
    mappedPresets = lib.mapAttrs packagePreset suite.target.package_presets;
  in
    {inherit pkgs;} // mappedPresets;

  generatedVmSuites =
    builtins.filter (
      suite:
        system
        == "x86_64-linux"
        && suite.layer == "vm"
        && suite.target.kind == "nix-build"
        && suite.target.register_flake_check
    )
    inventory.suites;
in
  builtins.listToAttrs (map
    (suite: {
      name = suite.target.check_attr;
      value = import (repoRoot + "/${suite.target.nix_file}") (suiteImportArgs suite);
    })
    generatedVmSuites)
