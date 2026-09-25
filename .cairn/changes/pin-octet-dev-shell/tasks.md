# Tasks: Pin the Octet toolchain in the dev shell

## Phase 1: Implementation

- [x] [serial] Add the pinned `cargo-octet` to `devShells.default`. a[pin-octet-dev-shell.pinned-tool]
- [x] [serial] Rewrite README:741 to the measured Octet state without a clean claim. a[pin-octet-dev-shell.truthful-readme]

## Phase 2: Validation

- [x] [serial] Positive: `command -v cargo-octet` in the dev shell resolves to `/nix/store/n8bkxc9p…-cargo-octet-0.1.0`. a[pin-octet-dev-shell.pinned-tool]
- [x] [serial] Run the full strict sequence with the dev shell tool and record receipts and counts. a[pin-octet-dev-shell.gate-evidence]
- [x] [serial] Negative: the strict gate still denies `warning-only`, and README repeats only numbers from the recorded run. a[pin-octet-dev-shell.truthful-readme]
