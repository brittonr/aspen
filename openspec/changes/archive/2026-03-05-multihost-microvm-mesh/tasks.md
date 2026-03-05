## 1. NixOS VM Test File Setup

- [x] 1.1 Create `nix/tests/multihost-microvm-mesh.nix` with inputs: `pkgs`, `microvm`, `aspenNodePackage`, `aspenCliPackage`, `aspenNetPackage`
- [x] 1.2 Define networking constants: host IPs (auto-assigned by test driver), TAP names/subnets per host, guest MACs
- [x] 1.3 Build Guest A microVM (service): Cloud Hypervisor, TAP networking, python3 HTTP server, static IP 10.10.1.2/24
- [x] 1.4 Build Guest B microVM (client): Cloud Hypervisor, TAP networking, curl installed, static IP 10.10.2.2/24
- [x] 1.5 Configure `nodes.hostA`: nested KVM, 6GB RAM, 4 cores, aspen-node × 2, aspen-net daemon, socat, Guest A runner
- [x] 1.6 Configure `nodes.hostB`: nested KVM, 6GB RAM, 4 cores, aspen-node × 1, aspen-net daemon, Guest B runner

## 2. Test Script: Bootstrap Raft Cluster Across Hosts (Phase 1)

- [x] 2.1 Start aspen-node-1 on Host A (port 7001), wait for cluster ticket
- [x] 2.2 Start aspen-node-2 on Host A (port 7002) with ticket
- [x] 2.3 Start aspen-node-3 on Host B (port 7001) with ticket from Host A
- [x] 2.4 Verify all 3 nodes active (check systemctl on both hosts)
- [x] 2.5 Run `cluster init` from Host A

## 3. Test Script: Start Net Daemons on Both Hosts (Phase 2)

- [x] 3.1 Start `aspen-net up` on Host A: SOCKS5 on 0.0.0.0:1080, `--no-dns`, with cluster ticket
- [x] 3.2 Start `aspen-net up` on Host B: SOCKS5 on 0.0.0.0:1080, `--no-dns`, with cluster ticket
- [x] 3.3 Wait for both SOCKS5 proxies listening (port 1080 on each host)
- [x] 3.4 Verify both daemons connected to cluster (journal grep on each host)

## 4. Test Script: Launch Guest A on Host A (Phase 3)

- [x] 4.1 Create TAP `vm-svc` on Host A, assign 10.10.1.1/24, bring up
- [x] 4.2 Launch Guest A microVM on Host A via systemd-run
- [x] 4.3 Wait for Guest A HTTP server: `hostA.succeed("curl -sf http://10.10.1.2:8080/index.html")` (timeout 180s)
- [x] 4.4 Verify content: response contains "hello from multihost mesh"

## 5. Test Script: Publish Service from Host A (Phase 4)

- [x] 5.1 Start socat bridge on Host A: `localhost:9080 → 10.10.1.2:8080`
- [x] 5.2 Verify socat bridge on Host A
- [x] 5.3 Extract Host A's endpoint ID from `cluster status`
- [x] 5.4 Publish service: `aspen-cli net publish my-svc --endpoint-id {eid} --port 9080 --proto tcp`
- [x] 5.5 Verify service listed on both hosts: `aspen-cli net services` from Host A and Host B

## 6. Test Script: Launch Guest B on Host B (Phase 5)

- [x] 6.1 Create TAP `vm-client` on Host B, assign 10.10.2.1/24, bring up
- [x] 6.2 Launch Guest B microVM on Host B via systemd-run
- [x] 6.3 Wait for Guest B to boot: `hostB.succeed("ping -c1 10.10.2.2")` (timeout 120s)

## 7. Test Script: Cross-Host Mesh Routing (Phase 6)

- [x] 7.1 Verify Host A can reach Guest A directly: `hostA.succeed("curl -sf http://10.10.1.2:8080/index.html")`
- [x] 7.2 Verify Host A's local SOCKS5 works: `hostA.succeed("curl --socks5-hostname 127.0.0.1:1080 http://my-svc.aspen:9080/index.html")`
- [x] 7.3 **Cross-host test**: From Host B, curl through Host B's SOCKS5 → iroh QUIC tunnel to Host A → Guest A: `hostB.succeed("curl --socks5-hostname 127.0.0.1:1080 http://my-svc.aspen:9080/index.html")`
- [x] 7.4 Verify response contains "hello from multihost mesh" (content served by Guest A on Host A, accessed from Host B)
- [x] 7.5 Verify `status.json` cross-host
- [x] 7.6 Second request for stability
- [x] 7.7 Log cross-host tunnel success with timing info

## 8. Test Script: Cleanup (Phase 7)

- [x] 8.1 Stop Guest B on Host B, Guest A on Host A
- [x] 8.2 Stop socat on Host A
- [x] 8.3 Stop aspen-net daemons on both hosts
- [x] 8.4 Stop aspen nodes (3 on Host B, then 2 and 1 on Host A)
- [x] 8.5 Log phase summary

## 9. Flake Integration

- [x] 9.1 Wire `nix/tests/multihost-microvm-mesh.nix` into `flake.nix` as `checks.x86_64-linux.multihost-microvm-mesh-test`
- [x] 9.2 Pass required inputs: aspenNodePackage, aspenCliPackage, aspenNetPackage, microvm
- [x] 9.3 Place in `hasExternalRepos` block (needs microvm flake input)
- [x] 9.4 Verify `nix eval .#checks.x86_64-linux.multihost-microvm-mesh-test.name` evaluates
