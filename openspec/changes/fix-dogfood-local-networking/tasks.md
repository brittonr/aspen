# Tasks

## 1. Fix dogfood node discovery via ClusterDiscovery

- [x] 1.1 In `node.rs`, pass `--data-dir` parent as cluster discovery dir so alice/bob discover each other's addresses via filesystem
- [x] 1.2 Ensure nodes wait for their own endpoint address to be populated before writing the ticket file
- [x] 1.3 Test: `nix run .#dogfood-federation -- start` succeeds and `status` shows both nodes healthy

## 2. Fix AspenClient local connectivity

- [x] 2.1 Parse the ticket's bootstrap peer addresses and use them as direct connection targets
- [x] 2.2 If no direct addresses in ticket, read cluster discovery files from the dogfood data dir
- [x] 2.3 Test: `nix run .#dogfood-federation -- start && nix run .#dogfood-federation -- status` returns healthy for both

## 3. Expand NixOS VM federation-git-clone test

- [x] 3.1 Add subtest that creates 100+ files in nested dirs (src/{a..z}/{1..4}.txt, docs/{a..j}.md, etc.)
- [x] 3.2 Push to alice via git-remote-aspen
- [x] 3.3 Federate and clone on bob
- [x] 3.4 Verify file count matches and spot-check 5+ file contents
- [ ] 3.5 Run: `nix build .#checks.x86_64-linux.federation-git-clone-test`

## 4. Run full dogfood-federation pipeline

- [ ] 4.1 After fixing connectivity: `nix run .#dogfood-federation -- full`
- [ ] 4.2 Verify git push of Aspen workspace (34K objects) completes
- [ ] 4.3 Verify CI pipeline triggers on bob
- [ ] 4.4 Check alice.log and bob.log for federation sync stats and DAG integrity
