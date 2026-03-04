# NixOS VM integration test: VirtioFS + Service Mesh in Cloud Hypervisor microVMs
#
# Proves that a single microVM can simultaneously use:
#   - Raft-backed storage (VirtioFS → AspenFs → KV store)
#   - Raft-backed networking (SOCKS5 → iroh QUIC tunnel → service mesh)
#
# Data flow (full path):
#   1. aspen-cli kv set → Raft KV (seed content)
#   2. AspenFs VirtioFS daemon serves KV data via vhost-user socket
#   3. Guest A mounts VirtioFS, nginx serves files from it
#   4. socat bridges localhost:9080 → Guest A:8080
#   5. aspen-net publishes Guest A as "web-svc" in the mesh
#   6. curl through SOCKS5 → iroh QUIC tunnel → socat → Guest A → nginx → VirtioFS → Raft KV
#
# Requires nested KVM.
#
# Build & run:
#   nix build .#checks.x86_64-linux.microvm-virtiofs-net-test --impure
{
  pkgs,
  microvm,
  aspenNodePackage,
  aspenCliPackage,
  aspenNetPackage,
  aspenClusterVirtiofsServer,
}: let
  cookie = "virtiofs-net-test";

  # ── Networking constants ──────────────────────────────────────────
  svcGuestIp = "10.10.1.2";
  svcHostIp = "10.10.1.1";
  svcTap = "vm-svc";
  svcMac = "02:00:00:00:01:01";

  clientGuestIp = "10.10.2.2";
  clientHostIp = "10.10.2.1";
  clientTap = "vm-client";
  clientMac = "02:00:00:00:02:01";

  virtiofsTag = "aspenfs";
  virtiofsSocket = "/tmp/aspenfs.sock";

  # ── Guest A: nginx + VirtioFS + TAP ──────────────────────────────
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
        shares = [
          {
            source = "/tmp/aspenfs"; # placeholder, required by microvm.nix
            mountPoint = "/var/www/aspen";
            tag = virtiofsTag;
            proto = "virtiofs";
            socket = virtiofsSocket;
          }
        ];
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

      services.nginx = {
        enable = true;
        virtualHosts.default = {
          default = true;
          root = "/var/www/aspen";
        };
      };

      networking = {
        hostName = "svc-virtiofs-guest";
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

  # ── Guest B: curl client ──────────────────────────────────────────
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
in
  pkgs.testers.nixosTest {
    name = "microvm-virtiofs-net";

    nodes.host = {
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
      virtualisation.memorySize = 8192;
      virtualisation.cores = 4;

      environment.systemPackages = [
        aspenNodePackage
        aspenCliPackage
        aspenNetPackage
        aspenClusterVirtiofsServer
        svcRunner
        clientRunner
        pkgs.cloud-hypervisor
        pkgs.curl
        pkgs.iproute2
        pkgs.socat
      ];

      networking.firewall.enable = false;
      boot.kernel.sysctl."net.ipv4.ip_forward" = 1;
    };

    testScript = ''
      import json
      import re
      import time

      CLI = "${aspenCliPackage}/bin/aspen-cli"

      host.start()
      host.wait_for_unit("multi-user.target")

      def get_ticket():
          return host.succeed("cat /tmp/aspen-1/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          """Run aspen-cli --json with the cluster ticket."""
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
      # Phase 1: Bootstrap 3-node Raft cluster
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 1: Starting 3-node Raft cluster ===")
      host.succeed("mkdir -p /tmp/aspen-{1,2,3}")

      host.succeed(
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

      host.wait_until_succeeds(
          "journalctl -u aspen-node-1 --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
          timeout=120,
      )
      host.wait_until_succeeds("test -f /tmp/aspen-1/cluster-ticket.txt", timeout=10)
      ticket = get_ticket()
      host.log(f"Cluster ticket: {ticket[:50]}...")

      for node_id in [2, 3]:
          host.succeed(
              f"systemd-run --unit=aspen-node-{node_id} "
              f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
              f"exec aspen-node "
              f"--node-id {node_id} "
              f"--cookie ${cookie} "
              f"--data-dir /tmp/aspen-{node_id} "
              f"--storage-backend inmemory "
              f"--relay-mode disabled "
              f"--disable-gossip "
              f"--disable-mdns "
              f"--bind-port 700{node_id} "
              f"--ticket {ticket}'"
          )

      for node_id in [2, 3]:
          host.wait_until_succeeds(
              f"journalctl -u aspen-node-{node_id} --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
              timeout=60,
          )

      for i in [1, 2, 3]:
          status = host.succeed(f"systemctl is-active aspen-node-{i}.service || echo dead").strip()
          assert status == "active", f"Node {i} not active: {status}"

      # Initialize the cluster
      host.succeed(
          f"{CLI} --ticket '{ticket}' cluster init >/dev/null 2>&1 || true"
      )
      time.sleep(2)
      host.log("=== Phase 1 PASSED: 3-node Raft cluster running ===")

      # ════════════════════════════════════════════════════════════
      # Phase 2: Seed KV data for VirtioFS
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 2: Seeding KV data ===")

      host.succeed(
          f"{CLI} --ticket '{ticket}' kv set /www/index.html 'hello from virtiofs-net mesh' "
          f">/dev/null 2>&1"
      )
      host.succeed(
          f"{CLI} --ticket '{ticket}' kv set /www/status.json "
          f"'{{\"source\":\"virtiofs-net\",\"ok\":true}}' "
          f">/dev/null 2>&1"
      )

      # Verify data is readable
      result = host.succeed(
          f"{CLI} --ticket '{ticket}' kv get /www/index.html 2>/dev/null"
      ).strip()
      assert "hello from virtiofs-net mesh" in result, f"KV read failed: {result!r}"
      host.log("=== Phase 2 PASSED: KV data seeded ===")

      # ════════════════════════════════════════════════════════════
      # Phase 3: Start VirtioFS daemon (Raft-backed)
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 3: Starting VirtioFS daemon ===")

      host.succeed(
          f"systemd-run --unit=aspenfs-virtiofs "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-cluster-virtiofs-server "
          f"--socket ${virtiofsSocket} "
          f"--ticket {ticket}'"
      )

      host.wait_until_succeeds("test -S ${virtiofsSocket}", timeout=30)
      host.log("VirtioFS socket ready")

      # Give the daemon time to connect to cluster
      host.wait_until_succeeds(
          "journalctl -u aspenfs-virtiofs --no-pager 2>/dev/null | grep -qi 'connected\\|ready\\|serving'",
          timeout=30,
      )
      host.log("=== Phase 3 PASSED: VirtioFS daemon connected to Raft cluster ===")

      # ════════════════════════════════════════════════════════════
      # Phase 4: Start aspen-net daemon (SOCKS5 + TunnelAcceptor)
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 4: Starting aspen-net daemon ===")

      host.succeed(
          f"systemd-run --unit=aspen-net-daemon "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-net up "
          f"--ticket {ticket} "
          f"--socks5-addr 0.0.0.0:1080 --no-dns'"
      )

      host.wait_until_succeeds("ss -tlnp | grep ':1080'", timeout=30)
      host.log("SOCKS5 proxy listening on 0.0.0.0:1080")

      host.wait_until_succeeds(
          "journalctl -u aspen-net-daemon --no-pager 2>/dev/null | grep -q 'connected to cluster'",
          timeout=30,
      )
      host.log("=== Phase 4 PASSED: aspen-net daemon running ===")

      # ════════════════════════════════════════════════════════════
      # Phase 5: Launch Guest A (VirtioFS + nginx + TAP)
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 5: Launching Guest A (VirtioFS + nginx) ===")

      # Create TAP for Guest A
      host.succeed("ip tuntap add ${svcTap} mode tap")
      host.succeed("ip addr add ${svcHostIp}/24 dev ${svcTap}")
      host.succeed("ip link set ${svcTap} up")

      # Launch Guest A with VirtioFS mount + TAP networking
      host.succeed(
          "systemd-run --unit=microvm-svc "
          "--property=WorkingDirectory=/tmp "
          "'--property=Environment=PATH=/run/current-system/sw/bin' "
          "${svcRunner}/bin/microvm-run"
      )
      host.log("Guest A microVM launched (VirtioFS + TAP)")

      # Wait for nginx to respond (guest boot + VirtioFS mount + nginx start)
      # Every nginx file read goes: VirtioFS → vhost-user → AspenFs → iroh → Raft KV
      host.wait_until_succeeds(
          "curl -sf --connect-timeout 2 http://${svcGuestIp}:80/index.html",
          timeout=180,
      )

      # Verify content comes from Raft KV via VirtioFS
      output = host.succeed("curl -sf http://${svcGuestIp}:80/index.html")
      assert "hello from virtiofs-net mesh" in output, f"wrong VirtioFS content: {output!r}"
      host.log(f"Guest A nginx serving from VirtioFS: {output.strip()}")

      host.log("=== Phase 5 PASSED: Guest A running with VirtioFS + nginx ===")

      # ════════════════════════════════════════════════════════════
      # Phase 6: Publish service via mesh
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 6: Publishing service to mesh ===")

      # socat bridges localhost:9080 → Guest A:80 (nginx default port)
      host.succeed(
          "systemd-run --unit=socat-bridge "
          "socat TCP-LISTEN:9080,fork,reuseaddr TCP:${svcGuestIp}:80"
      )
      time.sleep(1)

      # Verify socat bridge
      host.succeed("curl -sf http://127.0.0.1:9080/index.html")
      host.log("socat bridge verified: localhost:9080 -> Guest A:80")

      # Get endpoint ID
      status_out = cli("cluster status")
      host.log(f"Cluster status: {status_out}")
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
      host.log(f"Endpoint ID: {endpoint_id}")

      # Publish
      ticket = get_ticket()
      host.succeed(
          f"{CLI} --ticket '{ticket}' --json net publish web-svc "
          f"--endpoint-id {endpoint_id} --port 9080 --proto tcp --tag web "
          f">/dev/null 2>&1 || true"
      )

      # Verify service listed
      result = cli("net services")
      host.log(f"Services: {result}")
      svc_list = result.get("services", []) if isinstance(result, dict) else []
      names = [s.get("name", "") for s in svc_list]
      assert "web-svc" in names, f"web-svc not in services: {svc_list}"

      host.log("=== Phase 6 PASSED: Service published to mesh ===")

      # ════════════════════════════════════════════════════════════
      # Phase 7: Launch Guest B and route through mesh
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 7: Launching Guest B and routing through mesh ===")

      # Create TAP for Guest B
      host.succeed("ip tuntap add ${clientTap} mode tap")
      host.succeed("ip addr add ${clientHostIp}/24 dev ${clientTap}")
      host.succeed("ip link set ${clientTap} up")

      # Launch Guest B
      host.succeed(
          "systemd-run --unit=microvm-client "
          "--property=WorkingDirectory=/tmp "
          "'--property=Environment=PATH=/run/current-system/sw/bin' "
          "${clientRunner}/bin/microvm-run"
      )

      host.wait_until_succeeds(
          "ping -c1 -W2 ${clientGuestIp}",
          timeout=120,
      )
      host.log("Guest B is up and reachable")

      # Route through SOCKS5 mesh → iroh QUIC tunnel → socat → Guest A → nginx → VirtioFS → Raft KV
      result = host.succeed(
          "curl -sf --socks5-hostname 127.0.0.1:1080 "
          "http://web-svc.aspen:9080/index.html"
      )
      assert "hello from virtiofs-net mesh" in result, f"mesh routing failed: {result!r}"
      host.log(f"Mesh routing verified: {result.strip()}")

      # Verify status.json through the mesh
      result = host.succeed(
          "curl -sf --socks5-hostname 127.0.0.1:1080 "
          "http://web-svc.aspen:9080/status.json"
      )
      assert '"source":"virtiofs-net"' in result, f"wrong status.json: {result!r}"
      host.log(f"status.json via mesh: {result.strip()}")

      # Second request for stability
      result = host.succeed(
          "curl -sf --socks5-hostname 127.0.0.1:1080 "
          "http://web-svc.aspen:9080/index.html"
      )
      assert "hello from virtiofs-net mesh" in result, f"second request failed: {result!r}"
      host.log("Second request through mesh succeeded (connection stability OK)")

      host.log("=== Phase 7 PASSED: Full VirtioFS + mesh path verified ===")

      # ════════════════════════════════════════════════════════════
      # Cleanup
      # ════════════════════════════════════════════════════════════

      host.succeed("systemctl stop microvm-client.service 2>/dev/null || true")
      host.succeed("systemctl stop microvm-svc.service 2>/dev/null || true")
      host.succeed("systemctl stop socat-bridge.service 2>/dev/null || true")
      host.succeed("systemctl stop aspen-net-daemon.service 2>/dev/null || true")
      host.succeed("systemctl stop aspenfs-virtiofs.service 2>/dev/null || true")
      for i in [3, 2, 1]:
          host.succeed(f"systemctl stop aspen-node-{i}.service 2>/dev/null || true")
      time.sleep(1)

      host.log("=== ALL PHASES PASSED ===")
      host.log("  Phase 1: 3-node Raft cluster bootstrapped")
      host.log("  Phase 2: KV data seeded (index.html, status.json)")
      host.log("  Phase 3: VirtioFS daemon connected to Raft cluster")
      host.log("  Phase 4: aspen-net daemon with SOCKS5 proxy running")
      host.log("  Phase 5: Guest A booted with VirtioFS mount + nginx serving from Raft KV")
      host.log("  Phase 6: Service published to mesh via CLI")
      host.log("  Phase 7: Traffic routed through mesh — content from Raft KV -> VirtioFS -> nginx -> SOCKS5 -> iroh -> curl")
    '';
  }
