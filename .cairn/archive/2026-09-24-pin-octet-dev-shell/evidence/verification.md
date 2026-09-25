# Verification: Pin the Octet toolchain in the dev shell

Base: `origin/molten` `4e31cee55167a38978961faac5c46476aca6f5ac`.

- `nix develop -c bash -c 'command -v cargo-octet; readlink -f $(command -v cargo-octet)'` →
  `/nix/store/n8bkxc9p155a27xnx0iqm3l694s8ckkp-cargo-octet-0.1.0/bin/cargo-octet`. Before this change, the shell
  resolved the ambient `/etc/profiles/per-user/brittonr/bin/cargo-octet` (home-manager build `x4a5p5vn…`).
- Strict sequence in the dev shell with private `CARGO_TARGET_DIR`:
  - `cargo octet check`: exit 0, `warning-only`, 3627 findings (2016 distinct sites), config hash
    `b3:ac7488df52080e0a59d56aa0517f7222cf649ad2ea9053163c8d7ca2583b78c3`.
  - `cargo octet check -p molten -- --lib`: exit 0, `warning-only`, 1515 findings.
  - Object corpus receipt: exit 0.
  - `molten test octet artifacts import`: pass, `blake3:8de5fb08d7f7e01dea0eb0cf7e0e2b23d001e9cc34c883bf3673dfa878db4ea0`.
  - `molten test octet gate --profile strict-ci`: exit 1, deny,
    `blake3:2613b8c9a388deacb780c9ea57bf3751edc1ca106279f302a4a4b5a591e3014f`. `status-config-current` and
    `status-profile-current` pass. `strict-status-clean` fails ("strict profile denies octet status `warning-only`
    with 3627 findings"), and `no-critical-findings` fails ("unreviewed critical octet findings: 312").
  - `molten test octet remediation plan`: `blake3:e42a9adea610a11f1d66129c1e4ae6de1598c8bc2ab453d3588fc9bfe8dbb99e`.
- README:741 numbers are copied from these summaries.
