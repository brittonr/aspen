## 1. NixOS VM Test File Setup

- [ ] 1.1 Create `nix/tests/microvm-virtiofs-net.nix` with inputs: `pkgs`, `microvm`, `aspenNodePackage`, `aspenCliPackage`, `aspenNetPackage`, `aspenVirtiofsServer`
- [ ] 1.2 Define networking constants: TAP names (`vm-svc`, `vm-client`), subnets (10.10.1.0/24, 10.10.2.0/24), guest MACs, VirtioFS socket path
- [ ] 1.3 Build Guest A microVM: Cloud Hypervisor with VirtioFS share (`/var/www/aspen`) + TAP networking, nginx serving from VirtioFS mount, 512MB RAM
- [ ] 1.4 Build Guest B microVM: Cloud Hypervisor with TAP networking only, curl installed, 512MB RAM
- [ ] 1.5 Configure QEMU host: nested KVM, 8GB RAM, 4 cores, all required packages (both aspen daemons, cloud-hypervisor, curl, socat, iproute2)

## 2. Test Script: Raft Cluster Bootstrap (Phase 1)

- [ ] 2.1 Start 3 aspen-node processes via systemd-run (inmemory backend, relay disabled, no gossip/mdns, ports 7001-7003)
- [ ] 2.2 Wait for node 1 cluster ticket, start nodes 2+3 with ticket
- [ ] 2.3 Verify all 3 nodes active, run `cluster init`

## 3. Test Script: Seed KV Data (Phase 2)

- [ ] 3.1 Write `index.html` content to Raft KV via `aspen-cli kv set /www/index.html "hello from virtiofs-net mesh"`
- [ ] 3.2 Write `status.json` to Raft KV via `aspen-cli kv set /www/status.json '{"source":"virtiofs-net","ok":true}'`
- [ ] 3.3 Verify data readable via `aspen-cli kv get /www/index.html`

## 4. Test Script: Start VirtioFS Daemon (Phase 3)

- [ ] 4.1 Start `aspen-cluster-virtiofs-server` via systemd-run, connected to cluster ticket, socket at `/tmp/aspenfs.sock`, mount prefix `/www`
- [ ] 4.2 Wait for VirtioFS socket to appear: `test -S /tmp/aspenfs.sock`

## 5. Test Script: Start Net Daemon (Phase 4)

- [ ] 5.1 Start `aspen-net up` via systemd-run with cluster ticket, SOCKS5 on `0.0.0.0:1080`, `--no-dns`
- [ ] 5.2 Wait for SOCKS5 port 1080 listening
- [ ] 5.3 Verify daemon connected to cluster (journal grep)

## 6. Test Script: Launch Guest A with VirtioFS + Network (Phase 5)

- [ ] 6.1 Create TAP `vm-svc`, assign 10.10.1.1/24, bring up
- [ ] 6.2 Launch Guest A microVM via systemd-run (CH with VirtioFS share + TAP)
- [ ] 6.3 Wait for Guest A nginx to respond: `curl -sf http://10.10.1.2:8080/` (timeout 180s for VirtioFS mount + nginx start)
- [ ] 6.4 Verify content comes from KV: response contains `"hello from virtiofs-net mesh"`

## 7. Test Script: Publish Service (Phase 6)

- [ ] 7.1 Start socat bridge: `localhost:9080 → 10.10.1.2:8080`
- [ ] 7.2 Verify socat bridge works: `curl -sf http://127.0.0.1:9080/`
- [ ] 7.3 Extract endpoint ID from `cluster status`
- [ ] 7.4 Publish service: `aspen-cli net publish web-svc --endpoint-id {eid} --port 9080 --proto tcp`
- [ ] 7.5 Verify service listed: `aspen-cli net services` shows `web-svc`

## 8. Test Script: Launch Guest B and Route Through Mesh (Phase 7)

- [ ] 8.1 Create TAP `vm-client`, assign 10.10.2.1/24, bring up
- [ ] 8.2 Launch Guest B microVM via systemd-run
- [ ] 8.3 Wait for Guest B to boot: `ping -c1 10.10.2.2`
- [ ] 8.4 Curl through SOCKS5: `curl --socks5-hostname 127.0.0.1:1080 http://web-svc.aspen:9080/index.html`
- [ ] 8.5 Verify response contains `"hello from virtiofs-net mesh"` (content originated from Raft KV → VirtioFS → nginx → mesh)
- [ ] 8.6 Verify `status.json` through mesh: response contains `"virtiofs-net"`
- [ ] 8.7 Second request for connection stability

## 9. Test Script: Cleanup and Summary (Phase 8)

- [ ] 9.1 Stop Guest B, Guest A, socat, aspen-net daemon, VirtioFS daemon, Raft nodes
- [ ] 9.2 Log phase summary

## 10. Flake Integration

- [ ] 10.1 Wire `nix/tests/microvm-virtiofs-net.nix` into `flake.nix` as `checks.x86_64-linux.microvm-virtiofs-net-test`
- [ ] 10.2 Pass all required inputs: aspenNodePackage, aspenCliPackage, aspenNetPackage, aspenVirtiofsServer, microvm
- [ ] 10.3 Place in `hasExternalRepos` block (needs microvm flake input)
- [ ] 10.4 Verify `nix eval .#checks.x86_64-linux.microvm-virtiofs-net-test.name` evaluates
