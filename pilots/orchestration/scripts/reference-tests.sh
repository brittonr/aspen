#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail
# Run in a fully configured Aspen checkout after applying the guarded patch.
# Unlike the standalone workspace, these retain Aspen's private/Nix/Radicle deps.
cd "$(dirname "$0")/../../.."
[[ -f crates/molten-core/tests/scheduler_lease_fencing.rs ]] || { echo 'Apply the bundle to the pinned Aspen checkout first.' >&2; exit 1; }
cargo test -p molten-core --test scheduler_lease_fencing
cargo test -p molten-core fabric_simulation
cargo test -p molten fabric_simulation
