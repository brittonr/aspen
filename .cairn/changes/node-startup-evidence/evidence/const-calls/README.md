# VM-only continuation: const-call repair and missing tools

The user requested VM-only continuation. No physical deployment or startup bypass was attempted.

## Tool repair

Octet published `fix/const-call-eligibility` at `f42bc855d496db43cee49dbe7198f98f2bdd09ef`.
Implementation: `1e7b64f9c1a56052a51dc98e7538cbcb8531d15a`.
Evidence and replay helper: `verification/const-calls/` in that repository.

The repair excludes trait declarations/implementations and resolved non-const direct/method calls from const suggestions.
It preserves constructor handling, inherent-method candidates, calls to const methods, and the previous formatting controls.
This is a negative eligibility check, not a complete proof of const-feature, trait-bound, indirect-call, or destructor legality.
No Molten production code or startup policy changed.

The audited plan built one lint-library derivation, offline, with no substitutes or remote builders.
The existing compiler and repaired driver were reused. No Stage0, compiler, CLI, runtime, Mantle, or Darkhttpd build occurred.
Seven owner tests passed, followed by ten retained controls and nine new paired controls.
The preceding library fails the new Display control; the new library passes.

```text
Library: /nix/store/40f34yq9qpxmf4k3cdlcj24fj14gjh9f-octet-0.1.0/lib/liboctet.so
BLAKE3: 03e24c4900d0f403de955bd30786b17847e7aef9377694ea81fb27ba23e510e2
Derivation: /nix/store/h5d4z9iqxjfw0dzwp84igba8ybj5qqpq-octet-0.1.0.drv
```

This is the focused check override output, not the default package output.
Nix content verification passed with `--no-trust`; a private GC root retains the library.
Full Octet UI/workspace/flake/Clippy acceptance and approved-cohort status are not established.

## Full-command attempt blocked before execution

Task 10323 attempted frozen Molten source `ab3efbe9788643caddd7f22bb0a13eca0d90426f` in `lifecycle-source-gate-11`.
The archive matched run 10. The identity command then failed because these retained tools are missing:

- CLI: `/nix/store/c14wxbczlzph9l1j5sfy6d4nh1ghdcxz-cargo-octet-0.1.0/bin/cargo-octet`
- Hook: `/nix/store/pp4l3in8gypmsgrnav4f4z6vq168hzpg-source/hooks/octet-deny-all.sh`
- Runtime wrapper, checked separately: `/nix/store/arsdclqr5dk7qd51ap89p50mvq4pc8bz-cargo-octet-runtime-path/bin/cargo`

The wrapper exited 1 before invoking the CLI. No new source-gate status or finding count exists.
The failure took 425ms; invocation `641139f5ecf54563a3922c695d3f3972`.
This does not establish when or why the paths disappeared.

Bounded discovery found nine other CLI executables. None matches the expected CLI BLAKE3 `dbf4b36ceacdcc8d372a48498ae3cb513692a22d4bff30774db20fb4e9295f4f`.
The ambient CLI was not substituted.
A hook extracted from pinned source matches expected BLAKE3 `41603febce44635585c6010557f6991008976b7e2f7e417c9c78f5299690522f`.
That hook alone does not restore the missing CLI/runtime pair.
No replacement CLI or runtime build was started.

## Next boundary

Restore the exact missing tools or audit an explicit replacement build with the existing compiler.
Then rerun the complete-source diagnostic with the repaired library and unchanged production flags.
The last completed full check still reports 36 findings. The direct controls do not justify claiming 33.

No VM was launched. Startup admission, production source-to-runtime binding, normal-node VM proof, and native replay remain blocked.
The VM-only restriction, existing pins, startup guards, and lifecycle tasks 4–5 remain unchanged.
