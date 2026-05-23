## MODIFIED Requirements

### Requirement: VM-CI dogfood readiness distinguishes node health from VM pool bootstrap

`dogfood-local-vmci` SHALL NOT require golden-snapshot VM pool warmup to complete before the host Aspen node can pass its initial health check.

#### Scenario: Node health before VM pool warmup

- GIVEN `dogfood-local-vmci` starts a local Aspen node with VM-CI enabled
- AND VM pool warmup or golden-snapshot creation is slow or temporarily failing
- WHEN the dogfood runner performs the initial node health check
- THEN the node SHALL be allowed to report healthy based on core Aspen node readiness
- AND VM-CI worker readiness SHALL remain a separate readiness/evidence boundary

#### Scenario: VM pool bootstrap failure remains visible

- GIVEN VM-CI pool warmup fails after node startup
- WHEN the failure is logged or surfaced to dogfood evidence
- THEN the failure SHALL be classified as VM-CI worker readiness/provisioning evidence
- AND it SHALL NOT be reported as a base node-health failure unless the node itself cannot serve health RPCs

#### Scenario: Local direct health checks avoid external discovery

- GIVEN `dogfood-local-vmci` connects to the local node using a ticket that already contains direct socket addresses
- AND relay usage is disabled for the dogfood proof
- WHEN the dogfood runner performs initial health RPCs
- THEN the client SHALL use the ticket's direct addresses without DNS or relay address lookup
- AND external discovery latency SHALL NOT be part of the node-health success path

#### Scenario: VM worker guest ticket uses only the host bridge address

- GIVEN VM-CI provisions a worker guest with a host node ticket containing loopback, LAN, VPN, or other host-side direct addresses
- AND the host Iroh endpoint port is known for the CI bridge
- WHEN the ticket is written into the guest workspace for worker-only bootstrap
- THEN the guest ticket SHALL keep the bootstrap peer endpoint identity
- AND each bootstrap peer direct address set SHALL contain the host bridge socket address such as `10.200.0.1:<host_iroh_port>`
- AND the guest ticket SHALL NOT contain host loopback, IPv6 loopback, LAN, or VPN direct addresses

#### Scenario: Host setup admits guest Iroh UDP through the effective firewall

- GIVEN a NixOS host uses nftables firewall chains in addition to VM-CI compatibility NAT/filter chains
- WHEN `setup-ci-network` configures VM-CI bridge networking
- THEN the setup SHALL install idempotent bridge ingress rules in the effective NixOS firewall chain when present
- AND the VM-CI readiness marker SHALL distinguish this setup from older NAT-only or compatibility-chain-only markers

#### Scenario: Host client ticket behavior remains unfiltered

- GIVEN a same-host dogfood client uses the host-generated cluster ticket
- WHEN the client connects outside the VM guest
- THEN loopback-preferred direct addresses MAY remain available for same-host clients
- AND guest-only bridge filtering SHALL NOT be applied globally to host/client tickets
