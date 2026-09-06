# Producer evidence checkpoint — 2026-09-06

## Source and scope

Implementation: `b45625f9bbd4536c672e6d61251a31dfb48e4ea7`, based on current Molten `bb6f3830ee7327da9875ea85a8c8e25697eddc35`.

Actual tested snapshot: `3b5b79132e9b875c6c72c70ed7f38cfdd4e6f11a`, based on reviewed release `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0`.

The selected source files are byte-identical between the tested snapshot and implementation. The release manifest and lockfile are unchanged. This does not establish acceptance of the full current workspace.

## Observed results

The content core baseline passed 6 tests. The modified core passed 8 tests. The example passed 2 no-network tests. Core Clippy passed with `-D warnings` for all targets.

The real storage/client VM run passed exact transfer, server-side reader denial, wrong archive identity, clean storage restart, and damaged-store publication rejection. Its final duration was 31.544 seconds. `transfer.json` and `public-observations.json` retain these facts without private keys or locator handoffs.

Onix ran the retrieved archive through fresh Mantle import and native admission. Both Darkhttpd VM cases passed. That consumer evidence belongs to Onix, not Molten.

Consumer implementation: `91dd67969cce227b8806a811d47edd51e3813adc`.
Consumer evidence: `e2ef31cc399c9eb9e8e06e72868ffb8b6ab0a344` in `https://github.com/onixcomputer/onix-modules`, under `.cairn/changes/molten-blob-vm-transfer/evidence/`.

## Remaining blockers

The current workspace cannot resolve an unrelated optional Radicle input in offline mode. Selected-example Clippy stops in unchanged release upgrade code. Installed Cairn rejects the existing Molten policy/schema combination.

No dependency pin, policy requirement, or lint was suppressed. Full current-workspace tests, Octet enforcement, and flake checks remain unverified. The change remains active and is not release-admitted.

No host blob service, physical deployment, Stage0, Darkhttpd rebuild, Mantle rebuild, or toolchain rebuild occurred. Private guest disks and keys remain outside Git. Pin readback and clean restart do not prove garbage collection, crash durability, or general retention behavior.
