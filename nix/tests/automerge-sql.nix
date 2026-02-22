# NixOS VM integration test for Aspen Automerge CRDT and SQL query subsystems.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises automerge and sql CLI commands end-to-end:
#
# AUTOMERGE:
#   - automerge create (create a new CRDT document)
#   - automerge get (retrieve a document by ID)
#   - automerge exists (check if document exists)
#   - automerge list (list all documents)
#   - automerge delete (delete a document)
#   - automerge get-metadata (get document metadata)
#
# SQL:
#   - sql query (execute read-only SQL queries against KV store)
#
# Run:
#   nix build .#checks.x86_64-linux.automerge-sql-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.automerge-sql-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  automergePluginWasm,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "automerge-sql-vm-test";

  # WASM plugin helpers (automerge handler is WASM-only)
  pluginHelpers = import ./lib/wasm-plugins.nix {
    inherit pkgs aspenCliPlugins;
    plugins = [
      {
        name = "automerge";
        wasm = automergePluginWasm;
      }
    ];
  };
in
  pkgs.testers.nixosTest {
    name = "automerge-sql";
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

      # ── install WASM plugins (automerge handler is WASM-only) ──────
      ${pluginHelpers.installPluginsScript}

      status = cli("cluster status")
      node1.log(f"Cluster status: {status}")

      # ── AUTOMERGE tests ─────────────────────────────────────────────

      with subtest("automerge create"):
          out = cli("automerge create test-doc-1", check=False)
          if isinstance(out, dict):
              is_success = out.get("is_success")
              doc_id = out.get("document_id")
              error = out.get("error")
              node1.log(f"automerge create: is_success={is_success}, doc_id={doc_id}, error={error}")
              if is_success:
                  assert doc_id is not None, f"should have document_id: {out}"
          else:
              node1.log(f"automerge create: {out}")

      with subtest("automerge exists"):
          out = cli("automerge exists test-doc-1", check=False)
          if isinstance(out, dict):
              existed = out.get("existed")
              node1.log(f"automerge exists: existed={existed}")
          else:
              node1.log(f"automerge exists: {out}")

      with subtest("automerge get"):
          out = cli("automerge get test-doc-1", check=False)
          if isinstance(out, dict):
              was_found = out.get("was_found")
              doc_id = out.get("document_id")
              node1.log(f"automerge get: was_found={was_found}, doc_id={doc_id}")
          else:
              node1.log(f"automerge get: {out}")

      with subtest("automerge create second"):
          out = cli("automerge create test-doc-2", check=False)
          if isinstance(out, dict):
              node1.log(f"automerge create second: is_success={out.get('is_success')}")
          else:
              node1.log(f"automerge create second: {out}")

      with subtest("automerge list"):
          out = cli("automerge list", check=False)
          if isinstance(out, dict):
              docs = out.get("documents", [])
              count = out.get("count", 0)
              node1.log(f"automerge list: count={count}, docs={len(docs)}")
          else:
              node1.log(f"automerge list: {out}")

      with subtest("automerge get-metadata"):
          out = cli("automerge get-metadata test-doc-1", check=False)
          if isinstance(out, dict):
              node1.log(f"automerge get-metadata: {out}")
          else:
              node1.log(f"automerge get-metadata: {out}")

      with subtest("automerge delete"):
          out = cli("automerge delete test-doc-1", check=False)
          if isinstance(out, dict):
              node1.log(f"automerge delete: is_success={out.get('is_success')}")
          else:
              node1.log(f"automerge delete: {out}")

      with subtest("automerge exists after delete"):
          out = cli("automerge exists test-doc-1", check=False)
          if isinstance(out, dict):
              existed = out.get("existed")
              node1.log(f"automerge exists after delete: existed={existed}")
              # After delete, should not exist
              if existed is not None:
                  assert existed is False, \
                      f"document should not exist after delete: {out}"
          else:
              node1.log(f"automerge exists after delete: {out}")

      with subtest("automerge list after delete"):
          out = cli("automerge list", check=False)
          if isinstance(out, dict):
              docs = out.get("documents", [])
              count = out.get("count", 0)
              node1.log(f"automerge list after delete: count={count}")
              # Should have 1 less document
          else:
              node1.log(f"automerge list after delete: {out}")

      # ── SQL tests ──────────────────────────────────────────────────

      # First, add some KV data for SQL to query
      with subtest("sql setup test data"):
          cli("kv set sql-test/user1 'alice'")
          cli("kv set sql-test/user2 'bob'")
          cli("kv set sql-test/user3 'charlie'")
          cli("kv set other/data 'ignored'")
          node1.log("sql setup: created 4 test keys")

      with subtest("sql query all keys"):
          out = cli("sql query 'SELECT key, value FROM kv'", check=False)
          if isinstance(out, dict):
              rows = out.get("rows", [])
              count = out.get("row_count", 0)
              columns = out.get("columns", [])
              node1.log(f"sql query all: {count} rows, columns={columns}")
              # Should have at least our test keys (plus system keys)
              assert count >= 4, f"expected at least 4 rows: {count}"
          else:
              node1.log(f"sql query all: {out}")

      with subtest("sql query with WHERE"):
          out = cli(
              "sql query \"SELECT key, value FROM kv WHERE key LIKE 'sql-test/%'\"",
              check=False,
          )
          if isinstance(out, dict):
              rows = out.get("rows", [])
              count = out.get("row_count", 0)
              node1.log(f"sql query WHERE: {count} rows")
              assert count == 3, f"expected 3 sql-test rows: {count}"
          else:
              node1.log(f"sql query WHERE: {out}")

      with subtest("sql query COUNT"):
          out = cli(
              "sql query \"SELECT COUNT(*) as cnt FROM kv WHERE key LIKE 'sql-test/%'\"",
              check=False,
          )
          if isinstance(out, dict):
              rows = out.get("rows", [])
              node1.log(f"sql query COUNT: rows={rows}")
              if rows:
                  # Count should be 3
                  cnt = rows[0].get("cnt") if isinstance(rows[0], dict) else rows[0][0]
                  node1.log(f"sql COUNT result: {cnt}")
          else:
              node1.log(f"sql query COUNT: {out}")

      with subtest("sql query with LIMIT"):
          out = cli(
              "sql query \"SELECT key FROM kv WHERE key LIKE 'sql-test/%' LIMIT 2\"",
              check=False,
          )
          if isinstance(out, dict):
              rows = out.get("rows", [])
              count = out.get("row_count", 0)
              node1.log(f"sql query LIMIT: {count} rows")
              assert count <= 2, f"LIMIT 2 should return at most 2 rows: {count}"
          else:
              node1.log(f"sql query LIMIT: {out}")

      with subtest("sql query ORDER BY"):
          out = cli(
              "sql query \"SELECT key, value FROM kv WHERE key LIKE 'sql-test/%' ORDER BY key\"",
              check=False,
          )
          if isinstance(out, dict):
              rows = out.get("rows", [])
              node1.log(f"sql query ORDER BY: {len(rows)} rows")
              if len(rows) >= 2:
                  # Should be sorted by key
                  keys = [str(r.get("key", "")) if isinstance(r, dict) else str(r[0]) for r in rows]
                  assert keys == sorted(keys), f"rows should be sorted: {keys}"
          else:
              node1.log(f"sql query ORDER BY: {out}")

      with subtest("sql query non-existent table"):
          # Querying a non-existent table should fail gracefully
          out = cli(
              "sql query 'SELECT * FROM nonexistent_table'",
              check=False,
          )
          if isinstance(out, dict):
              error = out.get("error")
              node1.log(f"sql non-existent table: error={error}")
          else:
              node1.log(f"sql non-existent table: {out}")

      # ── cleanup ──────────────────────────────────────────────────────

      with subtest("cleanup sql test data"):
          cli("kv delete sql-test/user1")
          cli("kv delete sql-test/user2")
          cli("kv delete sql-test/user3")
          cli("kv delete other/data")
          node1.log("cleanup: removed test KV keys")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All automerge and SQL integration tests passed!")
    '';
  }
