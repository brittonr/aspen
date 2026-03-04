## 1. NixOS VM Test File Setup

- [ ] 1.1 Create `nix/tests/multihost-microvm-mesh.nix` with inputs: `pkgs`, `microvm`, `aspenNodePackage`, `aspenCliPackage`, `aspenNetPackage`
- [ ] 1.2 Define networking constants: host IPs (192.168.1.1, 192.168.1.2), TAP names/subnets per host, guest MACs
- [ ] 1.3 Build Guest A microVM (service): Cloud Hypervisor, TAP networking, python3 HTTP server, static IP 10.10.1.2/24
- [ ] 1.4 Build Guest B microVM (client): Cloud Hypervisor, TAP networking, curl installed, static IP 10.10.2.2/24
- [ ] 1.5 Configure `nodes.hostA`: nested KVM, 6GB RAM, 4 cores, aspen-node × 2, aspen-net daemon, socat, Guest A runner
- [ ] 1.6 Configure `nodes.hostB`: nested KVM, 4GB RAM, 2 cores, aspen-node × 1, aspen-net daemon, Guest B runner

## 2. Test Script: Bootstrap Raft Cluster Across Hosts (Phase 1)

- [ ] 2.1 Start aspen-node-1 on Host A (port 7001), wait for cluster ticket
- [ ] 2.2 Start aspen-node-2 on Host A (port 7002) with ticket
- [ ] 2.3 Start aspen-node-3 on Host B (port 7001) with ticket from Host A
- [ ] 2.4 Verify all 3 nodes active (check systemctl on both hosts)
- [ ] 2.5 Run `cluster init` from Host A

## 3. Test Script: Start Net Daemons on Both Hosts (Phase 2)

- [ ] 3.1 Start `aspen-net up` on Host A: SOCKS5 on 0.0.0.0:1080, `--no-dns`, with cluster ticket
- [ ] 3.2 Start `aspen-net up` on Host B: SOCKS5 on 0.0.0.0:1080, `--no-dns`, with cluster ticket
- [ ] 3.3 Wait for both SOCKS5 proxies listening (port 1080 on each host)
- [ ] 3.4 Verify both daemons connected to cluster (journal grep on each host)

## 4. Test Script: Launch Guest A on Host A (Phase 3)

- [ ] 4.1 Create TAP `vm-svc` on Host A, assign 10.10.1.1/24, bring up
- [ ] 4.2 Launch Guest A microVM on Host A via systemd-run
- [ ] 4.3 Wait for Guest A HTTP server: `hostA.succeed("curl -sf http://10.10.1.2:8080/index.html")` (timeout 180s)
- [ ] 4.4 Verify content: response contains "hello from multihost mesh"

## 5. Test Script: Publish Service from Host A (Phase 4)

- [ ] 5.1 Start socat bridge on Host A: `localhost:9080 → 10.10.1.2:8080`
- [ ] 5.2 Verify socat bridge on Host A
- [ ] 5.3 Extract Host A's endpoint ID from `cluster status`
- [ ] 5.4 Publish service: `aspen-cli net publish my-svc --endpoint-id {eid} --port 9080 --proto tcp`
- [ ] 5.5 Verify service listed on both hosts: `aspen-cli net services` from Host A and Host B

## 6. Test Script: Launch Guest B on Host B (Phase 5)

- [ ] 6.1 Create TAP `vm-client` on Host B, assign 10.10.2.1/24, bring up
- [ ] 6.2 Launch Guest B microVM on Host B via systemd-run
- [ ] 6.3 Wait for Guest B to boot: `hostB.succeed("ping -c1 10.10.2.2")` (timeout 120s)

## 7. Test Script: Cross-Host Mesh Routing (Phase 6)

- [ ] 7.1 Verify Host A can reach Guest A directly: `hostA.succeed("curl -sf http://10.10.1.2:8080/index.html")`
- [ ] 7.2 Verify Host A's local SOCKS5 works: `hostA.succeed("curl --socks5-hostname 127.0.0.1:1080 http://my-svc.aspen:9080/index.html")`
- [ ] 7.3 **Cross-host test**: From Host B, curl through Host B's SOCKS5 → iroh QUIC tunnel to Host A → Guest A: `hostB.succeed("curl --socks5-hostname 127.0.0.1:1080 http://my-svc.aspen:9080/index.html")`
- [ ] 7.4 Verify response contains "hello from multihost mesh" (content served by Guest A on Host A, accessed from Host B)
- [ ] 7.5 Verify `status.json` cross-host
- [ ] 7.6 Second request for stability
- [ ] 7.7 Log cross-host tunnel success with timing info

## 8. Test Script: Cleanup (Phase 7)

- [ ] 8.1 Stop Guest B on Host B, Guest A on Host A
- [ ] 8.2 Stop socat on Host A
- [ ] 8.3 Stop aspen-net daemons on both hosts
- [ ] 8.4 Stop aspen nodes (3 on Host B, then 2 and 1 on Host A)
- [ ] 8.5 Log phase summary

## 9. Flake Integration

- [ ] 9.1 Wire `nix/tests/multihost-microvm-mesh.nix` into `flake.nix` as `checks.x86_64-linux.multihost-microvm-mesh-test`
- [ ] 9.2 Pass required inputs: aspenNodePackage, aspenCliPackage, aspenNetPackage, microvm
- [ ] 9.3 Place in `hasExternalRepos` block (needs microvm flake input)
- [ ] 9.4 Verify `nix eval .#checks.x86_64-linux.multihost-microvm-mesh-test.name` evaluates
