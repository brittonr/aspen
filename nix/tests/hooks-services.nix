# NixOS VM integration test for Aspen hooks and service registry.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises the hook system and service registry CLI commands end-to-end:
#
#   - Hooks: list handlers, get metrics, trigger events, create hook URLs
#   - Service registry: register, discover, get, list, heartbeat,
#     health updates, metadata updates, deregister
#
# Run:
#   nix build .#checks.x86_64-linux.hooks-services-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.hooks-services-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  serviceRegistryPluginWasm,
  hooksPluginWasm,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "hooks-svc-vm-test";

  # WASM plugin helpers (service-registry handler is WASM-only)
  pluginHelpers = import ./lib/wasm-plugins.nix {
    inherit pkgs aspenCliPlugins;
    plugins = [
      {
        name = "service-registry";
        wasm = serviceRegistryPluginWasm;
      }
      {
        name = "hooks";
        wasm = hooksPluginWasm;
      }
    ];
  };
in
  pkgs.testers.nixosTest {
    name = "hooks-services";
    skipLint = true;

    nodes = {
      node1 = {
        imports = [
          ../../nix/modules/aspen-node.nix
          pluginHelpers.nixosConfig
        ];

        services.aspen.node = {
          enable = true;
          package = aspenNodePackage;
          nodeId = 1;
          inherit cookie;
          secretKey = secretKey;
          storageBackend = "redb";
          dataDir = "/var/lib/aspen";
          logLevel = "info,aspen=debug";
          relayMode = "disabled";
          enableWorkers = false;
          enableCi = false;
          features = [];
        };

        environment.systemPackages = [aspenCliPackage];

        networking.firewall.enable = false;

        virtualisation.memorySize = 4096;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket():
          """Read the cluster ticket written by aspen-node on startup."""
          return node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          """Run aspen-cli --json with the cluster ticket and return parsed JSON."""
          ticket = get_ticket()
          run = (
              f"aspen-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli_out.json 2>/dev/null"
          )
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              node1.log(f"cli() JSON parse failed, raw={raw!r}")
              return raw.strip()

      def cli_text(cmd):
          """Run aspen-cli (human output) with the cluster ticket."""
          ticket = get_ticket()
          node1.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node1.succeed("cat /tmp/_cli_out.txt")

      # ── cluster boot ─────────────────────────────────────────────────
      start_all()

      node1.wait_for_unit("aspen-node.service")
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )

      cli_text("cluster init")
      time.sleep(2)

      # ── install WASM plugins (service-registry handler is WASM-only) ─
      ${pluginHelpers.installPluginsScript}

      status = cli("cluster status")
      node1.log(f"Cluster status: {status}")

      # ================================================================
      # HOOKS EVENT SYSTEM
      # ================================================================

      with subtest("hook list"):
          out = cli("hook list")
          node1.log(f"hooks list: {out}")
          if isinstance(out, dict):
              node1.log(f"hooks system enabled={out.get('is_enabled')}")

      with subtest("hook metrics"):
          out = cli("hook metrics")
          node1.log(f"hooks metrics: {out}")
          if isinstance(out, dict):
              node1.log(
                  f"hooks metrics: total_events="
                  f"{out.get('total_events_processed', 0)}"
              )

      with subtest("hook trigger write_committed"):
          out = cli("hook trigger write_committed", check=False)
          node1.log(f"hook trigger write_committed: {out}")
          if isinstance(out, dict):
              node1.log(
                  f"dispatched_count={out.get('dispatched_count', 0)}, "
                  f"is_success={out.get('is_success')}"
              )

      with subtest("hook trigger with payload"):
          out = cli(
              "hook trigger leader_elected "
              "--payload '{\"node_id\": 1, \"term\": 42}'",
              check=False,
          )
          node1.log(f"hook trigger leader_elected: {out}")

      with subtest("hooks metrics after triggers"):
          out = cli("hook metrics")
          total = out.get("total_events_processed", 0)
          node1.log(f"hooks metrics after triggers: total_events={total}")

      with subtest("hook create-url"):
          out = cli(
              "hook create-url --event-type write_committed --expires 1",
              check=False,
          )
          node1.log(f"hook create-url: {out}")
          if isinstance(out, dict) and out.get("url"):
              hook_url = out.get("url", "")
              assert hook_url.startswith("aspenhook"), \
                  f"hook URL should start with aspenhook: {hook_url[:40]}"
              node1.log(f"hook URL created: {hook_url[:60]}...")

      # ================================================================
      # SERVICE REGISTRY
      # ================================================================

      with subtest("service register"):
          out = cli(
              "service register api-gateway gw-1 192.168.1.100:8080 "
              "--svc-version 2.1.0 --tags web,gateway --weight 100 --ttl 60000"
          )
          assert out.get("is_success") is True, \
              f"service register failed: {out}"
          fencing_token_gw1 = out.get("fencing_token")
          assert fencing_token_gw1 is not None, \
              f"no fencing token for registered service: {out}"
          node1.log(f"service registered: gw-1 token={fencing_token_gw1}")

      with subtest("service register second instance"):
          out = cli(
              "service register api-gateway gw-2 192.168.1.101:8080 "
              "--svc-version 2.1.0 --tags web,gateway --weight 50 --ttl 60000"
          )
          assert out.get("is_success") is True, \
              f"register second instance failed: {out}"
          fencing_token_gw2 = out.get("fencing_token")
          node1.log(f"service registered: gw-2 token={fencing_token_gw2}")

      with subtest("service register different service"):
          out = cli(
              "service register auth-service auth-1 192.168.1.200:9090 "
              "--svc-version 1.0.0 --tags internal --weight 100 --ttl 60000"
          )
          assert out.get("is_success") is True, \
              f"register auth service failed: {out}"
          fencing_token_auth = out.get("fencing_token")
          node1.log(f"service registered: auth-1 token={fencing_token_auth}")

      with subtest("service list all"):
          out = cli("service list")
          assert out.get("is_success") is True, \
              f"service list failed: {out}"
          services = out.get("services", [])
          assert len(services) >= 2, \
              f"expected at least 2 services, got {len(services)}: {services}"
          assert "api-gateway" in services, \
              f"api-gateway not in service list: {services}"
          assert "auth-service" in services, \
              f"auth-service not in service list: {services}"
          node1.log(f"service list: {services}")

      with subtest("service list by prefix"):
          out = cli("service list api")
          services = out.get("services", [])
          assert "api-gateway" in services, \
              f"api-gateway not in filtered list: {services}"
          # auth-service should not be in api-prefixed list
          node1.log(f"service list prefix=api: {services}")

      with subtest("service discover"):
          out = cli("service discover api-gateway")
          assert out.get("is_success") is True, \
              f"service discover failed: {out}"
          instances = out.get("instances", [])
          assert len(instances) == 2, \
              f"expected 2 api-gateway instances, got {len(instances)}: {instances}"
          ids = [i["instance_id"] for i in instances]
          assert "gw-1" in ids and "gw-2" in ids, \
              f"expected gw-1 and gw-2 in instances: {ids}"
          node1.log(f"service discover api-gateway: {len(instances)} instances")

      with subtest("service discover with version filter"):
          out = cli(
              "service discover api-gateway --version-prefix 2.1"
          )
          instances = out.get("instances", [])
          assert len(instances) == 2, \
              f"expected 2 instances matching v2.1: {instances}"
          node1.log("service discover with version filter: OK")

      with subtest("service get instance"):
          out = cli("service get api-gateway gw-1")
          assert out.get("is_success") is True, \
              f"service get failed: {out}"
          assert out.get("was_found") is True, \
              f"instance not found: {out}"
          inst = out.get("instance", {})
          assert inst.get("address") == "192.168.1.100:8080", \
              f"address mismatch: {inst}"
          assert inst.get("version") == "2.1.0", \
              f"version mismatch: {inst}"
          node1.log(f"service get gw-1: {inst}")

      with subtest("service heartbeat"):
          out = cli(
              f"service heartbeat api-gateway gw-1 "
              f"--fencing-token {fencing_token_gw1}"
          )
          assert out.get("is_success") is True, \
              f"heartbeat failed: {out}"
          node1.log("service heartbeat: OK")

      with subtest("service health update"):
          out = cli(
              f"service health api-gateway gw-2 "
              f"--fencing-token {fencing_token_gw2} --status unhealthy"
          )
          assert out.get("is_success") is True, \
              f"health update failed: {out}"
          node1.log("service health update to unhealthy: OK")

      with subtest("service discover healthy only"):
          out = cli("service discover api-gateway --healthy-only")
          instances = out.get("instances", [])
          # gw-2 is unhealthy, so only gw-1 should appear
          ids = [i["instance_id"] for i in instances]
          assert "gw-1" in ids, \
              f"gw-1 should be in healthy instances: {ids}"
          assert "gw-2" not in ids, \
              f"gw-2 (unhealthy) should be filtered out: {ids}"
          node1.log(f"service discover healthy only: {ids}")

      with subtest("service health restore"):
          out = cli(
              f"service health api-gateway gw-2 "
              f"--fencing-token {fencing_token_gw2} --status healthy"
          )
          assert out.get("is_success") is True, \
              f"health restore failed: {out}"
          node1.log("service health restored to healthy: OK")

      with subtest("service update metadata"):
          out = cli(
              f"service update api-gateway gw-1 "
              f"--fencing-token {fencing_token_gw1} "
              f"--svc-version 2.2.0 --weight 200"
          )
          assert out.get("is_success") is True, \
              f"service update failed: {out}"

          # Verify update took effect
          out = cli("service get api-gateway gw-1")
          inst = out.get("instance", {})
          assert inst.get("version") == "2.2.0", \
              f"version not updated: {inst}"
          assert inst.get("weight") == 200, \
              f"weight not updated: {inst}"
          node1.log("service update metadata: OK")

      with subtest("service get non-existent"):
          out = cli("service get api-gateway gw-99", check=False)
          if isinstance(out, dict):
              assert out.get("was_found") is False, \
                  f"non-existent instance should not be found: {out}"
          node1.log("service get non-existent: OK")

      with subtest("service deregister"):
          out = cli(
              f"service deregister api-gateway gw-2 "
              f"--fencing-token {fencing_token_gw2}"
          )
          assert out.get("is_success") is True, \
              f"deregister failed: {out}"

          # Verify gw-2 is gone
          out = cli("service discover api-gateway")
          instances = out.get("instances", [])
          ids = [i["instance_id"] for i in instances]
          assert "gw-2" not in ids, \
              f"gw-2 should be deregistered: {ids}"
          assert "gw-1" in ids, \
              f"gw-1 should still be present: {ids}"
          node1.log("service deregister: OK")

      with subtest("service cleanup"):
          cli(
              f"service deregister api-gateway gw-1 "
              f"--fencing-token {fencing_token_gw1}"
          )
          cli(
              f"service deregister auth-service auth-1 "
              f"--fencing-token {fencing_token_auth}"
          )

          out = cli("service list")
          services = out.get("services", [])
          node1.log(f"service list after cleanup: {services}")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All hooks and service registry integration tests passed!")
    '';
  }
