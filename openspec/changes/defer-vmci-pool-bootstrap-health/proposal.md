## Why

Live `dogfood-local-vmci` now passes the TAP-helper permission boundary but can still fail before node health because VM pool warmup/golden-snapshot creation starts during node bootstrap. The observed run timed out the host health check while the background VM path was trying to provision AspenFs workspace state. After that boundary was fixed, the next live VM-CI run showed relay-disabled VM workers timing out because the guest received host-scoped direct addresses such as loopback, LAN, and VPN addresses that are invalid from inside the VM. Once guest tickets were scoped to the bridge address, runtime evidence showed L3 bridge connectivity but continued guest->host Iroh/QUIC timeout, consistent with a host nftables/NixOS firewall chain dropping bridge UDP after a compatibility accept chain.

## What Changes

- Defer VM-CI pool warmup so `aspen-node` can report initial health and complete cluster initialization before booting golden-snapshot VMs.
- Keep VM-CI readiness/failures visible as worker-pool readiness evidence instead of making the base node health endpoint unreachable.
- Scope the cluster ticket written into VM guest workspaces to the host bridge direct address only, so relay-disabled workers do not try loopback/LAN/VPN addresses that are invalid from inside the guest.
- Require the VM-CI network setup marker produced after installing NixOS firewall-chain bridge ingress rules, not just NAT/compatibility-chain setup.
- Preserve TAP-helper mode and the existing VM worker architecture.

## Capabilities

### Modified Capabilities
- `dogfood-evidence`: VM-CI dogfood startup distinguishes node health from VM pool bootstrap readiness and scopes guest worker bootstrap tickets to VM-routable direct addresses.

## Impact

- **Files**: `crates/aspen-ci-executor-vm/src/worker.rs`, `crates/aspen-ci-executor-vm/src/vm/lifecycle.rs`, `crates/aspen-dogfood/src/vmci_readiness.rs`, `scripts/setup-ci-network.sh`, `scripts/dogfood-local-vmci.sh`, VM-CI dogfood/readiness docs/specs.
- **APIs**: no public API break; worker startup semantics become non-blocking for VM prewarm; guest ticket filtering is local to VM workspace provisioning; VM-CI host setup marker advances when firewall ingress semantics change.
- **Testing**: focused VM executor tests including guest ticket address scoping, OpenSpec validation, and a live `dogfood-local-vmci` retry on a host with the TAP helper installed.
