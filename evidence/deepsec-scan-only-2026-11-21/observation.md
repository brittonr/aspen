# Aspen #64: one-time deepsec regex-only observation

**Decision:** Complete the explicit scan-only experiment; do not adopt deepsec in Aspen CI or release checks. Revisit only if a reviewed Rust matcher covers a named Aspen surface or an operator authorizes a separate isolated, model-backed experiment with a written spend and time bound. Aspen security tooling maintains this record. No dependency audit or security qualification is implied.

## Exact inputs and isolation

- Aspen commit `d189ce48e760bbba10522d885e31a647df10f14c`, Git tree `448bd6d4885feacf8e2117907b638642830987a9`, materialized by `git archive`.
- npm `deepsec@2.3.16`, registry `gitHead` `9444b896ca60eb399f9cebaea0bef61655781d42`, archive integrity `sha512-zqqDaQjGSxlrRd7wAOawatPgFrI2QdsPuMMAkL96dTAz+WU3xG5dhlLFozp5kZHhvwtijtwC05Id2l27QiIY+g==`. The installed transitive lock BLAKE3 is `307816045427fba7c68921e5e5b88070bd2b24d72adfa6d2440b0bd92d760696`.
- npm acquisition only: `env -i HOME=<scratch>/empty PATH=<tool-path> timeout 180s npm install --ignore-scripts --no-audit --no-fund` in a fresh private package directory with `deepsec` pinned as `"2.3.16"`. No package hooks ran.
- Execution used Node `v24.15.0` from nixpkgs `f9d8b65950353691ab56561e7c73d2e1063d810b` in a non-fixed-output Nix build sandbox with no network, a disposable HOME/project, and no model, Vercel Gateway, bring-your-own API key, authorization tokens, or subscription. No `setup`, `process`, `revalidate`, or paid worker ran.

## Execution

```sh
nix build --impure --builders '' --max-jobs 1 --cores 2 \
  --option substituters https://cache.nixos.org --option sandbox true \
  --no-link --print-out-paths -L --file deepsec-scan.nix
# Inside the sandbox, using the pinned CLI:
timeout 30s node "$cli" init --scaffold-only
timeout 200s node "$cli" scan
timeout 30s node "$cli" export --format md-dir --out "$out/findings"
```

Pueue task 45 completed. Scaffold, scan and export each exited 0; the regex phase took 6.3 s. Output `/nix/store/q36ggpslpw9lyyky6arw68s1ws3k7xdw-aspen-deepsec-scan-only` retains the command outputs and empty export. The run ID was `20261121212641-f4d605415d5ae3a1`.

| Artifact in realized output | BLAKE3 |
| --- | --- |
| [`scaffold.log`](scaffold.log) | `7b2b277fdf1cb829cd9ec4fd71f6d3af002251921879f2f180fe97e116ca0d53` |
| [`scan.log`](scan.log) | `3e7b38104cbf38aa2fdf2fff787f5661007b58966e4742167c5448843379c4af` |
| [`export.log`](export.log) | `a4263a83ae63e34d6ddd54572bd859cfafae7f31624cda6d36555dbdf076dd1c` |

Deepsec reported 80 regex matches in 80 files and zero matches across 2,886 Rust files; it explicitly reported very low Rust coverage. Those matches are scanner candidates, not findings. Export emitted **zero** files because model triage did not run. Neither the absence of exported findings nor the lack of Rust matches establishes a clean tree. Raw candidate data stays in the exact local output pending review; no findings or fix issues were published, and no repository `.deepsec/` or default workflow was added.
