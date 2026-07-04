# Design: local multiprocess multinode harness

The harness has a pure planning core and a thin process shell. The core accepts a scenario fixture, node names, state-root handles, local transport handles, command plan, expected receipts, and cleanup policy. It returns a deterministic process plan, evidence expectations, and reconciliation inputs.

The shell owns temp directory creation, process spawning, signal handling, waiting, receipt file collection, and cleanup. It must record startup, health, ingress, queue, dispatch, shutdown, and cleanup receipts where those receipts are produced. It must also record unavailable or deny evidence when a required local capability cannot be exercised.

State roots and transport roots are explicit fixture fields or shell-assigned handles returned to the core. The core never discovers paths, ports, process ids, clocks, or environment variables. Any path or handle collision must deny before process start or before accepting pass evidence.

The initial scenario should cover one sender process and one receiver process running a status/control workflow through the admitted local path. Negative fixtures should mutate the ticket, peer, state root, or receipt set and prove the gate fails closed.
