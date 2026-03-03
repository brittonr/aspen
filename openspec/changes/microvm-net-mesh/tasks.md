## 1. NixOS VM Test File

- [ ] 1.1 Create `nix/tests/microvm-net-mesh.nix` with inputs: `pkgs`, `microvm`, `aspen-node-vm-test`, `aspen-cluster-virtiofs-server`, `aspenCliPackage`, `aspenNetPackage`, `kvPluginWasm`, `aspenCliPlugins`
- [ ] 1.2 Define networking constants: `svcGuestIp = "10.10.1.2"`, `svcHostIp = "10.10.1.1"`, `clientGuestIp = "10.10.2.2"`, `clientHostIp = "10.10.2.1"`, `svcTap = "vm-svc"`, `clientTap = "vm-client"`, guest MACs
- [ ] 1.3 Build guest A microVM (service): Cloud Hypervisor, TAP networking, python3 + HTTP server in `systemd.services`, static IP `10.10.1.2/24`, gateway `10.10.1.1`
- [ ] 1.4 Build guest B microVM (client): Cloud Hypervisor, TAP networking, curl installed, static IP `10.10.2.2/24`, gateway `10.10.2.1`
- [ ] 1.5 Configure host node: nested KVM, 8GB RAM, 4 cores, all required packages (aspen-node, aspen-cli, aspen-net, cloud-hypervisor, curl, iproute2, socat, python3)

## 2. Test Script: Raft Cluster Bootstrap (Phase 1)

- [ ] 2.1 Start 3 aspen-node processes via systemd-run (same pattern as microvm-raft-virtiofs: inmemory backend, relay disabled, no gossip/mdns, ports 7001-7003)
- [ ] 2.2 Wait for node 1 cluster ticket, start nodes 2+3 with ticket
- [ ] 2.3 Wait for all nodes to join, verify all 3 active
- [ ] 2.4 Run `cluster init` via CLI, install KV plugin (needed for net registry)

## 3. Test Script: Net Daemon (Phase 2)

- [ ] 3.1 Start aspen-net daemon via systemd-run: `aspen-net up --ticket {ticket} --socks5-port 1080 --no-dns`
- [ ] 3.2 Wait for SOCKS5 port 1080 to be listening (`ss -tlnp | grep :1080`)
- [ ] 3.3 Verify daemon connected to cluster (check journal for "connected to cluster")

## 4. Test Script: Guest A — Service VM (Phase 3)

- [ ] 4.1 Create TAP `vm-svc`, assign `10.10.1.1/24`, bring up
- [ ] 4.2 Launch guest A microVM via systemd-run
- [ ] 4.3 Wait for guest A's HTTP server to respond: `curl -sf http://10.10.1.2:8080/` from host (timeout 120s for guest boot)
- [ ] 4.4 Seed test content: write `index.html` to guest A's `/tmp/www/` via the guest's startup script (baked into the NixOS config)

## 5. Test Script: Publish Service (Phase 4)

- [ ] 5.1 Get the aspen-node endpoint ID from cluster status via CLI
- [ ] 5.2 Start socat port-forward on host: `socat TCP-LISTEN:9080,fork,reuseaddr TCP:10.10.1.2:8080 &` (bridges localhost → guest A)
- [ ] 5.3 Verify socat forward works: `curl -sf http://127.0.0.1:9080/index.html` from host
- [ ] 5.4 Publish service: `aspen-cli net publish my-svc --endpoint-id {eid} --port 9080 --proto tcp`
- [ ] 5.5 Verify service listed: `aspen-cli net services` shows `my-svc`

## 6. Test Script: Guest B — Client VM (Phase 5)

- [ ] 6.1 Create TAP `vm-client`, assign `10.10.2.1/24`, bring up
- [ ] 6.2 Enable IP forwarding on host for inter-TAP routing (guest B needs to reach host SOCKS5)
- [ ] 6.3 Launch guest B microVM via systemd-run
- [ ] 6.4 Wait for guest B to boot: `ping -c1 10.10.2.2` from host (timeout 120s)

## 7. Test Script: Route Through Mesh (Phase 6)

- [ ] 7.1 From guest B, curl through host SOCKS5: `curl --socks5-hostname 10.10.2.1:1080 http://my-svc.aspen:9080/index.html`
- [ ] 7.2 Verify response contains expected content from guest A
- [ ] 7.3 Test a second request to verify connection reuse / stability

## 8. Test Script: Cleanup (Phase 7)

- [ ] 8.1 Stop guest B microVM
- [ ] 8.2 Stop guest A microVM
- [ ] 8.3 Kill socat forwarder
- [ ] 8.4 Stop aspen-net daemon
- [ ] 8.5 Stop aspen nodes 3, 2, 1
- [ ] 8.6 Log phase summary

## 9. Flake Integration

- [ ] 9.1 Wire `nix/tests/microvm-net-mesh.nix` into flake.nix as `checks.x86_64-linux.microvm-net-mesh-test`
- [ ] 9.2 Pass required inputs: `aspen-node-vm-test`, `aspenCliPackage`, `aspenNetPackage`, `kvPluginWasm`, `aspenCliPlugins`, `microvm`
- [ ] 9.3 Verify `nix build .#checks.x86_64-linux.microvm-net-mesh-test --impure` compiles the test derivation
