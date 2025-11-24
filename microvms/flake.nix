{
  description = "MVM-CI Worker MicroVMs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    microvm = {
      url = "github:astro/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    mvm-ci-src = {
      url = "path:..";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, microvm, mvm-ci-src }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
      };

      # Build mvm-ci worker binary from parent directory
      mvm-ci-worker = pkgs.rustPlatform.buildRustPackage {
        pname = "mvm-ci-worker";
        version = "0.1.0";
        src = mvm-ci-src;

        cargoLock = {
          lockFile = "${mvm-ci-src}/Cargo.lock";
        };

        buildInputs = with pkgs; [
          openssl
          pkg-config
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        # Build only the worker binary
        cargoBuildFlags = [ "--bin" "worker" ];

        meta = {
          description = "MVM-CI Worker Binary";
          homepage = "https://github.com/brittonr/mvm-ci";
        };
      };

    in
    {
      # Worker VM configuration
      packages.${system}.worker-vm = microvm.lib.buildMicroVM {
        inherit pkgs;

        config = { config, pkgs, ... }: {
          # MicroVM settings
          microvm = {
            # Use Firecracker hypervisor
            hypervisor = "firecracker";

            # Memory allocation (can be overridden via environment variable)
            mem = pkgs.lib.toInt (builtins.getEnv "MEMORY_MB" or "512");

            # vCPU count (can be overridden via environment variable)
            vcpu = pkgs.lib.toInt (builtins.getEnv "VCPUS" or "1");

            # Kernel selection
            kernel = null; # Use default kernel

            # Shares (Firecracker doesn't support 9p/virtiofs, use block devices if needed)
            shares = [];

            # Network configuration
            interfaces = [
              {
                type = "tap";
                id = "vm-${builtins.getEnv "VM_ID" or "worker"}";
                mac = "02:00:00:00:00:01";
              }
            ];

            # Enable serial console for debugging
            writableStoreOverlay = "/nix/.rw-store";
          };

          # System configuration
          networking.hostName = "worker-${builtins.getEnv "VM_ID" or "vm"}";
          networking.firewall.enable = false;

          # Essential system packages
          environment.systemPackages = with pkgs; [
            curl
            jq
            mvm-ci-worker
          ];

          # Create worker user
          users.users.worker = {
            isNormalUser = true;
            group = "worker";
            home = "/home/worker";
            createHome = true;
          };
          users.groups.worker = {};

          # Worker service that runs on boot
          systemd.services.mvm-ci-worker = {
            description = "MVM-CI Worker";
            wantedBy = [ "multi-user.target" ];
            after = [ "network.target" ];

            environment = {
              CONTROL_PLANE_TICKET = builtins.getEnv "CONTROL_PLANE_TICKET";
              JOB_PAYLOAD_PATH = builtins.getEnv "JOB_PAYLOAD_PATH" or "/tmp/job-payload.json";
              RUST_LOG = "info";
            };

            serviceConfig = {
              Type = "simple";
              User = "worker";
              Group = "worker";
              WorkingDirectory = "/home/worker";
              ExecStart = "${mvm-ci-worker}/bin/worker";
              Restart = "no"; # Don't restart - VM should exit after job completion
              StandardOutput = "journal";
              StandardError = "journal";
            };
          };

          # Systemd journal configuration
          services.journald.extraConfig = ''
            Storage=volatile
            ForwardToConsole=yes
          '';

          # Enable SSH for debugging (optional)
          services.openssh = {
            enable = false; # Disable by default for security
            settings = {
              PermitRootLogin = "no";
              PasswordAuthentication = false;
            };
          };

          # Minimal system configuration
          boot.kernelParams = [ "console=ttyS0" ];
          system.stateVersion = "24.05";
        };
      };

      # Convenience script to run the VM
      packages.${system}.run-vm = pkgs.writeShellScriptBin "run-vm" ''
        set -euo pipefail

        VM_ID=''${VM_ID:-worker-$(date +%s)}
        MEMORY_MB=''${MEMORY_MB:-512}
        VCPUS=''${VCPUS:-1}
        CONTROL_PLANE_TICKET=''${CONTROL_PLANE_TICKET:?CONTROL_PLANE_TICKET must be set}

        echo "Starting MVM-CI Worker VM: $VM_ID"
        echo "Memory: $MEMORY_MB MB"
        echo "vCPUs: $VCPUS"

        # Run the VM
        exec ${self.packages.${system}.worker-vm}/bin/microvm-run
      '';

      # Default package
      packages.${system}.default = self.packages.${system}.worker-vm;

      # Development shell
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          nix
          firecracker
        ];

        shellHook = ''
          echo "MVM-CI Worker MicroVM Development Environment"
          echo "Available commands:"
          echo "  nix build .#worker-vm    - Build worker VM"
          echo "  nix run .#run-vm         - Run worker VM"
        '';
      };
    };
}
