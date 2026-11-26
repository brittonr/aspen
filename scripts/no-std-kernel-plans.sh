#!/usr/bin/env bash
# Regenerate (default) or verify (--check) the per-crate unit2nix plans of the no_std kernels.
#
# Each kernel resolves its own unit graph with `unit2nix -p <crate>`, so serde/blake3 features
# are never unified with the std consumers in build-plan.json. Next to each plan, the BLAKE3 of
# every input that can change the resolution is recorded in `<crate>.inputs.b3`:
#   - the workspace `Cargo.toml` and `Cargo.lock`;
#   - `.cargo/config.toml` and `rust-toolchain.toml`;
#   - the manifest of every local package in the kernel's `cargo metadata` path-dependency
#     closure (the kernel itself plus any path dependency it links).
# The `<crate>-no-std` flake checks reject a plan whose recorded inputs differ from the tree or
# whose recorded input list differs from the closure the plan itself resolves.
#
# r[impl molten.project.tier1_no_std_kernel_crates]
set -euo pipefail

root="${MOLTEN_ROOT:-$(git rev-parse --show-toplevel)}"
plans="nix/no-std-kernels"
kernels=(
  molten-composition-core
  molten-fastpath-core
  molten-preserves-core
  molten-raft-core
  molten-reliability-model-core
  molten-runtime-admission-core
  molten-wasm-policy-core
)
mode="${1:-generate}"

# Workspace-relative manifests of the local packages in one kernel's normal-dependency closure.
path_closure() {
  cargo metadata --format-version 1 --locked --manifest-path Cargo.toml \
    | jq -r --arg crate "$1" --arg root "$(pwd -P)" '
      def closure(nodes; ids):
        ([ids[] as $id | nodes[$id].deps[] | select(any(.dep_kinds[]; .kind == null)) | .pkg] + ids | unique) as $next
        | if ($next | length) == (ids | length) then ids else closure(nodes; $next) end;
      (.packages | map({key: .id, value: .}) | from_entries) as $packages
      | (.resolve.nodes | map({key: .id, value: .}) | from_entries) as $nodes
      | [.packages[] | select(.name == $crate) | .id] as $start
      | closure($nodes; $start)[]
      | $packages[.]
      | select(.source == null)
      | .manifest_path
      | ltrimstr($root + "/")'
}

inputs() {
  {
    printf '%s\n' Cargo.toml Cargo.lock .cargo/config.toml rust-toolchain.toml
    path_closure "$1"
  } | sort -u
}

cd "$root"
mkdir -p "$plans"
for crate in "${kernels[@]}"; do
  case "$mode" in
    generate)
      unit2nix -p "$crate" -o "$plans/$crate.build-plan.json"
      inputs "$crate" | xargs b3sum > "$plans/$crate.inputs.b3"
      ;;
    --check)
      if ! b3sum --check --quiet "$plans/$crate.inputs.b3" \
        || [ "$(inputs "$crate")" != "$(awk '{print $2}' "$plans/$crate.inputs.b3" | sort -u)" ]; then
        echo "$plans/$crate.build-plan.json is stale; run scripts/no-std-kernel-plans.sh" >&2
        exit 1
      fi
      ;;
    *)
      echo "usage: $0 [--check]" >&2
      exit 2
      ;;
  esac
done
