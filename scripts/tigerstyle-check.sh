#!/usr/bin/env bash
# Run Aspen's current Tiger Style starter-lint pilot through Dylint.
#
# Requirements:
#   - `cargo-dylint` and `dylint-link` installed in PATH
#   - sibling tigerstyle repo at ../tigerstyle, or set TIGERSTYLE_PATH
#
# Why wrappers exist:
#   - Aspen uses a nix-managed nightly toolchain, so `rustup show active-toolchain`
#     reports `nix-managed` instead of a full nightly triple.
#   - `cargo-dylint` builds a driver crate that still needs the nightly-fix patch
#     for `dylint_driver` on current rustc.
#   - `dylint-link` expects `RUSTUP_TOOLCHAIN` during link steps.
#
# This script creates temporary wrappers/config so Aspen can run tigerstyle
# without mutating the caller's shell environment.
#
# Usage:
#   scripts/tigerstyle-check.sh
#   scripts/tigerstyle-check.sh -p aspen-core -p aspen-time
#   TIGERSTYLE_PATH=~/src/tigerstyle scripts/tigerstyle-check.sh --workspace -- --all-targets

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/tigerstyle-check.sh [cargo-dylint options] [package scope] [-- cargo-check args]

Default scope (when no package/workspace is passed):
  -p aspen-time -p aspen-hlc -p aspen-core -p aspen-coordination

Starter lints enabled in phase 2 default pilot:
  - tigerstyle::compound_assertion
  - tigerstyle::ambient_clock
  - tigerstyle::contradictory_time

Examples:
  scripts/tigerstyle-check.sh
  scripts/tigerstyle-check.sh -p aspen-core -p aspen-time
  scripts/tigerstyle-check.sh --workspace -- --all-targets
  scripts/tigerstyle-check.sh --with-deps -p aspen-core -- --all-targets
  TIGERSTYLE_PATH=~/src/tigerstyle scripts/tigerstyle-check.sh --keep-going
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    exit 1
  fi
}

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

need_cmd cargo
need_cmd rustc
need_cmd python3
need_cmd cargo-dylint
need_cmd dylint-link

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "error: not inside the Aspen git repository" >&2
  exit 1
}
cd "$repo_root"

tigerstyle_path=$(realpath "${TIGERSTYLE_PATH:-$repo_root/../tigerstyle}")
if [[ ! -d "$tigerstyle_path" ]]; then
  echo "error: tigerstyle repo not found at $tigerstyle_path" >&2
  echo "set TIGERSTYLE_PATH=/path/to/tigerstyle and retry" >&2
  exit 1
fi
if [[ ! -f "$repo_root/dylint.toml" ]]; then
  echo "error: expected Aspen dylint config at $repo_root/dylint.toml" >&2
  exit 1
fi

host_triple=$(rustc -vV | awk '/^host:/ {print $2}')
toolchain_channel=$(python3 - "$tigerstyle_path/rust-toolchain.toml" <<'PY'
import pathlib
import sys
import tomllib
path = pathlib.Path(sys.argv[1])
data = tomllib.loads(path.read_text())
print(data["toolchain"]["channel"])
PY
)
toolchain="$toolchain_channel"
if [[ "$toolchain" != *"$host_triple" ]]; then
  toolchain="${toolchain}-${host_triple}"
fi

dylint_rev="${TIGERSTYLE_DYLINT_REV:-0e0a71eefe6f01563d11acc6e1d7af1d505934a9}"
real_cargo=$(command -v cargo)
real_rustc=$(command -v rustc)
real_dylint_link=$(command -v dylint-link)

wrap_dir=$(mktemp -d)
cargo_home_tmp=$(mktemp -d)
cleanup() {
  rm -rf "${wrap_dir:-}" "${cargo_home_tmp:-}"
}
trap cleanup EXIT

ln -s "${CARGO_HOME:-$HOME/.cargo}/registry" "$cargo_home_tmp/registry"
ln -s "${CARGO_HOME:-$HOME/.cargo}/git" "$cargo_home_tmp/git"
cat > "$cargo_home_tmp/config.toml" <<EOF
[patch.crates-io]
dylint_driver = { git = "https://github.com/trailofbits/dylint", rev = "$dylint_rev" }
EOF

cat > "$wrap_dir/rustup" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  show)
    if [[ "${2:-}" == "active-toolchain" ]]; then
      echo "__TOOLCHAIN__ (wrapper)"
      exit 0
    fi
    ;;
  which)
    if [[ "${2:-}" == "rustc" ]]; then
      echo "__RUSTC__"
      exit 0
    fi
    ;;
esac
echo "unsupported rustup wrapper invocation: $*" >&2
exit 1
EOF

cat > "$wrap_dir/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-__TOOLCHAIN__}"
exec "__REAL_CARGO__" "$@"
EOF

cat > "$wrap_dir/dylint-link" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-__TOOLCHAIN__}"
exec "__REAL_DYLINT_LINK__" "$@"
EOF

sed -i \
  -e "s|__TOOLCHAIN__|$toolchain|g" \
  -e "s|__RUSTC__|$real_rustc|g" \
  "$wrap_dir/rustup"
sed -i \
  -e "s|__TOOLCHAIN__|$toolchain|g" \
  -e "s|__REAL_CARGO__|$real_cargo|g" \
  "$wrap_dir/cargo"
sed -i \
  -e "s|__TOOLCHAIN__|$toolchain|g" \
  -e "s|__REAL_DYLINT_LINK__|$real_dylint_link|g" \
  "$wrap_dir/dylint-link"
chmod +x "$wrap_dir/rustup" "$wrap_dir/cargo" "$wrap_dir/dylint-link"

upper_host=${host_triple^^}
upper_host=${upper_host//-/_}
linker_var="CARGO_TARGET_${upper_host}_LINKER"
export "$linker_var"=dylint-link
export PATH="$wrap_dir:${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
export CARGO_HOME="$cargo_home_tmp"
export DYLINT_DRIVER_PATH="$cargo_home_tmp/dylint-drivers"
mkdir -p "$DYLINT_DRIVER_PATH"

starter_lints=(
  ambient_clock
  compound_assertion
  contradictory_time
)

scope_args=()
dylint_args=(--all --path "$tigerstyle_path")
check_args=()
saw_scope=0
no_deps=1
while (($#)); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    -p|--package)
      saw_scope=1
      scope_args+=("$1" "$2")
      shift 2
      ;;
    --workspace)
      saw_scope=1
      no_deps=0
      scope_args+=("$1")
      shift
      ;;
    --with-deps)
      no_deps=0
      shift
      ;;
    --)
      shift
      check_args=("$@")
      break
      ;;
    *)
      dylint_args+=("$1")
      shift
      ;;
  esac
done

if (( ! saw_scope )); then
  scope_args=(
    -p aspen-time
    -p aspen-hlc
    -p aspen-core
    -p aspen-coordination
  )
fi

if (( no_deps )); then
  dylint_args+=(--no-deps)
fi

if [[ ${#check_args[@]} -eq 0 ]]; then
  check_args=(--lib)
fi

echo "[tigerstyle] repo:      $tigerstyle_path"
echo "[tigerstyle] toolchain: $toolchain"
echo "[tigerstyle] scope:     ${scope_args[*]}"
echo "[tigerstyle] lints:     ${starter_lints[*]}"

exec cargo dylint "${dylint_args[@]}" "${scope_args[@]}" -- "${check_args[@]}"
