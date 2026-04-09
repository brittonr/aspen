# Shared wait helpers for NixOS VM integration tests.
#
# Import in a VM test's testScript:
#   helpers = import ./lib/wait-helpers.nix {};
#
# Then use:
#   helpers.wait_for_service(node, "aspen-node")
#   helpers.wait_for_cluster_ticket(node)
#   helpers.wait_for_healthy(node, get_ticket)
#   helpers.wait_for_socket(node, "/run/aspen/aspen.sock")
#
# Each helper is a Python snippet string that can be interpolated into
# testScript. Use them as functions by embedding them in the test's
# Python scope.
{}: {
  # Python helper functions to embed in testScript.
  # Usage: include `${helpers.pythonHelpers}` at the top of testScript.
  pythonHelpers = ''
    import time

    def wait_for_service(node, service_name="aspen-node", timeout=60):
        """Wait for a systemd service to be active."""
        node.wait_for_unit(f"{service_name}.service", timeout=timeout)

    def wait_for_cluster_ticket(node, path="/var/lib/aspen/cluster-ticket.txt", timeout=30):
        """Wait for the cluster ticket file to appear."""
        node.wait_for_file(path, timeout=timeout)

    def wait_for_socket(node, path, timeout=30):
        """Wait for a Unix socket to appear."""
        node.wait_for_file(path, timeout=timeout)

    def wait_for_open_port(node, port, timeout=30):
        """Wait for a TCP port to be listening."""
        node.wait_for_open_port(port, timeout=timeout)

    def wait_for_aspen_ready(node, timeout=60):
        """Wait for aspen-node service + cluster ticket (standard bootstrap)."""
        wait_for_service(node, "aspen-node", timeout=timeout)
        wait_for_cluster_ticket(node)

    def wait_for_healthy(node, get_ticket_fn, timeout=60):
        """Wait for node to pass cluster health check via CLI.

        get_ticket_fn: callable that takes a node and returns the ticket string.
        """
        wait_for_aspen_ready(node, timeout=timeout)
        ticket = get_ticket_fn(node)
        node.wait_until_succeeds(
            f"aspen-cli --ticket '{ticket}' cluster health 2>/dev/null",
            timeout=timeout,
        )

    def wait_for_job_done(node, get_ticket_fn, job_id, timeout=300):
        """Poll until a job reaches a terminal state."""
        ticket = get_ticket_fn(node)
        deadline = time.time() + timeout
        while time.time() < deadline:
            result = node.succeed(
                f"aspen-cli --ticket '{ticket}' job status {job_id} --json 2>/dev/null || echo '{{}}'"
            )
            if '"completed"' in result or '"failed"' in result or '"cancelled"' in result:
                return result
            time.sleep(2)
        raise Exception(f"job {job_id} did not finish within {timeout}s")

    def wait_for_leader_elected(node, get_ticket_fn, timeout=60):
        """Wait until the cluster reports a leader."""
        ticket = get_ticket_fn(node)
        node.wait_until_succeeds(
            f"aspen-cli --ticket '{ticket}' cluster metrics --json 2>/dev/null | grep -q leader",
            timeout=timeout,
        )
  '';
}
