{
  description = "Service VM - Long-running microVM for job processing";

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
    in
    {
      nixosConfigurations.service-vm = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          microvm.nixosModules.microvm
          {
            # MicroVM configuration
            microvm = {
              hypervisor = "qemu"; # or "firecracker" for production
              mem = 1024;  # 1GB RAM for service VM
              vcpu = 2;    # 2 vCPUs

              # Shared directories
              shares = [
                {
                  source = "/nix/store";
                  mountPoint = "/nix/.ro-store";
                  tag = "store";
                  proto = "9p";
                  securityModel = "none";
                }
                {
                  # Job queue directory
                  source = "/var/lib/mvm-ci/jobs";
                  mountPoint = "/mnt/jobs";
                  tag = "jobs";
                  proto = "9p";
                  securityModel = "mapped-file";
                }
                {
                  # Control socket directory
                  source = "/var/lib/mvm-ci/sockets";
                  mountPoint = "/mnt/sockets";
                  tag = "sockets";
                  proto = "9p";
                  securityModel = "mapped-file";
                }
              ];

              writableStoreOverlay = "/nix/.rw-store";

              interfaces = [{
                type = "user";
                id = "net0";
                mac = "02:00:00:00:00:01";
              }];

              # Forward control socket from host
              forwardPorts = [
                {
                  from = "host";
                  host.port = 8080;
                  guest.port = 8080;
                }
              ];
            };

            # System configuration
            boot.loader.grub.enable = false;

            networking = {
              hostName = "service-vm";
              useDHCP = true;
              firewall.enable = false;
            };

            # Essential packages
            environment.systemPackages = with pkgs; [
              curl
              jq
              socat
              python3
            ];

            # Service user
            users.users.worker = {
              isNormalUser = true;
              group = "worker";
              home = "/home/worker";
              createHome = true;
            };
            users.groups.worker = {};

            # Control socket service - listens for commands
            systemd.services.vm-control-socket = {
              description = "VM Control Socket Service";
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];

              serviceConfig = {
                Type = "simple";
                User = "worker";
                Group = "worker";
                Restart = "always";
                RestartSec = "5s";
              };

              script = ''
                #!/usr/bin/env bash
                set -e

                SOCKET_PATH="/mnt/sockets/$(hostname).sock"
                STATE_FILE="/var/lib/vm-state.json"

                # Initialize state
                echo '{"status":"ready","jobs_completed":0,"start_time":"'$(date -Iseconds)'"}' > "$STATE_FILE"

                # Control handler function
                handle_command() {
                  local line="$1"
                  local cmd=$(echo "$line" | ${pkgs.jq}/bin/jq -r .type 2>/dev/null || echo "invalid")

                  case "$cmd" in
                    Ping)
                      local state=$(cat "$STATE_FILE")
                      local jobs=$(echo "$state" | ${pkgs.jq}/bin/jq -r .jobs_completed)
                      echo "{\"type\":\"Pong\",\"jobs_completed\":$jobs,\"status\":\"ready\"}"
                      ;;

                    ExecuteJob)
                      local job=$(echo "$line" | ${pkgs.jq}/bin/jq .job 2>/dev/null)
                      if [ -n "$job" ] && [ "$job" != "null" ]; then
                        # Queue job for processor
                        local job_id=$(echo "$job" | ${pkgs.jq}/bin/jq -r .id)
                        echo "$job" > "/var/lib/jobs/pending/$job_id.json"
                        echo '{"type":"Ack","job_id":"'$job_id'"}'
                      else
                        echo '{"type":"Error","message":"Invalid job"}'
                      fi
                      ;;

                    GetStatus)
                      cat "$STATE_FILE"
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

                # Create required directories
                mkdir -p /var/lib/jobs/{pending,processing,completed}
                mkdir -p $(dirname "$SOCKET_PATH")

                # Start socket listener
                echo "Starting control socket at $SOCKET_PATH"
                rm -f "$SOCKET_PATH"

                # Use socat to handle socket connections
                ${pkgs.socat}/bin/socat UNIX-LISTEN:"$SOCKET_PATH",fork SYSTEM:'
                  read line
                  handle_command "$line"
                ',pty,raw,echo=0
              '';
            };

            # Job processor service - processes jobs from queue
            systemd.services.vm-job-processor = {
              description = "VM Job Processor Service";
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];

              serviceConfig = {
                Type = "simple";
                User = "worker";
                Group = "worker";
                Restart = "always";
                RestartSec = "5s";
              };

              script = ''
                #!/usr/bin/env bash
                set -e

                STATE_FILE="/var/lib/vm-state.json"
                PENDING_DIR="/var/lib/jobs/pending"
                PROCESSING_DIR="/var/lib/jobs/processing"
                COMPLETED_DIR="/var/lib/jobs/completed"

                # Create directories
                mkdir -p "$PENDING_DIR" "$PROCESSING_DIR" "$COMPLETED_DIR"

                echo "Job processor started, monitoring $PENDING_DIR"

                while true; do
                  # Check for pending jobs
                  for job_file in "$PENDING_DIR"/*.json; do
                    [ -f "$job_file" ] || continue

                    job_id=$(basename "$job_file" .json)
                    echo "Processing job: $job_id"

                    # Move to processing
                    mv "$job_file" "$PROCESSING_DIR/"

                    # Execute job (placeholder - would run actual worker here)
                    job_content=$(cat "$PROCESSING_DIR/$job_id.json")
                    echo "Executing job: $job_content"

                    # Simulate job execution
                    sleep 3

                    # Move to completed
                    mv "$PROCESSING_DIR/$job_id.json" "$COMPLETED_DIR/"

                    # Update state
                    current_count=$(${pkgs.jq}/bin/jq -r .jobs_completed "$STATE_FILE")
                    new_count=$((current_count + 1))
                    ${pkgs.jq}/bin/jq ".jobs_completed = $new_count" "$STATE_FILE" > "$STATE_FILE.tmp"
                    mv "$STATE_FILE.tmp" "$STATE_FILE"

                    echo "Job $job_id completed (total: $new_count)"
                  done

                  sleep 1
                done
              '';
            };

            # Health check service
            systemd.services.vm-health-check = {
              description = "VM Health Check Service";
              after = [ "multi-user.target" ];
              wantedBy = [ "multi-user.target" ];

              serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = true;
              };

              script = ''
                echo "Service VM started successfully at $(date)"
                echo "Hostname: $(hostname)"
                echo "IP: $(ip -4 addr show | grep inet | grep -v 127.0.0.1 | awk '{print $2}')"
                echo "Services running:"
                systemctl status vm-control-socket --no-pager || true
                systemctl status vm-job-processor --no-pager || true
              '';
            };

            system.stateVersion = "24.05";
          }
        ];
      };

      packages.${system} = {
        # Build the service VM
        service-vm = self.nixosConfigurations.service-vm.config.microvm.declaredRunner;

        # Runner script to start service VM
        run-service-vm = pkgs.writeShellScriptBin "run-service-vm" ''
          #!/usr/bin/env bash
          set -euo pipefail

          VM_ID="''${1:-service-vm-$(date +%s)}"
          HOST_PORT="''${2:-8080}"

          echo "Starting Service VM"
          echo "  VM ID: $VM_ID"
          echo "  Control port: $HOST_PORT"

          # Create required directories
          sudo mkdir -p /var/lib/mvm-ci/{jobs,sockets}
          sudo chown -R $USER:$USER /var/lib/mvm-ci

          # Run the VM
          echo "Starting microVM..."
          ${self.packages.${system}.service-vm}/bin/microvm-run

          echo "Service VM is running!"
          echo "Control socket available at: /var/lib/mvm-ci/sockets/$VM_ID.sock"
        '';
      };

      # Default package
      defaultPackage.${system} = self.packages.${system}.run-service-vm;
    };
}