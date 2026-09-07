# Nested constant arithmetic: confirmed false positive

The full gate still reports 33 node-host findings. This review does not reduce that count.
One finding targets `MAX_NODE_STATE_FILE_BYTES: u64 = 16 * 1_024 * 1_024` at `node/state.rs:22`.
This is a constant initializer, not unchecked runtime arithmetic.

Octet's `src/safety/raw_arithmetic_overflow.rs` explicitly exempts constant operands because the compiler checks them.
However, `is_constant_expr` recognizes only `ExprKind::Lit`.
The nested left operand `(16 * 1_024)` is a binary expression, so the outer multiplication is incorrectly diagnosed.
The two-literal secret bound on the next line is exempted.
Do not replace the expression with saturation, an unexplained literal, or a suppression.

## Measured probes

Tasks 10356 (private helper) and 10357 (this retained helper) each passed five checks:

| Input | Tool | Exit | Observation |
|---|---|---:|---|
| Exact nested constant declaration | rustc | 0 | Accepted |
| Same declaration | Dylint | 101 | Incorrect unsigned multiplication warning |
| Two-literal constant declaration | Dylint | 0 | Existing exemption works |
| Runtime unsigned addition | Dylint | 101 | Genuine candidate remains diagnosed |
| Overflowing `u8` constant | rustc | 1 | E0080; invalid constant rejected |

Inputs and tool digests matched before and after each run. No ICE markers appeared.
These are targeted compiler/driver diagnostics, not complete-source, CLI, VM, or lifecycle evidence.
Only `unknown_lints` and `raw_arithmetic_overflow` are denied in the direct-driver probes.
The production gate flags remain unchanged.
No tool, compiler, or production binary was rebuilt for this review. No lint implementation changed.

## Exact replay

From the Molten worktree, run:

```sh
sh .cairn/changes/node-startup-evidence/evidence/arithmetic-review/verify.sh \
  /home/brittonr/.local/state/onix/molten-node-vm/arithmetic-review/run-2 \
  /nix/store/r5bzbvda2ydnz09c6vxvqhxsmh37nhpw-dylint-driver-5.0.0/bin/dylint-driver \
  /nix/store/40f34yq9qpxmf4k3cdlcj24fj14gjh9f-octet-0.1.0/lib/liboctet.so \
  /nix/store/1yvh3d6y3fj3xk2dgwczrp1dj5svd92c-rust-default-1.96.0-nightly-2026-03-21
```

That exact output now exists; choose a fresh absolute output for another replay.
This helper intentionally reproduces the current defect. A repaired library must not pass the erroneous-warning expectation.

Next owner work: repair constant-expression eligibility with positive and negative controls.
Do not exempt parameter-dependent arithmetic merely because its enclosing function is `const fn`.
The validity of the other 32 findings has not been established.
