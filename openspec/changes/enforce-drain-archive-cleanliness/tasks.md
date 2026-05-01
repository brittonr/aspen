## 1. Implementation

- [x] I1 Add a deterministic drain completion audit for active paths, archive path, and `.drain-state.md` absence. [covers=openspec-governance.drain-archive-cleanliness.clean-drain-passes,openspec-governance.drain-archive-cleanliness.leftover-active-path-fails] ✅ 20m (started: 2026-05-01T01:45:00Z → completed: 2026-05-01T02:05:33Z)
- [x] I2 Add archive-path consistency checks for archived verification/task coverage files. [covers=openspec-governance.drain-archive-cleanliness.archive-paths-used] ✅ 15m (started: 2026-05-01T01:53:00Z → completed: 2026-05-01T02:08:10Z)
- [ ] I3 Update drain skill instructions and evidence template to require the post-archive audit transcript. [covers=openspec-governance.drain-archive-cleanliness]

## 2. Verification

- [x] V1 Add positive clean-drain and negative leftover-active fixtures. [covers=openspec-governance.drain-archive-cleanliness] ✅ 20m (started: 2026-05-01T01:45:00Z → completed: 2026-05-01T02:05:33Z)
- [x] V2 Run the audit fixtures and save transcripts. [covers=openspec-governance.drain-archive-cleanliness] ✅ 20m (started: 2026-05-01T01:45:00Z → completed: 2026-05-01T02:05:33Z)
- [ ] V3 Run `openspec validate enforce-drain-archive-cleanliness --type change --strict --no-interactive` and the relevant preflight after implementation; save transcripts. [covers=openspec-governance.drain-archive-cleanliness]
