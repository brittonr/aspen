# NixOS VM integration test: Multi-host service mesh with Cloud Hypervisor microVMs
#
# Proves cross-host service mesh routing over real network links:
#
#   1. Two QEMU hosts (hostA, hostB) on a virtual bridge
#   2. Three aspen-node processes form a Raft cluster across both hosts (2+1 split)
#   3. Each host runs its own aspen-net daemon (SOCKS5 + TunnelAcceptor)
#   4. Guest A (CH microVM on Host A) runs an HTTP server
#   5. Host A publishes Guest A's service to the mesh
#   6. From Host B, curl through SOCKS5 → iroh QUIC tunnel across network → Host A → Guest A
#
# Data flow (cross-host):
#   curl (Host B) → SOCKS5 (Host B) → iroh QUIC → Host A TunnelAcceptor →
#   socat → TAP → Guest A HTTP server
#
# Requires nested KVM.
#
# Build & run:
#   nix build .#checks.x86_64-linux.multihost-microvm-mesh-test --impure
{
  pkgs,
  microvm,
  aspenNodePackage,
  aspenCliPackage,
  aspenNetPackage,
}: let
  cookie = "multihost-mesh-test";

  # ── Host networking (QEMU virtual bridge) ─────────────────────────
  # NixOS test driver assigns IPs automatically, but we reference them
  # in the test script via hostA.name / hostB.name.

  # ── Guest A networking (on Host A) ───────────────────────────────
  svcGuestIp = "10.10.1.2";
  svcHostIp = "10.10.1.1";
  svcTap = "vm-svc";
  svcMac = "02:00:00:00:01:01";

  # ── Guest B networking (on Host B) ───────────────────────────────
  clientGuestIp = "10.10.2.2";
  clientHostIp = "10.10.2.1";
  clientTap = "vm-client";
  clientMac = "02:00:00:00:02:01";

  # ── Guest A: HTTP service VM ──────────────────────────────────────
  svcGuest = pkgs.nixos [
    microvm.nixosModules.microvm
    ({...}: {
      microvm = {
        hypervisor = "cloud-hypervisor";
        mem = 512;
        vcpu = 1;
        cloud-hypervisor.extraArgs = [
          "--serial"
          "file=/tmp/guest-svc-serial.log"
          "--console"
          "off"
        ];
        kernelParams = [
          "console=ttyS0"
          "panic=1"
        ];
        volumes = [];
        shares = [];
        interfaces = [
          {
            type = "tap";
            id = svcTap;
            mac = svcMac;
          }
        ];
      };

      system.stateVersion = "24.11";
      documentation.enable = false;
      programs.command-not-found.enable = false;
      boot.loader.grub.enable = false;

      environment.systemPackages = [pkgs.python3];

      systemd.services.http-server = {
        description = "Test HTTP server";
        after = ["network-online.target"];
        wants = ["network-online.target"];
        wantedBy = ["multi-user.target"];
        serviceConfig = {
          Type = "simple";
          ExecStartPre = "${pkgs.coreutils}/bin/mkdir -p /tmp/www";
          ExecStart = "${pkgs.python3}/bin/python3 -m http.server 8080 --directory /tmp/www";
          Restart = "on-failure";
        };
      };

      system.activationScripts.seedContent = ''
        mkdir -p /tmp/www
        echo 'hello from multihost mesh' > /tmp/www/index.html
        echo '{"source":"multihost-mesh","ok":true}' > /tmp/www/status.json
      '';

      networking = {
        hostName = "svc-guest";
        firewall.enable = false;
        useDHCP = false;
        usePredictableInterfaceNames = false;
      };

      systemd.network = {
        enable = true;
        networks."10-guest" = {
          matchConfig.Type = "ether";
          address = ["${svcGuestIp}/24"];
          routes = [
            {Gateway = svcHostIp;}
          ];
          networkConfig.DHCP = "no";
        };
      };
    })
  ];

  # ── Guest B: Client VM ────────────────────────────────────────────
  clientGuest = pkgs.nixos [
    microvm.nixosModules.microvm
    ({...}: {
      microvm = {
        hypervisor = "cloud-hypervisor";
        mem = 512;
        vcpu = 1;
        cloud-hypervisor.extraArgs = [
          "--serial"
          "file=/tmp/guest-client-serial.log"
          "--console"
          "off"
        ];
        kernelParams = [
          "console=ttyS0"
          "panic=1"
        ];
        volumes = [];
        shares = [];
        interfaces = [
          {
            type = "tap";
            id = clientTap;
            mac = clientMac;
          }
        ];
      };

      system.stateVersion = "24.11";
      documentation.enable = false;
      programs.command-not-found.enable = false;
      boot.loader.grub.enable = false;

      environment.systemPackages = [pkgs.curl];

      networking = {
        hostName = "client-guest";
        firewall.enable = false;
        useDHCP = false;
        usePredictableInterfaceNames = false;
      };

      systemd.network = {
        enable = true;
        networks."10-guest" = {
          matchConfig.Type = "ether";
          address = ["${clientGuestIp}/24"];
          routes = [
            {Gateway = clientHostIp;}
          ];
          networkConfig.DHCP = "no";
        };
      };
    })
  ];

  svcRunner = svcGuest.config.microvm.runner.cloud-hypervisor;
  clientRunner = clientGuest.config.microvm.runner.cloud-hypervisor;

  # Common host configuration
  mkHostConfig = {extraPackages ? []}: {
    config,
    pkgs,
    lib,
    ...
  }: {
    virtualisation.qemu.options = [
      "-enable-kvm"
      "-cpu"
      "host"
    ];
    virtualisation.memorySize = 6144;
    virtualisation.cores = 4;

    environment.systemPackages =
      [
        aspenNodePackage
        aspenCliPackage
        aspenNetPackage
        pkgs.cloud-hypervisor
        pkgs.curl
        pkgs.iproute2
        pkgs.socat
      ]
      ++ extraPackages;

    networking.firewall.enable = false;
    boot.kernel.sysctl."net.ipv4.ip_forward" = 1;
  };
