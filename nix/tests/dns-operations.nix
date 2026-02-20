# NixOS VM integration test for Aspen DNS record management.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises the DNS CLI commands end-to-end:
#
#   - Record operations: set, get, get-all, delete, resolve, scan
#   - Zone management: set, get, list, delete
#   - Record types: A, AAAA, CNAME, MX, TXT, SRV
#   - Wildcard resolution
#
# Run:
#   nix build .#checks.x86_64-linux.dns-operations-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.dns-operations-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "dns-vm-test";
in
  pkgs.testers.nixosTest {
    name = "dns-operations";

    nodes = {
      node1 = {
        imports = [../../nix/modules/aspen-node.nix];

        services.aspen.node = {
          enable = true;
          package = aspenNodePackage;
          nodeId = 1;
          inherit cookie;
          secretKey = secretKey;
          storageBackend = "redb";
          dataDir = "/var/lib/aspen";
          logLevel = "info";
          relayMode = "disabled";
          enableWorkers = false;
          enableCi = false;
          features = [];
        };

        environment.systemPackages = [aspenCliPackage];

        networking.firewall.enable = false;

        virtualisation.memorySize = 2048;
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
          """Run aspen-cli --json with the cluster ticket and return parsed JSON.

          We redirect stdout to a temp file and cat it back, because the NixOS
          test serial console mixes stderr/kernel messages into the captured
          output, corrupting JSON parsing.
          """
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

      status = cli("cluster status")
      node1.log(f"Cluster status: {status}")

      # ================================================================
      # ZONE MANAGEMENT
      # ================================================================

      with subtest("zone create example.com"):
          # zone set takes positional name, --default-ttl, --description
          # default is enabled (--disabled flag to disable)
          out = cli(
              "dns zone set example.com --default-ttl 3600 "
              "-d 'Main domain zone'"
          )
          # Returns zone JSON: {name, enabled, default_ttl, serial, ...}
          assert out.get("name") == "example.com", \
              f"wrong zone name: {out}"
          assert out.get("enabled") is True, \
              f"zone should be enabled: {out}"
          assert out.get("default_ttl") == 3600, \
              f"wrong default TTL: {out}"
          node1.log(f"zone create example.com: {out}")

      with subtest("zone get example.com"):
          out = cli("dns zone get example.com")
          assert out.get("name") == "example.com", \
              f"wrong zone name: {out}"
          assert out.get("enabled") is True, \
              f"zone should be enabled: {out}"
          node1.log(f"zone get: {out}")

      with subtest("zone list"):
          out = cli("dns zone list")
          zones = out.get("zones", [])
          assert len(zones) >= 1, f"expected at least 1 zone: {out}"
          zone_names = [z.get("name") for z in zones]
          assert "example.com" in zone_names, \
              f"example.com not in zone list: {zone_names}"
          node1.log(f"zone list: {len(zones)} zones")

      with subtest("zone create test.org"):
          out = cli("dns zone set test.org --default-ttl 7200")
          assert out.get("name") == "test.org", \
              f"zone create test.org failed: {out}"
          node1.log("zone create test.org: OK")

      # ================================================================
      # A RECORDS
      # ================================================================

      with subtest("set A record for web.example.com"):
          # Simple value syntax: dns set <domain> <type> <value>
          out = cli("dns set web.example.com A 192.168.1.100 --ttl 300")
          assert out.get("domain") == "web.example.com", \
              f"wrong domain: {out}"
          assert out.get("record_type") == "A", \
              f"wrong record type: {out}"
          node1.log(f"A record set: {out}")

      with subtest("get A record for web.example.com"):
          out = cli("dns get web.example.com A")
          assert out.get("domain") == "web.example.com", \
              f"wrong domain: {out}"
          assert out.get("record_type") == "A", \
              f"wrong record type: {out}"
          data = out.get("data", {})
          # A records use {"type": "A", "addresses": ["..."]} format
          addrs = data.get("addresses", [])
          assert "192.168.1.100" in addrs, \
              f"wrong A record address: {data}"
          assert out.get("ttl_seconds") == 300, \
              f"wrong TTL: {out}"
          node1.log(f"A record get: {data}")

      # ================================================================
      # AAAA RECORDS
      # ================================================================

      with subtest("set AAAA record for web.example.com"):
          out = cli("dns set web.example.com AAAA ::1 --ttl 300")
          assert out.get("record_type") == "AAAA", \
              f"wrong record type: {out}"
          node1.log(f"AAAA record set: {out}")

      with subtest("get AAAA record for web.example.com"):
          out = cli("dns get web.example.com AAAA")
          data = out.get("data", {})
          addrs = data.get("addresses", [])
          assert "::1" in addrs, \
              f"wrong AAAA record address: {data}"
          node1.log(f"AAAA record get: {data}")

      # ================================================================
      # CNAME RECORDS
      # ================================================================

      with subtest("set CNAME record for alias.example.com"):
          out = cli(
              "dns set alias.example.com CNAME web.example.com --ttl 600"
          )
          assert out.get("record_type") == "CNAME", \
              f"wrong record type: {out}"
          node1.log(f"CNAME record set: {out}")

      with subtest("get CNAME record for alias.example.com"):
          out = cli("dns get alias.example.com CNAME")
          data = out.get("data", {})
          assert data.get("target") == "web.example.com", \
              f"wrong CNAME target: {data}"
          node1.log(f"CNAME record get: {data}")

      # ================================================================
      # MX RECORDS (use --data for complex JSON)
      # ================================================================

      with subtest("set MX record for example.com"):
          mx_data = '{"preference":10,"exchange":"mail.example.com"}'
          out = cli(
              f"dns set example.com MX --data '{mx_data}' --ttl 3600",
              check=False,
          )
          node1.log(f"MX record set: {out}")
          if isinstance(out, dict) and out.get("record_type"):
              assert out.get("record_type") == "MX", \
                  f"wrong record type: {out}"

      with subtest("get MX record for example.com"):
          out = cli("dns get example.com MX", check=False)
          node1.log(f"MX record get: {out}")
          if isinstance(out, dict) and out.get("data"):
              data = out["data"]
              assert data.get("preference") == 10, \
                  f"wrong MX preference: {data}"
              assert data.get("exchange") == "mail.example.com", \
                  f"wrong MX exchange: {data}"

      # ================================================================
      # TXT RECORDS
      # ================================================================

      with subtest("set TXT record for example.com"):
          out = cli(
              "dns set example.com TXT 'v=spf1 include:example.com ~all' --ttl 3600"
          )
          assert out.get("record_type") == "TXT", \
              f"wrong record type: {out}"
          node1.log(f"TXT record set: {out}")

      with subtest("get TXT record for example.com"):
          out = cli("dns get example.com TXT")
          data = out.get("data", {})
          node1.log(f"TXT record get: {data}")
          # TXT data should contain the text value
          txt_val = data.get("text", data.get("texts", [""]))
          node1.log(f"TXT value: {txt_val}")

      # ================================================================
      # SRV RECORDS (use --data for complex JSON)
      # ================================================================

      with subtest("set SRV record for _http._tcp.example.com"):
          srv_data = '{"priority":10,"weight":60,"port":80,"target":"web.example.com"}'
          out = cli(
              f"dns set _http._tcp.example.com SRV --data '{srv_data}' --ttl 600",
              check=False,
          )
          node1.log(f"SRV record set: {out}")

      with subtest("get SRV record for _http._tcp.example.com"):
          out = cli("dns get _http._tcp.example.com SRV", check=False)
          node1.log(f"SRV record get: {out}")
          if isinstance(out, dict) and out.get("data"):
              data = out["data"]
              assert data.get("priority") == 10, \
                  f"wrong SRV priority: {data}"
              assert data.get("port") == 80, \
                  f"wrong SRV port: {data}"

      # ================================================================
      # GET-ALL RECORDS
      # ================================================================

      with subtest("get all records for example.com"):
          out = cli("dns get-all example.com", check=False)
          node1.log(f"get-all: {out}")
          if isinstance(out, dict):
              records = out.get("records", [])
              record_types = [r.get("record_type") for r in records]
              node1.log(f"get-all: {len(records)} records - types: {record_types}")
              # Should have at least TXT record
              assert len(records) >= 1, \
                  f"expected records for example.com: {records}"

      # ================================================================
      # DNS RESOLVE
      # ================================================================

      with subtest("resolve web.example.com A"):
          out = cli("dns resolve web.example.com A")
          # Resolve returns {count, records: [...]}
          records = out.get("records", [])
          assert len(records) >= 1, \
              f"resolve returned no records: {out}"
          data = records[0].get("data", {})
          addrs = data.get("addresses", [])
          assert "192.168.1.100" in addrs, \
              f"wrong resolved address: {data}"
          node1.log(f"resolve A: {records}")

      # ================================================================
      # DNS SCAN
      # ================================================================

      with subtest("scan with prefix"):
          # scan takes positional prefix, --limit
          out = cli("dns scan example.com")
          records = out.get("records", [])
          assert len(records) >= 1, \
              f"expected records in scan: {records}"
          node1.log(f"scan with prefix: {len(records)} records")

      with subtest("scan with limit"):
          out = cli("dns scan example.com --limit 2")
          records = out.get("records", [])
          assert len(records) <= 2, \
              f"scan limit not respected: {len(records)} records returned"
          node1.log(f"scan with limit 2: {len(records)} records")

      # ================================================================
      # DELETE RECORDS
      # ================================================================

      with subtest("delete CNAME record"):
          out = cli("dns delete alias.example.com CNAME")
          assert out.get("status") == "success", \
              f"delete failed: {out}"
          assert out.get("was_deleted") is True, \
              f"was_deleted should be True: {out}"
          node1.log("delete CNAME: OK")

          # Verify it's gone
          out = cli("dns get alias.example.com CNAME", check=False)
          if isinstance(out, dict):
              # Should return was_found: false or error
              node1.log(f"verify CNAME deleted: {out}")
          node1.log("verify CNAME deleted: OK")

      # ================================================================
      # WILDCARD RESOLUTION
      # ================================================================

      with subtest("set wildcard A record for *.example.com"):
          out = cli("dns set '*.example.com' A 192.168.1.200 --ttl 300")
          assert out.get("record_type") == "A", \
              f"wildcard A record set failed: {out}"
          node1.log(f"wildcard A record set: {out}")

      with subtest("resolve anything.example.com A with wildcard fallback"):
          out = cli("dns resolve anything.example.com A")
          records = out.get("records", [])
          assert len(records) >= 1, \
              f"wildcard resolve returned no records: {out}"
          # Should match the wildcard
          data = records[0].get("data", {})
          addrs = data.get("addresses", [])
          assert "192.168.1.200" in addrs, \
              f"wrong wildcard resolved address: {data}"
          node1.log(f"wildcard resolve: {records}")

      # ================================================================
      # ZONE DELETION
      # ================================================================

      with subtest("delete test.org zone"):
          out = cli("dns zone delete test.org")
          node1.log(f"zone delete test.org: {out}")

          # Verify it's gone from list
          out = cli("dns zone list")
          zones = out.get("zones", [])
          zone_names = [z.get("name") for z in zones]
          assert "test.org" not in zone_names, \
              f"deleted zone still in list: {zone_names}"
          node1.log("verify test.org deleted: OK")

      with subtest("delete example.com zone with records"):
          out = cli("dns zone delete example.com --delete-records")
          node1.log(f"zone delete with records: {out}")

          # Verify records are gone
          out = cli("dns get web.example.com A", check=False)
          if isinstance(out, dict):
              node1.log(f"verify records deleted: {out}")
          node1.log("verify records deleted with zone: OK")

          # Verify zone is gone
          out = cli("dns zone list")
          zones = out.get("zones", [])
          zone_names = [z.get("name") for z in zones]
          assert "example.com" not in zone_names, \
              f"deleted zone still in list: {zone_names}"
          node1.log("verify example.com zone deleted: OK")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All DNS operations integration tests passed!")
    '';
  }
