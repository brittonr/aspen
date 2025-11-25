{
  description = "MVM-CI Worker MicroVMs using QEMU (no special permissions needed)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    microvm = {
      url = "github:microvm-nix/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, microvm }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      # Mock worker for testing VM infrastructure
      mvm-ci-worker = pkgs.writeShellScriptBin "worker" ''
        #!/usr/bin/env bash
        echo "===================="
        echo "MVM-CI Worker (Mock)"
        echo "===================="
        echo ""
        echo "Job ID: ''${JOB_ID:-unknown}"
        echo "Control Plane: ''${CONTROL_PLANE_TICKET:-not set}"
        echo ""
        echo "Job payload:"
        echo "$JOB_PAYLOAD" | ${pkgs.jq}/bin/jq '.' 2>/dev/null || echo "$JOB_PAYLOAD"
        echo ""

        # Simulate processing
        echo "Processing job..."
        for i in 1 2 3; do
          echo "  Step $i/3..."
          sleep 1
        done

        echo ""
        echo "✓ Job completed successfully"
        exit 0
      '';
    in
    {
      # Service VM configuration with QEMU and user networking
      nixosConfigurations.service-vm = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          microvm.nixosModules.microvm
          {
            microvm = {
              hypervisor = "qemu";  # Use QEMU instead of Cloud Hypervisor
              mem = 1024;
              vcpu = 2;

              # Enable balloon device for memory efficiency
              balloon = true;

              shares = [
                {
                  source = "/nix/store";
                  mountPoint = "/nix/.ro-store";
                  tag = "store";
                  proto = "9p";  # 9p works well with QEMU
                }
                {
                  source = "/var/lib/mvm-ci/jobs";  # Changed to use standard path
                  mountPoint = "/mnt/jobs";
                  tag = "jobs";
                  proto = "9p";
                }
              ];

              writableStoreOverlay = "/nix/.rw-store";

              # User networking - no special permissions needed
              interfaces = [{
                type = "user";
                id = "vm-net0";
                mac = "02:00:00:00:00:02";
              }];
            };

            boot.loader.grub.enable = false;

            # Kernel parameters for QEMU
            boot.kernelParams = [
              "console=ttyS0"
              "panic=1"
            ];

            # Minimal system services
            services.journald.console = "/dev/console";

            # Service VM control socket handler
            systemd.services.vm-control-handler = {
              description = "VM Control Socket Handler";
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];
              script = ''
                #!/bin/sh
                SOCKET_PATH="/var/run/vm-control.sock"

                # Clean up old socket
                rm -f $SOCKET_PATH

                echo "Service VM control handler started (VM ID: ''${VM_ID:-unknown})"
                echo "Mock: Would listen on $SOCKET_PATH"

                # Keep service running
                while true; do
                  sleep 10
                  echo "Service VM still running..."
                done
              '';
              serviceConfig = {
                Type = "simple";
                Restart = "always";
                RestartSec = 5;
              };
            };

            # Main worker service for processing jobs
            systemd.services.mvm-ci-worker = {
              description = "MVM-CI Worker Service";
              after = [ "vm-control-handler.service" ];
              wantedBy = [ "multi-user.target" ];
              environment = {
                VM_TYPE = "service";
                JOB_DIR = "/mnt/jobs";
              };
              path = [ mvm-ci-worker pkgs.coreutils pkgs.jq ];
              script = ''
                echo "MVM-CI Worker Service starting..."
                echo "VM Type: $VM_TYPE"
                echo "Job Directory: $JOB_DIR"

                # Mock: Process jobs from queue
                while true; do
                  # In production, would fetch jobs from control plane
                  echo "Waiting for jobs..."
                  sleep 30
                done
              '';
              serviceConfig = {
                Type = "simple";
                Restart = "always";
                RestartSec = 10;
              };
            };

            # Basic system packages
            environment.systemPackages = with pkgs; [
              coreutils
              jq
              curl
              mvm-ci-worker
            ];

            # Optimize for fast boot
            boot.initrd.compressor = "cat";
            boot.initrd.compressorArgs = [];

            # Reduce closure size
            documentation.enable = false;
            documentation.man.enable = false;
            documentation.info.enable = false;
            documentation.doc.enable = false;

            # Enable basic networking
            networking.useDHCP = false;
            networking.interfaces.eth0.useDHCP = true;

            system.stateVersion = "24.11";
          }
        ];
      };

      # Worker VM configuration with QEMU
      nixosConfigurations.worker-vm = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          microvm.nixosModules.microvm
          {
            microvm = {
              hypervisor = "qemu";
              mem = 512;
              vcpu = 1;
              balloon = true;

              shares = [
                {
                  source = "/nix/store";
                  mountPoint = "/nix/.ro-store";
                  tag = "store";
                  proto = "9p";
                }
                {
                  source = "/var/lib/mvm-ci/jobs";
                  mountPoint = "/mnt/jobs";
                  tag = "jobs";
                  proto = "9p";
                }
              ];

              writableStoreOverlay = "/nix/.rw-store";

              # User networking
              interfaces = [{
                type = "user";
                id = "vm-net0";
                mac = "02:00:00:00:00:01";
              }];
            };

            boot.loader.grub.enable = false;

            boot.kernelParams = [
              "console=ttyS0"
              "panic=1"
            ];

            services.journald.console = "/dev/console";

            # Auto-run worker and shutdown after completion
            systemd.services.mvm-ci-worker-oneshot = {
              description = "MVM-CI Worker (One-shot)";
              after = [ "multi-user.target" ];
              wantedBy = [ "multi-user.target" ];
              environment = {
                VM_TYPE = "ephemeral";
                JOB_DIR = "/mnt/jobs";
              };
              path = [ mvm-ci-worker pkgs.coreutils pkgs.jq ];
              script = ''
                echo "Starting ephemeral worker..."

                # Process single job from /mnt/jobs/job.json if it exists
                if [ -f /mnt/jobs/job.json ]; then
                  export JOB_PAYLOAD=$(cat /mnt/jobs/job.json)
                  worker
                else
                  echo "No job found at /mnt/jobs/job.json"
                fi

                # Shutdown after completion
                echo "Job complete, shutting down..."
                /run/current-system/sw/bin/poweroff
              '';
              serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = false;
              };
            };

            environment.systemPackages = with pkgs; [
              coreutils
              jq
              mvm-ci-worker
            ];

            boot.initrd.compressor = "cat";
            boot.initrd.compressorArgs = [];

            documentation.enable = false;
            documentation.man.enable = false;
            documentation.info.enable = false;
            documentation.doc.enable = false;

            networking.useDHCP = false;
            networking.interfaces.eth0.useDHCP = true;

            system.stateVersion = "24.11";
          }
        ];
      };

      # Build commands
      packages.${system} = {
        service-vm = microvm.lib.buildRunner {
          inherit system;
          nixosConfiguration = self.nixosConfigurations.service-vm;
        };

        worker-vm = microvm.lib.buildRunner {
          inherit system;
          nixosConfiguration = self.nixosConfigurations.worker-vm;
        };
      };

      defaultPackage.${system} = self.packages.${system}.service-vm;
    };
}