in
  pkgs.testers.nixosTest {
    name = "multihost-microvm-mesh";

    nodes.hostA = mkHostConfig {
      extraPackages = [svcRunner];
    };

    nodes.hostB = mkHostConfig {
      extraPackages = [clientRunner];
    };

    testScript = ''
      import json
      import re
      import time

      CLI = "${aspenCliPackage}/bin/aspen-cli"

      hostA.start()
      hostB.start()
      hostA.wait_for_unit("multi-user.target")
      hostB.wait_for_unit("multi-user.target")

      # Verify cross-host connectivity
      hostA.wait_until_succeeds("ping -c1 -W2 hostB", timeout=30)
      hostB.wait_until_succeeds("ping -c1 -W2 hostA", timeout=30)
      hostA.log("Cross-host connectivity verified")

      def get_ticket():
          return hostA.succeed("cat /tmp/aspen-1/cluster-ticket.txt").strip()

      def cli_on(host, cmd, check=True):
          """Run aspen-cli --json on a specific host."""
          ticket = get_ticket()
          run = (
              f"{CLI} --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli_out.json 2>/dev/null"
          )
          if check:
              host.succeed(run)
          else:
              host.execute(run)
          raw = host.succeed("cat /tmp/_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              host.log(f"cli() JSON parse failed, raw={raw!r}")
              return raw.strip()

      # ════════════════════════════════════════════════════════════
      # Phase 1: Bootstrap 3-node Raft cluster across both hosts
      # ════════════════════════════════════════════════════════════

      hostA.log("=== Phase 1: Starting cross-host Raft cluster ===")
      hostA.succeed("mkdir -p /tmp/aspen-{1,2}")
      hostB.succeed("mkdir -p /tmp/aspen-3")

      # Node 1 on Host A (auto-bootstraps)
      hostA.succeed(
          "systemd-run --unit=aspen-node-1 "
          "bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          "exec aspen-node "
          "--node-id 1 "
          "--cookie ${cookie} "
          "--data-dir /tmp/aspen-1 "
          "--storage-backend inmemory "
          "--relay-mode disabled "
          "--disable-gossip "
          "--disable-mdns "
          "--bind-port 7001'"
      )

      hostA.wait_until_succeeds(
          "journalctl -u aspen-node-1 --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
          timeout=120,
      )
      hostA.wait_until_succeeds("test -f /tmp/aspen-1/cluster-ticket.txt", timeout=10)
      ticket = get_ticket()
      hostA.log(f"Cluster ticket: {ticket[:50]}...")

      # Node 2 on Host A
      hostA.succeed(
          f"systemd-run --unit=aspen-node-2 "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-node "
          f"--node-id 2 "
          f"--cookie ${cookie} "
          f"--data-dir /tmp/aspen-2 "
          f"--storage-backend inmemory "
          f"--relay-mode disabled "
          f"--disable-gossip "
          f"--disable-mdns "
          f"--bind-port 7002 "
          f"--ticket {ticket}'"
      )

      hostA.wait_until_succeeds(
          "journalctl -u aspen-node-2 --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
          timeout=60,
      )

      # Node 3 on Host B (cross-host join)
      hostB.succeed(
          f"systemd-run --unit=aspen-node-3 "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-node "
          f"--node-id 3 "
          f"--cookie ${cookie} "
          f"--data-dir /tmp/aspen-3 "
          f"--storage-backend inmemory "
          f"--relay-mode disabled "
          f"--disable-gossip "
          f"--disable-mdns "
          f"--bind-port 7001 "
          f"--ticket {ticket}'"
      )

      hostB.wait_until_succeeds(
          "journalctl -u aspen-node-3 --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
          timeout=60,
      )

      # Verify all nodes active
      for i in [1, 2]:
          status = hostA.succeed(f"systemctl is-active aspen-node-{i}.service || echo dead").strip()
          assert status == "active", f"Node {i} on Host A not active: {status}"
      status = hostB.succeed("systemctl is-active aspen-node-3.service || echo dead").strip()
      assert status == "active", f"Node 3 on Host B not active: {status}"

      # Initialize cluster
      hostA.succeed(
          f"{CLI} --ticket '{ticket}' cluster init >/dev/null 2>&1 || true"
      )
      time.sleep(2)

      hostA.log("=== Phase 1 PASSED: Cross-host Raft cluster running (2 on Host A, 1 on Host B) ===")

      # ════════════════════════════════════════════════════════════
      # Phase 2: Start aspen-net daemons on both hosts
      # ════════════════════════════════════════════════════════════

      hostA.log("=== Phase 2: Starting aspen-net daemons on both hosts ===")

      # Net daemon on Host A (TunnelAcceptor receives incoming tunnels)
      hostA.succeed(
          f"systemd-run --unit=aspen-net-daemon "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-net up "
          f"--ticket {ticket} "
          f"--socks5-addr 0.0.0.0:1080 --no-dns'"
      )

      # Net daemon on Host B (SOCKS5 proxy for Guest B)
      hostB.succeed(
          f"systemd-run --unit=aspen-net-daemon "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-net up "
          f"--ticket {ticket} "
          f"--socks5-addr 0.0.0.0:1080 --no-dns'"
      )

      hostA.wait_until_succeeds("ss -tlnp | grep ':1080'", timeout=30)
      hostB.wait_until_succeeds("ss -tlnp | grep ':1080'", timeout=30)
      hostA.log("Both SOCKS5 proxies listening")

      hostA.wait_until_succeeds(
          "journalctl -u aspen-net-daemon --no-pager 2>/dev/null | grep -q 'connected to cluster'",
          timeout=30,
      )
      hostB.wait_until_succeeds(
          "journalctl -u aspen-net-daemon --no-pager 2>/dev/null | grep -q 'connected to cluster'",
          timeout=30,
      )

      hostA.log("=== Phase 2 PASSED: aspen-net daemons running on both hosts ===")

      # ════════════════════════════════════════════════════════════
      # Phase 3: Launch Guest A on Host A
      # ════════════════════════════════════════════════════════════

      hostA.log("=== Phase 3: Launching Guest A on Host A ===")

      hostA.succeed("ip tuntap add ${svcTap} mode tap")
      hostA.succeed("ip addr add ${svcHostIp}/24 dev ${svcTap}")
      hostA.succeed("ip link set ${svcTap} up")

      hostA.succeed(
          "systemd-run --unit=microvm-svc "
          "--property=WorkingDirectory=/tmp "
          "'--property=Environment=PATH=/run/current-system/sw/bin' "
          "${svcRunner}/bin/microvm-run"
      )

      hostA.wait_until_succeeds(
          "curl -sf --connect-timeout 2 http://${svcGuestIp}:8080/index.html",
          timeout=180,
      )
      output = hostA.succeed("curl -sf http://${svcGuestIp}:8080/index.html")
      assert "hello from multihost mesh" in output, f"wrong content: {output!r}"
      hostA.log(f"Guest A HTTP server responding: {output.strip()}")

      hostA.log("=== Phase 3 PASSED: Guest A running on Host A ===")

      # ════════════════════════════════════════════════════════════
      # Phase 4: Publish service from Host A
      # ════════════════════════════════════════════════════════════

      hostA.log("=== Phase 4: Publishing service from Host A ===")

      # socat bridges localhost:9080 → Guest A:8080
      hostA.succeed(
          "systemd-run --unit=socat-bridge "
          "socat TCP-LISTEN:9080,fork,reuseaddr TCP:${svcGuestIp}:8080"
      )
      time.sleep(1)
      hostA.succeed("curl -sf http://127.0.0.1:9080/index.html")
      hostA.log("socat bridge verified on Host A")

      # Get endpoint ID from Host A's cluster status
      status_out = cli_on(hostA, "cluster status")
      hostA.log(f"Cluster status: {status_out}")
      raw_eid = ""
      if isinstance(status_out, dict):
          raw_eid = status_out.get("endpoint_id", "")
          if not raw_eid:
              nodes_list = status_out.get("nodes", [])
              if nodes_list:
                  raw_eid = nodes_list[0].get("endpoint_id", "")
      pk_match = re.search(r'PublicKey\(([0-9a-f]+)\)', str(raw_eid))
      if pk_match:
          endpoint_id = pk_match.group(1)
      else:
          endpoint_id = str(raw_eid).strip()
      assert endpoint_id, f"could not extract endpoint_id from: {status_out}"
      hostA.log(f"Endpoint ID: {endpoint_id}")

      # Publish service
      ticket = get_ticket()
      hostA.succeed(
          f"{CLI} --ticket '{ticket}' --json net publish my-svc "
          f"--endpoint-id {endpoint_id} --port 9080 --proto tcp --tag web "
          f">/dev/null 2>&1 || true"
      )

      # Verify service visible from both hosts
      result_a = cli_on(hostA, "net services")
      svc_list_a = result_a.get("services", []) if isinstance(result_a, dict) else []
      names_a = [s.get("name", "") for s in svc_list_a]
      assert "my-svc" in names_a, f"my-svc not on Host A: {svc_list_a}"
      hostA.log("Service visible on Host A")

      result_b = cli_on(hostB, "net services")
      svc_list_b = result_b.get("services", []) if isinstance(result_b, dict) else []
      names_b = [s.get("name", "") for s in svc_list_b]
      assert "my-svc" in names_b, f"my-svc not on Host B: {svc_list_b}"
      hostB.log("Service visible on Host B (cross-host registry)")

      hostA.log("=== Phase 4 PASSED: Service published and visible on both hosts ===")

      # ════════════════════════════════════════════════════════════
      # Phase 5: Launch Guest B on Host B
      # ════════════════════════════════════════════════════════════

      hostB.log("=== Phase 5: Launching Guest B on Host B ===")

      hostB.succeed("ip tuntap add ${clientTap} mode tap")
      hostB.succeed("ip addr add ${clientHostIp}/24 dev ${clientTap}")
      hostB.succeed("ip link set ${clientTap} up")

      hostB.succeed(
          "systemd-run --unit=microvm-client "
          "--property=WorkingDirectory=/tmp "
          "'--property=Environment=PATH=/run/current-system/sw/bin' "
          "${clientRunner}/bin/microvm-run"
      )

      hostB.wait_until_succeeds(
          "ping -c1 -W2 ${clientGuestIp}",
          timeout=120,
      )
      hostB.log("Guest B is up and reachable on Host B")

      hostB.log("=== Phase 5 PASSED: Guest B running on Host B ===")

      # ════════════════════════════════════════════════════════════
      # Phase 6: Cross-host mesh routing
      # ════════════════════════════════════════════════════════════

      hostA.log("=== Phase 6: Cross-host mesh routing ===")

      # Step 1: Verify local SOCKS5 works on Host A (same-host baseline)
      hostA.wait_until_succeeds(
          "curl -sf --socks5-hostname 127.0.0.1:1080 "
          "http://my-svc.aspen:9080/index.html",
          timeout=30,
      )
      hostA.log("Host A local SOCKS5 routing works (baseline)")

      # Step 2: THE REAL TEST — Route from Host B's SOCKS5 through iroh QUIC to Host A
      # This is the cross-host path:
      #   Host B SOCKS5 → resolve my-svc (Raft KV) → iroh QUIC tunnel to Host A →
      #   Host A TunnelAcceptor → socat → 10.10.1.2:8080 → Guest A HTTP
      result = hostB.succeed(
          "curl -sf --socks5-hostname 127.0.0.1:1080 "
          "http://my-svc.aspen:9080/index.html"
      )
      assert "hello from multihost mesh" in result, f"cross-host routing failed: {result!r}"
      hostB.log(f"CROSS-HOST mesh routing verified: {result.strip()}")

      # Verify status.json cross-host
      result = hostB.succeed(
          "curl -sf --socks5-hostname 127.0.0.1:1080 "
          "http://my-svc.aspen:9080/status.json"
      )
      assert '"source":"multihost-mesh"' in result, f"wrong status.json: {result!r}"
      hostB.log(f"status.json via cross-host mesh: {result.strip()}")

      # Second request for stability
      result = hostB.succeed(
          "curl -sf --socks5-hostname 127.0.0.1:1080 "
          "http://my-svc.aspen:9080/index.html"
      )
      assert "hello from multihost mesh" in result, f"second cross-host request failed: {result!r}"
      hostB.log("Second cross-host request succeeded (tunnel stability OK)")

      hostA.log("=== Phase 6 PASSED: Cross-host service mesh routing works ===")

      # ════════════════════════════════════════════════════════════
      # Cleanup
      # ════════════════════════════════════════════════════════════

      hostB.succeed("systemctl stop microvm-client.service 2>/dev/null || true")
      hostA.succeed("systemctl stop microvm-svc.service 2>/dev/null || true")
      hostA.succeed("systemctl stop socat-bridge.service 2>/dev/null || true")
      hostB.succeed("systemctl stop aspen-net-daemon.service 2>/dev/null || true")
      hostA.succeed("systemctl stop aspen-net-daemon.service 2>/dev/null || true")
      hostB.succeed("systemctl stop aspen-node-3.service 2>/dev/null || true")
      hostA.succeed("systemctl stop aspen-node-2.service 2>/dev/null || true")
      hostA.succeed("systemctl stop aspen-node-1.service 2>/dev/null || true")
      time.sleep(1)

      hostA.log("=== ALL PHASES PASSED ===")
      hostA.log("  Phase 1: Cross-host Raft cluster (2 on Host A + 1 on Host B)")
      hostA.log("  Phase 2: aspen-net daemons on both hosts")
      hostA.log("  Phase 3: Guest A HTTP server on Host A (CH microVM)")
      hostA.log("  Phase 4: Service published and visible from both hosts")
      hostA.log("  Phase 5: Guest B client on Host B (CH microVM)")
      hostA.log("  Phase 6: Cross-host routing: Host B SOCKS5 -> iroh QUIC -> Host A -> Guest A")
    '';
  }
