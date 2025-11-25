{
  description = "MVM-CI Worker MicroVMs using microvm.nix";

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
      # Replace with actual worker build once workspace issues are resolved
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
      # Service VM configuration - long-running VM that processes multiple jobs
      nixosConfigurations.service-vm = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          microvm.nixosModules.microvm
          {
            microvm = {
              hypervisor = "cloud-hypervisor";
              mem = 1024;  # 1GB for service VMs
              vcpu = 2;

              # Enable balloon device for memory efficiency
              balloon = true;

              shares = [
                {
                  source = "/nix/store";
                  mountPoint = "/nix/.ro-store";
                  tag = "store";
                  proto = "virtiofs";  # Use virtiofs instead of 9p for better performance
                }
                {
                  source = "/home/brittonr/mvm-ci-test/jobs";
                  mountPoint = "/mnt/jobs";
                  tag = "jobs";
                  proto = "virtiofs";
                }
              ];

              writableStoreOverlay = "/nix/.rw-store";

              # Virtio-net configuration
              interfaces = [{
                type = "tap";
                id = "vm-net0";
                mac = "02:00:00:00:00:02";
              }];
            };

            boot.loader.grub.enable = false;

            # Kernel optimizations for Cloud Hypervisor and virtio
            boot.kernelParams = [
              "console=hvc0"
              "reboot=k"
              "panic=1"
              "pci=off"  # Cloud Hypervisor doesn't use PCI
              "nomodules"
              "virtio_mmio.device=4K@0xd0000000:5"
              "virtio_mmio.device=4K@0xd0001000:6"
              "virtio_mmio.device=4K@0xd0002000:7"
            ];

            # Enable virtio modules
            boot.initrd.kernelModules = [
              "virtio_balloon"
              "virtio_blk"
              "virtio_net"
              "virtio_pci"
              "virtio_mmio"
              "virtiofs"
            ];

            networking = {
              hostName = "service-vm";
              useDHCP = true;
              firewall.enable = false;
            };

            environment.systemPackages = with pkgs; [
              jq
              socat
              coreutils
            ];

            users.users.worker = {
              isNormalUser = true;
              group = "worker";
              home = "/home/worker";
              createHome = true;
              password = "worker";  # For debugging
            };
            users.groups.worker = {};

            # Set root password for debugging (remove in production)
            users.users.root.password = "root";

            # Service VM control socket service
            systemd.services.control-socket = {
              description = "Service VM Control Socket";
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];

              script = ''
                #!/usr/bin/env bash
                SOCKET="/run/control.sock"
                STATE="/var/lib/vm-state.json"

                mkdir -p /var/lib/jobs/{pending,processing,completed}
                echo '{"status":"ready","jobs_completed":0}' > "$STATE"

                handle_request() {
                  read -r line
                  cmd=$(echo "$line" | ${pkgs.jq}/bin/jq -r .type 2>/dev/null)

                  case "$cmd" in
                    Ping)
                      jobs=$(${pkgs.jq}/bin/jq -r .jobs_completed "$STATE")
                      echo "{\"type\":\"Pong\",\"jobs_completed\":$jobs}"
                      ;;
                    ExecuteJob)
                      job=$(echo "$line" | ${pkgs.jq}/bin/jq .job)
                      job_id=$(echo "$job" | ${pkgs.jq}/bin/jq -r .id)
                      echo "$job" > "/var/lib/jobs/pending/$job_id.json"
                      echo '{"type":"Ack"}'
                      ;;
                    Shutdown)
                      echo '{"type":"Ack"}'
                      systemctl poweroff
                      ;;
                    *)
                      echo '{"type":"Error","message":"Unknown command"}'
                      ;;
                  esac
                }

                export -f handle_request
                ${pkgs.socat}/bin/socat UNIX-LISTEN:"$SOCKET",fork EXEC:"/bin/bash -c handle_request"
              '';

              serviceConfig = {
                Type = "simple";
                User = "worker";
                Restart = "always";
              };
            };

            # Job processor service
            systemd.services.job-processor = {
              description = "Service VM Job Processor";
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];

              script = ''
                #!/usr/bin/env bash
                STATE="/var/lib/vm-state.json"

                while true; do
                  for job in /var/lib/jobs/pending/*.json; do
                    [ -f "$job" ] || continue

                    job_id=$(basename "$job" .json)
                    mv "$job" "/var/lib/jobs/processing/"

                    # Process job (placeholder)
                    echo "Processing job $job_id"
                    sleep 2

                    mv "/var/lib/jobs/processing/$job_id.json" "/var/lib/jobs/completed/"

                    # Update job count
                    count=$(${pkgs.jq}/bin/jq -r .jobs_completed "$STATE")
                    new_count=$((count + 1))
                    ${pkgs.jq}/bin/jq ".jobs_completed = $new_count" "$STATE" > "$STATE.tmp"
                    mv "$STATE.tmp" "$STATE"
                  done
                  sleep 1
                done
              '';

              serviceConfig = {
                Type = "simple";
                User = "worker";
                Restart = "always";
              };
            };

            system.stateVersion = "24.05";
          }
        ];
      };

      # Define the worker VM configuration (ephemeral - one job then terminate)
      nixosConfigurations.worker-vm = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          microvm.nixosModules.microvm
          {
            # MicroVM configuration
            microvm = {
              # Use Cloud Hypervisor for better performance
              hypervisor = "cloud-hypervisor";

              # Default resources (can be overridden at runtime)
              mem = 512;  # Memory in MB
              vcpu = 1;   # Number of vCPUs

              # Enable balloon device for memory efficiency
              balloon = true;

              # Share directories using virtiofs
              shares = [
                {
                  source = "/nix/store";
                  mountPoint = "/nix/.ro-store";
                  tag = "store";
                  proto = "virtiofs";  # Better performance than 9p
                }
                # Job data directory for passing jobs to VMs
                {
                  # Use home directory for testing (change to /var/lib/mvm-ci/jobs for production)
                  source = "/home/brittonr/mvm-ci-test/jobs";
                  mountPoint = "/mnt/jobs";
                  tag = "jobs";
                  proto = "virtiofs";
                }
              ];

              # Writable store overlay for runtime package installations
              writableStoreOverlay = "/nix/.rw-store";

              # Virtio-net configuration
              interfaces = [{
                type = "tap";
                id = "vm-net1";
                mac = "02:00:00:00:00:01"; # Will be overridden per-VM
              }];

              # Control socket for VM management
              # socket = "/var/lib/mvm-ci/vms/control.sock";
            };

            # Boot configuration
            boot.loader.grub.enable = false;
            boot.isContainer = false;

            # Kernel optimizations for Cloud Hypervisor and virtio
            boot.kernelParams = [
              "console=hvc0"
              "reboot=k"
              "panic=1"
              "pci=off"
              "nomodules"
              "virtio_mmio.device=4K@0xd0000000:5"
              "virtio_mmio.device=4K@0xd0001000:6"
              "virtio_mmio.device=4K@0xd0002000:7"
            ];

            # Enable virtio modules
            boot.initrd.kernelModules = [
              "virtio_balloon"
              "virtio_blk"
              "virtio_net"
              "virtio_pci"
              "virtio_mmio"
              "virtiofs"
            ];

            boot.kernel.sysctl = {
              "net.ipv4.ip_forward" = 1;
              "net.ipv6.conf.all.forwarding" = 1;
            };

            # Network configuration
            networking = {
              hostName = "worker-vm";
              useDHCP = true;
              firewall.enable = false;
            };

            # Essential packages
            environment.systemPackages = with pkgs; [
              curl
              jq
              inotify-tools
              coreutils
              mvm-ci-worker
            ];

            # Worker user
            users.users.worker = {
              isNormalUser = true;
              group = "worker";
              home = "/home/worker";
              createHome = true;
            };
            users.groups.worker = {};

            # Job executor service
            systemd.services.mvm-ci-executor = {
              description = "MVM-CI Job Executor";
              after = [ "network.target" "multi-user.target" ];

              path = with pkgs; [ inotify-tools coreutils jq curl mvm-ci-worker ];

              script = ''
                set -e

                # Get VM ID (use hostname or machine-id)
                VM_ID=$(hostname)
                JOB_DIR="/mnt/jobs/$VM_ID"

                echo "MVM-CI Executor: Starting for VM $VM_ID"
                echo "Job directory: $JOB_DIR"

                # Create job directory if it doesn't exist
                mkdir -p "$JOB_DIR"

                # Wait for job file
                echo "Waiting for job at $JOB_DIR/job.json..."

                while [ ! -f "$JOB_DIR/job.json" ]; do
                  if [ -d "$JOB_DIR" ]; then
                    inotifywait -t 5 -e create -e moved_to "$JOB_DIR" 2>/dev/null || true
                  fi
                  sleep 1
                done

                echo "Job file detected, starting execution"

                # Read job configuration
                export JOB_PAYLOAD=$(cat "$JOB_DIR/job.json")
                export JOB_ID=$(jq -r .id "$JOB_DIR/job.json" || echo "unknown")

                # Read control plane ticket if available
                if [ -f "$JOB_DIR/ticket" ]; then
                  export CONTROL_PLANE_TICKET=$(cat "$JOB_DIR/ticket")
                fi

                echo "Executing job $JOB_ID"

                # Execute worker and capture output
                ${mvm-ci-worker}/bin/worker 2>&1 | tee "$JOB_DIR/output.log"
                EXIT_CODE=$?

                # Write result
                echo "$EXIT_CODE" > "$JOB_DIR/exit_code"
                echo "Job completed with exit code: $EXIT_CODE"

                # Signal completion
                touch "$JOB_DIR/completed"

                # Give host time to read results
                sleep 2

                # Shutdown VM after job
                echo "Shutting down VM..."
                systemctl poweroff
              '';

              serviceConfig = {
                Type = "simple";
                User = "worker";
                Group = "worker";
                StandardOutput = "journal+console";
                StandardError = "journal+console";
                Restart = "no";
              };

              wantedBy = [ "multi-user.target" ];
            };

            # Auto-shutdown service if no job arrives
            systemd.services.auto-shutdown = {
              description = "Auto shutdown if no job";
              after = [ "multi-user.target" ];

              script = ''
                # Wait for job or timeout
                sleep 60

                VM_ID=$(hostname)
                if [ ! -f "/mnt/jobs/$VM_ID/job.json" ]; then
                  echo "No job found after 60 seconds, shutting down"
                  systemctl poweroff
                fi
              '';

              serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = false;
              };

              wantedBy = [ "multi-user.target" ];
            };

            # SSH for debugging (disable in production)
            services.openssh = {
              enable = true;
              settings = {
                PermitRootLogin = "yes";
                PasswordAuthentication = true;
              };
            };

            # Set root password for debugging (remove in production)
            users.users.root.password = "root";

            system.stateVersion = "24.05";
          }
        ];
      };

      # Export packages for the host system
      packages.${system} = rec {
        # The ephemeral worker VM runner
        worker-vm = self.nixosConfigurations.worker-vm.config.microvm.declaredRunner;

        # The service VM runner (actual microVM)
        service-vm = self.nixosConfigurations.service-vm.config.microvm.declaredRunner;

        # Script to run actual service VM
        run-service-vm = pkgs.writeShellScriptBin "run-service-vm" ''
          #!/usr/bin/env bash
          set -euo pipefail

          VM_ID="''${1:-service-vm-$(date +%s)}"
          echo "Starting Service VM: $VM_ID"

          # Create required directories for the VM
          mkdir -p /var/lib/mvm-ci/jobs/{pending,processing,completed}

          # Start the actual microVM
          echo "Starting actual microVM with QEMU..."
          echo "This will run a real NixOS VM with systemd services inside."
          echo ""
          echo "The VM will have:"
          echo "  - systemd service 'control-socket' listening on /run/control.sock"
          echo "  - systemd service 'job-processor' processing jobs from queue"
          echo ""

          # Run the service VM
          ${service-vm}/bin/microvm-run

          echo "Service VM terminated"
        '';

        # Helper script to run worker VMs with job data
        run-worker-vm = pkgs.writeShellScriptBin "run-worker-vm" ''
          set -euo pipefail

          # Parse arguments
          JOB_FILE="''${1:-}"
          VM_ID="''${2:-$(${pkgs.util-linux}/bin/uuidgen)}"
          TICKET="''${3:-http://localhost:3020}"
          MEM="''${4:-512}"
          VCPU="''${5:-1}"

          if [ -z "$JOB_FILE" ] || [ ! -f "$JOB_FILE" ]; then
            echo "Usage: $0 <job-file> [vm-id] [ticket] [memory-mb] [vcpus]"
            echo ""
            echo "  job-file: Path to JSON file containing job data"
            echo "  vm-id: Optional VM identifier (default: random UUID)"
            echo "  ticket: Optional control plane URL (default: http://localhost:3020)"
            echo "  memory-mb: Optional memory in MB (default: 512)"
            echo "  vcpus: Optional number of vCPUs (default: 1)"
            exit 1
          fi

          echo "Starting worker VM:"
          echo "  VM ID: $VM_ID"
          echo "  Job file: $JOB_FILE"
          echo "  Memory: ''${MEM}MB"
          echo "  vCPUs: $VCPU"

          # Prepare job directory (using home directory for testing)
          JOB_DIR="$HOME/mvm-ci-test/jobs/$VM_ID"
          mkdir -p "$JOB_DIR"

          # Copy job data
          cp "$JOB_FILE" "$JOB_DIR/job.json"
          echo "$TICKET" > "$JOB_DIR/ticket"

          # Create VM state directory
          VM_STATE_DIR="$HOME/mvm-ci-test/vms/$VM_ID"
          mkdir -p "$VM_STATE_DIR"

          echo "Job directory prepared at: $JOB_DIR"

          # Run the VM
          echo "Starting microVM..."
          # Note: microvm-run may not support all these flags with QEMU user networking
          ${worker-vm}/bin/microvm-run || true

          # Check result
          if [ -f "$JOB_DIR/exit_code" ]; then
            EXIT_CODE=$(cat "$JOB_DIR/exit_code")
            echo "Job completed with exit code: $EXIT_CODE"

            # Display output if available
            if [ -f "$JOB_DIR/output.log" ]; then
              echo "=== Job Output ==="
              tail -n 50 "$JOB_DIR/output.log"
              echo "=================="
            fi

            # Cleanup job directory
            rm -rf "$JOB_DIR"
            rm -rf "$VM_STATE_DIR"

            exit $EXIT_CODE
          else
            echo "Job execution failed - no exit code found"
            rm -rf "$JOB_DIR"
            rm -rf "$VM_STATE_DIR"
            exit 1
          fi
        '';

        # Test script to verify VM functionality
        test-vm = pkgs.writeShellScriptBin "test-vm" ''
          set -euo pipefail

          echo "Creating test job..."

          # Create test job file
          TEST_JOB=$(mktemp --suffix=.json)
          cat > "$TEST_JOB" << 'EOF'
          {
            "id": "test-job-001",
            "payload": {
              "task": "echo 'Hello from MicroVM!'",
              "type": "simple"
            },
            "priority": 1,
            "status": "Pending"
          }
          EOF

          echo "Test job created at: $TEST_JOB"
          echo "Contents:"
          cat "$TEST_JOB"
          echo ""

          # Run the VM with test job
          ${run-worker-vm}/bin/run-worker-vm "$TEST_JOB" "test-vm-$(date +%s)"

          # Cleanup
          rm -f "$TEST_JOB"
        '';

        # Build the VM image (for inspection/debugging)
        # vm-image = self.nixosConfigurations.worker-vm.config.microvm.runner.microvmConfig;

        default = run-worker-vm;
      };

      # Development shell
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          qemu
          firecracker
          inotify-tools
          jq
          curl
        ];

        shellHook = ''
          echo "MicroVM.nix Worker Development Environment"
          echo ""
          echo "Available commands:"
          echo "  nix run .#test-vm        - Run a test VM with sample job"
          echo "  nix run .#run-worker-vm   - Run VM with custom job file"
          echo "  nix build .#worker-vm     - Build the VM runner"
          echo ""
          echo "Test directories are set up at: ~/mvm-ci-test/"
          echo ""
          echo "For production, update paths to /var/lib/mvm-ci and run:"
          echo "  sudo mkdir -p /var/lib/mvm-ci/{jobs,vms}"
          echo "  sudo chown -R $USER:$USER /var/lib/mvm-ci"
        '';
      };
    };
}