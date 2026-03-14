# Development shells — fuzz and verus variants.
#
# The default devShell remains in flake.nix because it depends on
# craneLib, verus, VM helpers, and other build artifacts defined
# in the main perSystem let-block. It will migrate here once the
# rust.nix module exports those values via flake-parts options.
{inputs, ...}: {
  perSystem = {system, ...}: let
    pkgs = import inputs.nixpkgs {
      inherit system;
      overlays = [(import inputs.rust-overlay)];
    };
  in {
    devShells.fuzz = let
      rustNightly = pkgs.rust-bin.nightly.latest.default.override {
        extensions = ["rust-src" "llvm-tools-preview"];
      };
    in
      pkgs.mkShell {
        buildInputs = with pkgs; [
          rustNightly
          cargo-fuzz
          llvmPackages_latest.llvm
          llvmPackages_latest.libclang
          pkg-config
          openssl.dev
        ];

        LIBCLANG_PATH = "${pkgs.llvmPackages_latest.libclang.lib}/lib";
        LLVM_COV = "${pkgs.llvmPackages_latest.llvm}/bin/llvm-cov";
        LLVM_PROFDATA = "${pkgs.llvmPackages_latest.llvm}/bin/llvm-profdata";

        shellHook = ''
          echo "Fuzzing development environment ready!"
          echo ""
          echo "Nix apps (from project root):"
          echo "  nix run .#fuzz           # Parallel fuzzing (1hr/target)"
          echo "  nix run .#fuzz-quick     # Quick smoke test (5min/target)"
          echo "  nix run .#fuzz-overnight # Overnight run (8hr, 4 parallel targets)"
          echo "  nix run .#fuzz-intensive # Full campaign (6hr/target, sequential)"
          echo "  nix run .#fuzz-coverage  # Generate coverage report"
          echo "  nix run .#fuzz-corpus    # Generate seed corpus"
          echo ""
          echo "Manual commands (in this shell):"
          echo "  cargo fuzz list"
          echo "  cargo fuzz run fuzz_raft_rpc --sanitizer none -- -max_total_time=60"
          echo "  cargo fuzz coverage fuzz_raft_rpc"
          echo ""
          echo "Outputs saved to:"
          echo "  fuzz/corpus/          # Coverage-increasing inputs"
          echo "  fuzz/artifacts/       # Crash-triggering inputs"
          echo "  fuzz/dictionaries/    # Auto-generated dictionaries"
          echo "  fuzz/coverage/        # Coverage reports"
        '';
      };
  };
}
