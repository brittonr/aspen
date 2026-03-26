{
  description = "Aspen example clan — 3-node physical cluster on GMK mini PCs";

  inputs = {
    clan-core.url = "https://git.clan.lol/clan/clan-core/archive/main.tar.gz";
    nixpkgs.follows = "clan-core/nixpkgs";

    aspen = {
      url = "path:../";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    clan-core,
    nixpkgs,
    aspen,
    ...
  } @ inputs: let
    clan = clan-core.lib.clan {
      inherit self;
      imports = [./clan.nix];
    };
  in {
    inherit (clan.config) nixosConfigurations clanInternals;
    clan = clan.config;

    devShells =
      nixpkgs.lib.genAttrs
      ["x86_64-linux" "aarch64-linux"]
      (system: {
        default = nixpkgs.legacyPackages.${system}.mkShell {
          packages = [
            clan-core.packages.${system}.clan-cli
          ];
        };
      });
  };
}
