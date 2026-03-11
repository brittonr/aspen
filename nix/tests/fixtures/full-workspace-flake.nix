{
  description = "Aspen full workspace — 80-crate self-build verification";
  inputs.nixpkgs.url = "nixpkgs";
  outputs = {
    nixpkgs,
    self,
    ...
  }: let
    pkgs = nixpkgs.legacyPackages.x86_64-linux;
  in {
    packages.x86_64-linux.default = pkgs.stdenv.mkDerivation {
      pname = "aspen-node";
      version = "0.1.0";

      # Source is the entire repo root. The workspace is in aspen/,
      # sibling dirs (iroh-proxy-utils, aspen-wasm-plugin, aspen-dns)
      # contain path deps referenced via ../iroh-proxy-utils etc.
      src = ./.;

      nativeBuildInputs = with pkgs; [
        cargo
        rustc
        perl
        pkg-config
        cmake
        python3
      ];

      buildInputs = with pkgs; [openssl];

      # The vendor directory must be pre-staged at ./aspen/vendor/ before
      # nix build runs. The CI pipeline uses a shell job to copy it from
      # the VM's shared nix store into the checkout directory.
      configurePhase = ''
        export HOME=$(mktemp -d)
        if [ ! -d aspen/vendor ]; then
          echo "ERROR: vendor directory not found at aspen/vendor/"
          echo "The CI pipeline should stage it before nix build."
          exit 1
        fi
        mkdir -p aspen/.cargo
        # crane's config.toml may be at vendor/config.toml or nested
        config=$(find aspen/vendor -name 'config.toml' -maxdepth 2 | head -1)
        if [ -z "$config" ]; then
          echo "ERROR: no config.toml found in vendor dir"
          ls -la aspen/vendor/
          exit 1
        fi
        cp "$config" aspen/.cargo/config.toml
        echo "Using cargo vendor config from $config"
      '';

      buildPhase = ''
        cd aspen
        cargo build --release \
          --bin aspen-node \
          --features ci,docs,hooks,shell-worker,automerge,secrets,forge,git-bridge,blob \
          --offline \
          2>&1
      '';

      installPhase = ''
        mkdir -p $out/bin
        cp aspen/target/release/aspen-node $out/bin/
      '';

      doCheck = false;
      OPENSSL_NO_VENDOR = "1";
    };
  };
}
