# Change-local acceptance

a[pin-octet-dev-shell.pinned-tool] `nix develop -c cargo octet` resolves to the `octet-toolchain` `fc38f593` build of `cargo-octet`, not an ambient install.
a[pin-octet-dev-shell.gate-evidence] The README strict sequence runs end to end with the dev shell tool, and the gate records config and profile hashes as current.
a[pin-octet-dev-shell.truthful-readme] README:741 states the measured pinned-tool counts, the gate deny, the burn-down series, and deferred enforcement, with no clean claim.
