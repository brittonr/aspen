## 1. NixOS VM Test File Setup

- [x] 1.1 Create `nix/tests/microvm-virtiofs-net.nix` with inputs: `pkgs`, `microvm`, `aspenNodePackage`, `aspenCliPackage`, `aspenNetPackage`, `aspenClusterVirtiofsServer`
- [x] 1.2 Define networking constants: TAP names (`vm-svc`, `vm-client`), subnets (10.10.1.0/24, 10.10.2.0/24), guest MACs, VirtioFS socket path
- [x] 1.3 Build Guest A microVM: Cloud Hypervisor with VirtioFS share (`/var/www/aspen`) + TAP networking, nginx serving from VirtioFS mount, 512MB RAM
- [x] 1.4 Build Guest B microVM: Cloud Hypervisor with TAP networking only, curl installed, 512MB RAM
- [x] 1.5 Configure QEMU host: nested KVM, 8GB RAM, 4 cores, all required packages (both aspen daemons, cloud-hypervisor, curl, socat, iproute2)

## 2. Test Script: Raft Cluster Bootstrap (Phase 1)

- [x] 2.1 Start 3 aspen-node processes via systemd-run (inmemory backend, relay disabled, no gossip/mdns, ports 7001-7003)
- [x] 2.2 Wait for node 1 cluster ticket, start nodes 2+3 with ticket
- [x] 2.3 Verify all 3 nodes active, run `cluster init`

## 3. Test Script: Seed KV Data (Phase 2)

- [x] 3.1 Write `index.html` content to Raft KV via `aspen-cli kv set /www/index.html "hello from virtiofs-net mesh"`
- [x] 3.2 Write `status.json` to Raft KV via `aspen-cli kv set /www/status.json '{"source":"virtiofs-net","ok":true}'`
- [x] 3.3 Verify data readable via `aspen-cli kv get /www/index.html`

## 4. Test Script: Start VirtioFS Daemon (Phase 3)

- [x] 4.1 Start `aspen-cluster-virtiofs-server` via systemd-run, connected to cluster ticket, socket at `/tmp/aspenfs.sock`, mount prefix `/www`
- [x] 4.2 Wait for VirtioFS socket to appear: `test -S /tmp/aspenfs.sock`

## 5. Test Script: Start Net Daemon (Phase 4)

- [x] 5.1 Start `aspen-net up` via systemd-run with cluster ticket, SOCKS5 on `0.0.0.0:1080`, `--no-dns`
- [x] 5.2 Wait for SOCKS5 port 1080 listening
- [x] 5.3 Verify daemon connected to cluster (journal grep)

## 6. Test Script: Launch Guest A with VirtioFS + Network (Phase 5)

- [x] 6.1 Create TAP `vm-svc`, assign 10.10.1.1/24, bring up
- [x] 6.2 Launch Guest A microVM via systemd-run (CH with VirtioFS share + TAP)
- [x] 6.3 Wait for Guest A nginx to respond: `curl -sf http://10.10.1.2:80/` (timeout 180s for VirtioFS mount + nginx start)
- [x] 6.4 Verify content comes from KV: response contains `"hello from virtiofs-net mesh"`

## 7. Test Script: Publish Service (Phase 6)

- [x] 7.1 Start socat bridge: `localhost:9080 → 10.10.1.2:80`
- [x] 7.2 Verify socat bridge works: `curl -sf http://127.0.0.1:9080/`
- [x] 7.3 Extract endpoint ID from `cluster status`
- [x] 7.4 Publish service: `aspen-cli net publish web-svc --endpoint-id {eid} --port 9080 --proto tcp`
- [x] 7.5 Verify service listed: `aspen-cli net services` shows `web-svc`

## 8. Test Script: Launch Guest B and Route Through Mesh (Phase 7)

- [x] 8.1 Create TAP `vm-client`, assign 10.10.2.1/24, bring up
- [x] 8.2 Launch Guest B microVM via systemd-run
- [x] 8.3 Wait for Guest B to boot: `ping -c1 10.10.2.2`
- [x] 8.4 Curl through SOCKS5: `curl --socks5-hostname 127.0.0.1:1080 http://web-svc.aspen:9080/index.html`
- [x] 8.5 Verify response contains `"hello from virtiofs-net mesh"` (content originated from Raft KV → VirtioFS → nginx → mesh)
- [x] 8.6 Verify `status.json` through mesh: response contains `"virtiofs-net"`
- [x] 8.7 Second request for connection stability

## 9. Test Script: Cleanup and Summary (Phase 8)

- [x] 9.1 Stop Guest B, Guest A, socat, aspen-net daemon, VirtioFS daemon, Raft nodes
- [x] 9.2 Log phase summary

## 10. Flake Integration

- [x] 10.1 Wire `nix/tests/microvm-virtiofs-net.nix` into `flake.nix` as `checks.x86_64-linux.microvm-virtiofs-net-test`
- [x] 10.2 Pass all required inputs: aspenNodePackage, aspenCliPackage, aspenNetPackage, aspenClusterVirtiofsServer, microvm
- [x] 10.3 Place in `hasExternalRepos` block (needs microvm flake input)
- [x] 10.4 Verify `nix eval .#checks.x86_64-linux.microvm-virtiofs-net-test.name` evaluates
