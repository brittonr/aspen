# Implementation tasks

- [x] [serial] Author versioned canonical manifest, complete assignments and frozen scene. r[molten.batch_render.identity]
- [x] [serial] Implement original bounded renderer, checked tile/final envelopes and strict assembly. r[molten.batch_render.assembly]
- [x] [serial] Bind guest ABI and job construction to existing batch model/codec. r[molten.batch_render.identity] r[molten.batch_render.assembly]
- [x] [serial] Author executable determinism, identity, malformed coverage/substitution and resource-bound tests. r[molten.batch_render.identity] r[molten.batch_render.assembly] r[molten.batch_render.evidence]
- [x] [serial] Author build and isolated separate-process scripts, integration documentation and evidence caveats. r[molten.batch_render.evidence]
- [x] [serial] Owner runs focused tests, component build and process fixture; record actual outcomes separately. r[molten.batch_render.evidence]

## Parent execution (2026-11-13)

The pinned compiler and checksum-verified `wasm32-unknown-unknown` standard
library built the real guest; `wasm-tools component new` emitted a component
whose WIT contains only the `run` export and no imports. The native renderer's
eight conformance tests passed. The first real CLI process suite with that guest
passed eight of nine cases; its separate progress-retention failure remains
retained. After the progress storage correction, the unchanged retention case
passed in 243.15 seconds, and the uninstrumented progress suite passed all six
non-renderer cases. The renderer's three explicit cases remain bound to their
earlier actual guest execution, not the later suite's ignored inventory. These
are same-host observations, not cross-host, machine-loss or release qualification.

