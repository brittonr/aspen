## 1. Implementation

- [ ] I1 Add a deterministic drain completion audit for active paths, archive path, and `.drain-state.md` absence. [covers=openspec-governance.drain-archive-cleanliness.clean-drain-passes,openspec-governance.drain-archive-cleanliness.leftover-active-path-fails]
- [ ] I2 Add archive-path consistency checks for archived verification/task coverage files. [covers=openspec-governance.drain-archive-cleanliness.archive-paths-used]
- [ ] I3 Update drain skill instructions and evidence template to require the post-archive audit transcript. [covers=openspec-governance.drain-archive-cleanliness]

## 2. Verification

- [ ] V1 Add positive clean-drain and negative leftover-active fixtures. [covers=openspec-governance.drain-archive-cleanliness]
- [ ] V2 Run the audit fixtures and save transcripts. [covers=openspec-governance.drain-archive-cleanliness]
- [ ] V3 Run `openspec validate enforce-drain-archive-cleanliness --type change --strict --no-interactive` and the relevant preflight after implementation; save transcripts. [covers=openspec-governance.drain-archive-cleanliness]
