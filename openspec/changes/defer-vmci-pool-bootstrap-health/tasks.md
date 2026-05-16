## Phase 1: Spec and implementation

- [x] [serial] Capture VM-CI node-health vs pool-bootstrap requirement in OpenSpec.
- [x] [serial] Make CloudHypervisorWorker startup non-blocking for VM pool warmup.
- [x] [serial] Add focused regression coverage for non-blocking worker startup.
- [x] [serial] Scope VM guest workspace tickets to the host bridge direct address and cover loopback/LAN/VPN filtering.
- [x] [serial] Preserve relay-disabled worker endpoint policy for VM guests.
- [x] [serial] Install NixOS firewall-chain bridge ingress rules so guest->host Iroh/QUIC UDP is not dropped after a compatibility accept chain.

## Phase 2: Verification and landing

- [x] [depends:implementation] Run focused VM executor tests and formatting checks.
- [x] [depends:verification] Run OpenSpec validation and live VM-CI dogfood retry.
- [ ] [depends:dogfood] Archive the OpenSpec, commit, push, and report evidence.
