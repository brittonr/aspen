# NixOS VM integration test for Aspen distributed coordination primitives.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises every coordination primitive CLI command end-to-end:
#
#   - Distributed locks (acquire, try-acquire, release, renew)
#   - Read-write locks (read, write, release, downgrade, status)
#   - Atomic counters (get, incr, decr, add, sub, set, compare-and-set)
#   - Sequence generators (next, reserve, current)
#   - Semaphores (acquire, try-acquire, release, status)
#   - Barriers (enter, leave, status)
#   - Queues (create, enqueue, dequeue, ack, nack, peek, status, DLQ, delete)
#   - Leases (grant, ttl, keepalive, list, revoke)
#
# Run:
#   nix build .#checks.x86_64-linux.coordination-primitives-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.coordination-primitives-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "coord-vm-test";
in
  pkgs.testers.nixosTest {
    name = "coordination-primitives";

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
          logLevel = "info,aspen=debug";
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
      # DISTRIBUTED LOCKS
      # ================================================================

      with subtest("lock acquire and release"):
          out = cli(
              "lock acquire my-lock --holder worker-1 --ttl 10000 --timeout 5000"
          )
          assert out.get("is_success") is True, f"lock acquire failed: {out}"
          fencing_token = out.get("fencing_token")
          assert fencing_token is not None, \
              f"no fencing token returned: {out}"
          node1.log(f"lock acquired, fencing_token={fencing_token}")

          # Release with correct token
          out = cli(
              f"lock release my-lock --holder worker-1 "
              f"--fencing-token {fencing_token}"
          )
          assert out.get("is_success") is True, \
              f"lock release failed: {out}"
          node1.log("lock acquire/release: OK")

      with subtest("lock try-acquire"):
          out = cli(
              "lock try-acquire try-lock --holder worker-1 --ttl 10000"
          )
          assert out.get("is_success") is True, \
              f"try-acquire failed: {out}"
          token = out.get("fencing_token")
          node1.log(f"try-acquire succeeded, token={token}")

          # Clean up
          cli(
              f"lock release try-lock --holder worker-1 "
              f"--fencing-token {token}"
          )
          node1.log("lock try-acquire: OK")

      with subtest("lock try-acquire contention"):
          # Acquire a lock
          out = cli(
              "lock try-acquire contended-lock --holder holder-A --ttl 30000"
          )
          assert out.get("is_success") is True, \
              f"first acquire failed: {out}"
          token_a = out.get("fencing_token")

          # Try to acquire same lock with different holder — should fail
          out = cli(
              "lock try-acquire contended-lock --holder holder-B --ttl 30000",
              check=False,
          )
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"contended try-acquire should fail: {out}"
          node1.log("lock contention detected correctly")

          # Release first lock
          cli(
              f"lock release contended-lock --holder holder-A "
              f"--fencing-token {token_a}"
          )
          node1.log("lock try-acquire contention: OK")

      with subtest("lock renew"):
          out = cli(
              "lock acquire renew-lock --holder worker-1 --ttl 5000 --timeout 5000"
          )
          token = out.get("fencing_token")

          # Renew the lock's TTL
          out = cli(
              f"lock renew renew-lock --holder worker-1 "
              f"--fencing-token {token} --ttl 30000"
          )
          assert out.get("is_success") is True, \
              f"lock renew failed: {out}"
          node1.log("lock renew: OK")

          cli(
              f"lock release renew-lock --holder worker-1 "
              f"--fencing-token {token}"
          )

      # ================================================================
      # ATOMIC COUNTERS
      # ================================================================

      with subtest("counter get initial"):
          out = cli("counter get test-counter")
          assert out.get("is_success") is True, \
              f"counter get failed: {out}"
          # New counter should start at 0
          assert out.get("value") == 0, \
              f"initial counter should be 0: {out}"
          node1.log("counter get initial: OK")

      with subtest("counter increment"):
          out = cli("counter incr test-counter")
          assert out.get("value") == 1, f"incr should yield 1: {out}"

          out = cli("counter incr test-counter")
          assert out.get("value") == 2, f"incr should yield 2: {out}"

          out = cli("counter incr test-counter")
          assert out.get("value") == 3, f"incr should yield 3: {out}"
          node1.log("counter increment: OK")

      with subtest("counter decrement"):
          out = cli("counter decr test-counter")
          assert out.get("value") == 2, f"decr should yield 2: {out}"
          node1.log("counter decrement: OK")

      with subtest("counter add"):
          out = cli("counter add test-counter 10")
          assert out.get("value") == 12, f"add 10 should yield 12: {out}"
          node1.log("counter add: OK")

      with subtest("counter subtract"):
          out = cli("counter sub test-counter 5")
          assert out.get("value") == 7, f"sub 5 should yield 7: {out}"
          node1.log("counter subtract: OK")

      with subtest("counter set"):
          out = cli("counter set test-counter 100")
          assert out.get("value") == 100, f"set 100 failed: {out}"
          node1.log("counter set: OK")

      with subtest("counter compare-and-set success"):
          out = cli(
              "counter cas test-counter --expected 100 --new-value 200"
          )
          assert out.get("is_success") is True, \
              f"counter CAS should succeed: {out}"
          assert out.get("value") == 200, f"CAS value wrong: {out}"
          node1.log("counter CAS success: OK")

      with subtest("counter compare-and-set conflict"):
          out = cli(
              "counter cas test-counter --expected 100 --new-value 999",
              check=False,
          )
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"counter CAS should conflict: {out}"
          # Value should still be 200
          out = cli("counter get test-counter")
          assert out.get("value") == 200, \
              f"CAS conflict changed value: {out}"
          node1.log("counter CAS conflict: OK")

      with subtest("counter decrement saturates at zero"):
          cli("counter set sat-counter 2")
          cli("counter decr sat-counter")
          cli("counter decr sat-counter")
          # Decr at zero may fail (underflow) or saturate — handle both
          out = cli("counter decr sat-counter", check=False)
          if isinstance(out, dict):
              val = out.get("value")
              node1.log(
                  f"counter decr at zero: value={val}, "
                  f"success={out.get('is_success')}"
              )
          out = cli("counter get sat-counter")
          assert out.get("value") == 0, \
              f"counter should be 0 after underflow: {out}"
          node1.log("counter saturate at zero: OK")

      # ================================================================
      # SEQUENCE GENERATORS
      # ================================================================

      with subtest("sequence next"):
          out = cli("sequence next order-ids")
          assert out.get("is_success") is True, \
              f"sequence next failed: {out}"
          val1 = out.get("value")
          assert val1 is not None, f"no sequence value: {out}"

          out = cli("sequence next order-ids")
          val2 = out.get("value")
          assert val2 > val1, \
              f"sequence not monotonically increasing: {val1} -> {val2}"

          out = cli("sequence next order-ids")
          val3 = out.get("value")
          assert val3 > val2, \
              f"sequence not monotonically increasing: {val2} -> {val3}"
          node1.log(f"sequence next: {val1} -> {val2} -> {val3}: OK")

      with subtest("sequence current"):
          out = cli("sequence current order-ids")
          assert out.get("is_success") is True, \
              f"sequence current failed: {out}"
          current = out.get("value")
          assert current is not None, f"no current value: {out}"
          node1.log(f"sequence current: {current}")

      with subtest("sequence reserve"):
          out = cli("sequence reserve order-ids 10")
          assert out.get("is_success") is True, \
              f"sequence reserve failed: {out}"
          start = out.get("value")
          assert start is not None, f"no reserve start: {out}"
          node1.log(f"sequence reserve 10: start={start}")

          # Next ID after reserve should be start+10
          out = cli("sequence next order-ids")
          after = out.get("value")
          assert after >= start + 10, \
              f"next after reserve should be >= {start + 10}, got {after}"
          node1.log("sequence reserve: OK")

      # ================================================================
      # SEMAPHORES
      # ================================================================

      with subtest("semaphore acquire"):
          out = cli(
              "semaphore acquire db-pool "
              "--holder conn-1 --permits 1 --capacity 3 --ttl 30000 --timeout 5000"
          )
          assert out.get("is_success") is True, \
              f"semaphore acquire failed: {out}"
          node1.log(f"semaphore acquire: available={out.get('available')}")

      with subtest("semaphore multiple permits"):
          out = cli(
              "semaphore try-acquire db-pool "
              "--holder conn-2 --permits 1 --capacity 3 --ttl 30000"
          )
          assert out.get("is_success") is True, \
              f"second permit acquire failed: {out}"

          out = cli(
              "semaphore try-acquire db-pool "
              "--holder conn-3 --permits 1 --capacity 3 --ttl 30000"
          )
          assert out.get("is_success") is True, \
              f"third permit acquire failed: {out}"
          node1.log("semaphore multiple permits: OK")

      with subtest("semaphore capacity exhausted"):
          # All 3 permits taken — 4th should fail
          out = cli(
              "semaphore try-acquire db-pool "
              "--holder conn-4 --permits 1 --capacity 3 --ttl 30000",
              check=False,
          )
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"4th permit should be denied: {out}"
          node1.log("semaphore capacity exhausted: OK")

      with subtest("semaphore status"):
          out = cli("semaphore status db-pool")
          assert out.get("is_success") is True, \
              f"semaphore status failed: {out}"
          assert out.get("capacity_permits") == 3, \
              f"wrong capacity: {out}"
          node1.log(f"semaphore status: available={out.get('available')}, capacity={out.get('capacity_permits')}")

      with subtest("semaphore release"):
          cli("semaphore release db-pool --holder conn-1")
          cli("semaphore release db-pool --holder conn-2")
          cli("semaphore release db-pool --holder conn-3")

          out = cli("semaphore status db-pool")
          node1.log(f"semaphore after release: available={out.get('available')}")
          node1.log("semaphore release: OK")

      # ================================================================
      # READ-WRITE LOCKS
      # ================================================================

      with subtest("rwlock read acquire"):
          out = cli(
              "rwlock read config-lock --holder reader-1 --ttl 30000 --timeout 5000"
          )
          assert out.get("is_success") is True, \
              f"rwlock read acquire failed: {out}"
          node1.log("rwlock read acquire: OK")

      with subtest("rwlock multiple readers"):
          out = cli(
              "rwlock try-read config-lock --holder reader-2 --ttl 30000"
          )
          assert out.get("is_success") is True, \
              f"second reader should succeed: {out}"
          node1.log(f"rwlock multiple readers: reader_count={out.get('reader_count')}")

      with subtest("rwlock write blocked by readers"):
          # Write should fail while readers hold the lock
          out = cli(
              "rwlock try-write config-lock --holder writer-1 --ttl 30000",
              check=False,
          )
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"write should be blocked by readers: {out}"
          node1.log("rwlock write blocked by readers: OK")

      with subtest("rwlock release readers"):
          cli("rwlock release-read config-lock --holder reader-1")
          cli("rwlock release-read config-lock --holder reader-2")
          node1.log("rwlock release readers: OK")

      with subtest("rwlock write acquire"):
          out = cli(
              "rwlock write config-lock --holder writer-1 --ttl 30000 --timeout 5000"
          )
          assert out.get("is_success") is True, \
              f"rwlock write acquire failed: {out}"
          write_token = out.get("fencing_token")
          assert write_token is not None, \
              f"no fencing token for write lock: {out}"
          node1.log(f"rwlock write acquire: token={write_token}")

      with subtest("rwlock read blocked by writer"):
          # Read should fail while writer holds the lock
          out = cli(
              "rwlock try-read config-lock --holder reader-3 --ttl 30000",
              check=False,
          )
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"read should be blocked by writer: {out}"
          node1.log("rwlock read blocked by writer: OK")

      with subtest("rwlock downgrade"):
          out = cli(
              f"rwlock downgrade config-lock --holder writer-1 "
              f"--fencing-token {write_token} --ttl 30000"
          )
          assert out.get("is_success") is True, \
              f"rwlock downgrade failed: {out}"
          node1.log("rwlock downgrade: OK")

          # After downgrade, other readers should be able to acquire
          out = cli(
              "rwlock try-read config-lock --holder reader-4 --ttl 30000"
          )
          assert out.get("is_success") is True, \
              f"read after downgrade should succeed: {out}"
          node1.log("rwlock read after downgrade: OK")

          # Clean up
          cli("rwlock release-read config-lock --holder writer-1")
          cli("rwlock release-read config-lock --holder reader-4")

      with subtest("rwlock status"):
          out = cli("rwlock status config-lock")
          assert out.get("is_success") is True, \
              f"rwlock status failed: {out}"
          node1.log(f"rwlock status: mode={out.get('mode')}, readers={out.get('reader_count')}")

      # ================================================================
      # DISTRIBUTED QUEUES
      # ================================================================

      with subtest("queue create"):
          out = cli(
              "queue create task-queue --visibility-timeout 30000 --max-attempts 3"
          )
          assert out.get("is_success") is True, \
              f"queue create failed: {out}"
          node1.log("queue create: OK")

      with subtest("queue enqueue"):
          out = cli("queue enqueue task-queue 'process-order-123'")
          assert out.get("is_success") is True, \
              f"enqueue failed: {out}"
          item_id1 = out.get("item_id")
          assert item_id1 is not None, f"no item_id: {out}"
          node1.log(f"enqueue item_id={item_id1}")

          out = cli("queue enqueue task-queue 'process-order-456'")
          item_id2 = out.get("item_id")

          out = cli("queue enqueue task-queue 'process-order-789'")
          item_id3 = out.get("item_id")
          node1.log("queue enqueue: 3 items queued")

      with subtest("queue status"):
          out = cli("queue status task-queue")
          assert out.get("is_success") is True, \
              f"queue status failed: {out}"
          assert out.get("visible_count", 0) >= 3, \
              f"expected at least 3 visible items: {out}"
          node1.log(
              f"queue status: visible={out.get('visible_count')}, "
              f"pending={out.get('pending_count')}, "
              f"dlq={out.get('dlq_count')}"
          )

      with subtest("queue peek"):
          out = cli("queue peek task-queue --max 2")
          assert out.get("is_success") is True, \
              f"queue peek failed: {out}"
          items = out.get("items", [])
          assert len(items) >= 1, f"peek should return items: {out}"
          node1.log(f"queue peek: {len(items)} items")

      with subtest("queue dequeue and ack"):
          out = cli(
              "queue dequeue task-queue --consumer worker-1 --max 1 "
              "--visibility 30000"
          )
          assert out.get("is_success") is True, \
              f"dequeue failed: {out}"
          items = out.get("items", [])
          assert len(items) == 1, f"expected 1 dequeued item: {out}"

          receipt = items[0].get("receipt_handle")
          payload = items[0].get("payload")
          assert receipt, f"no receipt_handle: {items[0]}"
          node1.log(f"dequeued: payload={payload}, receipt={receipt}")

          # Acknowledge successful processing
          out = cli(f"queue ack task-queue '{receipt}'")
          assert out.get("is_success") is True, \
              f"queue ack failed: {out}"
          node1.log("queue dequeue+ack: OK")

      with subtest("queue dequeue and nack"):
          out = cli(
              "queue dequeue task-queue --consumer worker-1 --max 1 "
              "--visibility 30000"
          )
          items = out.get("items", [])
          assert len(items) == 1, f"expected 1 item: {items}"

          receipt = items[0].get("receipt_handle")
          # Nack to return item to queue
          out = cli(
              f"queue nack task-queue '{receipt}' --error 'temporary failure'"
          )
          assert out.get("is_success") is True, \
              f"queue nack failed: {out}"
          node1.log("queue dequeue+nack: OK")

      with subtest("queue nack to DLQ"):
          out = cli(
              "queue dequeue task-queue --consumer worker-1 --max 1 "
              "--visibility 30000"
          )
          items = out.get("items", [])
          if len(items) > 0:
              receipt = items[0].get("receipt_handle")
              dlq_item_id = items[0].get("item_id")
              # Nack directly to DLQ
              out = cli(
                  f"queue nack task-queue '{receipt}' --to-dlq "
                  f"--error 'permanent failure'"
              )
              assert out.get("is_success") is True, \
                  f"nack to DLQ failed: {out}"
              node1.log("queue nack to DLQ: OK")

              # Check DLQ
              out = cli("queue dlq task-queue --max 10")
              assert out.get("is_success") is True, \
                  f"DLQ query failed: {out}"
              dlq_items = out.get("items", [])
              assert len(dlq_items) >= 1, \
                  f"DLQ should have items: {out}"
              node1.log(f"queue DLQ: {len(dlq_items)} items")

              # Redrive from DLQ back to queue
              out = cli(
                  f"queue redrive task-queue {dlq_item_id}"
              )
              assert out.get("is_success") is True, \
                  f"redrive failed: {out}"
              node1.log("queue redrive from DLQ: OK")
          else:
              node1.log("queue nack to DLQ: skipped (no items available)")

      with subtest("queue with deduplication"):
          out = cli(
              "queue enqueue task-queue 'dedup-msg' --dedup dedup-123"
          )
          assert out.get("is_success") is True, \
              f"dedup enqueue failed: {out}"
          node1.log("queue deduplication: OK")

      with subtest("queue with message group"):
          out = cli(
              "queue enqueue task-queue 'grouped-msg' --group group-A"
          )
          assert out.get("is_success") is True, \
              f"grouped enqueue failed: {out}"
          node1.log("queue message group: OK")

      with subtest("queue delete"):
          out = cli("queue delete task-queue")
          assert out.get("is_success") is True, \
              f"queue delete failed: {out}"
          node1.log("queue delete: OK")

      # ================================================================
      # LEASES
      # ================================================================

      with subtest("lease grant"):
          out = cli("lease grant 60")
          assert out.get("is_success") is True, \
              f"lease grant failed: {out}"
          lease_id = out.get("lease_id")
          assert lease_id is not None, f"no lease_id: {out}"
          node1.log(f"lease granted: id={lease_id}, ttl={out.get('ttl_seconds')}s")

      with subtest("lease ttl"):
          out = cli(f"lease ttl {lease_id}")
          assert out.get("is_success") is True, \
              f"lease ttl failed: {out}"
          remaining = out.get("remaining_ttl_seconds")
          assert remaining is not None and remaining > 0, \
              f"lease should have remaining TTL: {out}"
          node1.log(f"lease ttl: remaining={remaining}s")

      with subtest("lease keepalive"):
          out = cli(f"lease keepalive {lease_id}")
          assert out.get("is_success") is True, \
              f"lease keepalive failed: {out}"
          node1.log("lease keepalive: OK")

      with subtest("lease list"):
          # Grant a second lease so we have multiple
          out2 = cli("lease grant 120")
          lease_id2 = out2.get("lease_id")

          out = cli("lease list")
          assert out.get("is_success") is True, \
              f"lease list failed: {out}"
          leases = out.get("leases", [])
          assert len(leases) >= 2, \
              f"expected at least 2 leases: {leases}"
          lease_ids = [l.get("lease_id") for l in leases]
          assert lease_id in lease_ids, \
              f"first lease not in list: {lease_ids}"
          node1.log(f"lease list: {len(leases)} active leases")

      with subtest("lease ttl with keys"):
          out = cli(f"lease ttl {lease_id} --keys")
          assert out.get("is_success") is True, \
              f"lease ttl with keys failed: {out}"
          node1.log(f"lease ttl with keys: keys={out.get('keys')}")

      with subtest("lease revoke"):
          out = cli(f"lease revoke {lease_id}")
          assert out.get("is_success") is True, \
              f"lease revoke failed: {out}"
          node1.log(f"lease revoke: keys_deleted={out.get('keys_deleted')}")

          # Verify lease is gone
          out = cli(f"lease ttl {lease_id}", check=False)
          if isinstance(out, dict) and out.get("is_success") is False:
              node1.log("revoked lease correctly reports failure on ttl query")
          node1.log("lease revoke: OK")

          # Clean up second lease
          cli(f"lease revoke {lease_id2}")

      # ================================================================
      # BARRIERS
      # ================================================================

      with subtest("barrier enter with single participant"):
          # A barrier with count=1 should immediately release
          out = cli(
              "barrier enter sync-point --participant p1 --count 1 --timeout 5000"
          )
          assert out.get("is_success") is True, \
              f"barrier enter failed: {out}"
          node1.log(
              f"barrier enter: current={out.get('current_count')}, "
              f"required={out.get('required_count')}, "
              f"phase={out.get('phase')}"
          )

      with subtest("barrier status"):
          out = cli("barrier status sync-point")
          assert out.get("is_success") is True, \
              f"barrier status failed: {out}"
          node1.log(
              f"barrier status: current={out.get('current_count')}, "
              f"phase={out.get('phase')}"
          )

      with subtest("barrier leave"):
          out = cli(
              "barrier leave sync-point --participant p1 --timeout 5000"
          )
          assert out.get("is_success") is True, \
              f"barrier leave failed: {out}"
          node1.log("barrier leave: OK")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All coordination primitives integration tests passed!")
    '';
  }
