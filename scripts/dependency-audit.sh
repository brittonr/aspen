#!/usr/bin/env bash
# An audit never fetches: refreshing the maintained RustSec DB is a separate observation.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"
db= revision= target= out= as_of=
release_args=()
while (($#)); do
  (($# >= 2)) || { echo "missing value: $1" >&2; exit 2; }
  case "$1" in
    --database) db="$2" ;;
    --database-revision) revision="$2" ;;
    --target) target="$2" ;;
    --out) out="$2" ;;
    --as-of) as_of="$2" ;;
    --release-evidence-source) release_args+=(--evidence-source "$2") ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift 2
done
if [[ -z "$db" || ! "$revision" =~ ^[a-f0-9]{40}$ || ! "$target" =~ ^[a-zA-Z0-9_]+-[a-zA-Z0-9_-]+$ || -z "$out" || -z "$as_of" ]]; then
  echo 'usage: molten-dependency-audit --database DIR --database-revision COMMIT --target TRIPLE --as-of YYYY-MM-DD --out NEW-DIR [--release-evidence-source NAME=PATH]' >&2
  exit 2
fi
: "${MOLTEN_AUDIT_SCANNER:?Use the pinned dependency-audit Nix app; ambient scanner is not evidence}"
: "${MOLTEN_RELEASE_POLICY_BIN:?The existing release-policy guard must be supplied by the Nix app}"
: "${MOLTEN_AUDIT_NORMALIZER_BIN:?The Nix app must supply the pinned normalizer}"
[[ "$MOLTEN_AUDIT_SCANNER" == /nix/store/*/bin/cargo-audit ]] || { echo 'scanner must be the pinned Nix store executable' >&2; exit 2; }
db="$(realpath "$db")"
out="$(realpath -m "$out")"
mkdir "$out" # Preserve prior observations, including failed attempts.
mkdir "$out/scanner-home"
capture() {
  local name="$1"; shift
  local status=0
  (ulimit -f 32768; timeout --kill-after=5s 300s "$@") > "$out/$name" 2> "$out/$name.stderr" || status=$?
  printf '%s' "$status"
}
hash() { b3sum --no-names "$1"; }
source_inventory() {
  git ls-files -z --cached --others --exclude-standard |
    while IFS= read -r -d '' file; do
      [[ "$root/$file" == "$out"/* ]] && continue
      if [[ "$file" == *$'\n'* || "$file" == *$'\t'* ]]; then
        echo "unsupported source inventory name: $file" >&2; return 1
      fi
      if [[ -L "$file" ]]; then
        printf 'symlink:blake3:%s\t%s\n' "$(readlink -n "$file" | b3sum --no-names)" "$file"
      elif [[ -f "$file" ]]; then
        printf 'blake3:%s\t%s\n' "$(hash "$file")" "$file"
      else
        echo "unavailable source inventory member: $file" >&2; return 1
      fi
    done |
    jq -Rn '[inputs | split("\t") | {key:.[1],value:.[0]}] | from_entries'
}
source_inventory > "$out/source-files.json"
cp Cargo.lock "$out/Cargo.lock"
# Never treat an arbitrary caller-authored empty repo as a maintained scan.
[[ "$(git -C "$db" rev-parse HEAD)" == "$revision" ]] || { echo 'RustSec revision mismatch' >&2; exit 2; }
origin="$(git -C "$db" remote get-url origin)"
[[ "$origin" == 'https://github.com/RustSec/advisory-db' || "$origin" == 'https://github.com/RustSec/advisory-db.git' ]] || {
  echo 'RustSec origin is not the maintained upstream' >&2; exit 2;
}
[[ -z "$(git -C "$db" status --porcelain --untracked-files=all)" ]] || { echo 'RustSec checkout is dirty' >&2; exit 2; }
git -C "$db" ls-tree -r --full-tree HEAD > "$out/database-tree.txt"
git -C "$db" cat-file commit HEAD > "$out/database-commit.txt"
database_date="$(date -u -d "@$(git -C "$db" show -s --format=%ct HEAD)" +%F)"
rustc_status="$(capture rustc.txt rustc -Vv)"
cargo_status="$(capture cargo.txt cargo -Vv)"
scanner_version_status="$(capture scanner.version "$MOLTEN_AUDIT_SCANNER" --version)"
scanner_version="$(cat "$out/scanner.version")"
[[ "$scanner_version_status" == 0 && "$scanner_version" == 'cargo-audit 0.23.0' ]] || {
  echo 'pinned cargo-audit 0.23.0 is unavailable' >&2; exit 2;
}
metadata_status="$(capture metadata.json cargo metadata --locked --offline --format-version 1 --filter-platform "$target")"
policy_status="$(capture policy.json nickel export --format json config/dependency-audit/policy.ncl)"
release_profile_status="$(capture release-profile.json nickel export --format json config/release-dependencies/profile.ncl)"
license_status="$(capture license-guard.txt cargo -q -Zscript tools/check-license-boundary.rs)"
release_status="$(capture release-guard.json "$MOLTEN_RELEASE_POLICY_BIN" --root "$root" "${release_args[@]}")"
source_status="$(capture source-guard.txt bash scripts/git-source-hashes.sh --check)"
notice_status="$(capture notices.json "$MOLTEN_AUDIT_NORMALIZER_BIN" --capture-notices "$out/metadata.json" --bundle "$out")"
# Empty scanner CARGO_HOME and cwd prevent ambient audit.toml suppression. The
# full lock scan is conservative; the report projects onto the default closure.
scanner_status=0
(
  cd "$out"
  ulimit -f 32768
  CARGO_HOME="$out/scanner-home" timeout --kill-after=5s 300s "$MOLTEN_AUDIT_SCANNER" audit \
    --db "$db" --file "$out/Cargo.lock" --no-fetch --no-yanked --json --deny warnings
) > "$out/scanner.json" 2> "$out/scanner.stderr" || scanner_status=$?
source_inventory > "$out/source-files.after.json"
if ! cmp -s "$out/source-files.json" "$out/source-files.after.json"; then
  metadata_status=125
  printf '\nsource inventory changed during capture\n' >> "$out/metadata.json.stderr"
fi
if [[ "$(git -C "$db" rev-parse HEAD)" != "$revision" || -n "$(git -C "$db" status --porcelain --untracked-files=all)" ]]; then
  scanner_status=125
  printf '\nRustSec checkout changed during capture\n' >> "$out/scanner.stderr"
fi
jq -n --argjson metadata "$metadata_status" --argjson policy "$policy_status" \
  --argjson release_profile "$release_profile_status" --argjson license_guard "$license_status" \
  --argjson release_guard "$release_status" --argjson source_guard "$source_status" \
  --argjson rustc "$rustc_status" --argjson cargo "$cargo_status" \
  --argjson notice_capture "$notice_status" --argjson scanner "$scanner_status" \
  '{metadata:$metadata,policy:$policy,release_profile:$release_profile,license_guard:$license_guard,
    release_guard:$release_guard,source_guard:$source_guard,notice_capture:$notice_capture,scanner:$scanner,
    rustc:$rustc,cargo:$cargo}' > "$out/status.json"
files='{}'
for file in "$out"/*; do
  [[ -f "$file" ]] || continue
  files="$(jq --arg name "${file##*/}" --arg ref "blake3:$(hash "$file")" '. + {($name):$ref}' <<< "$files")"
done
jq -n --arg target "$target" --arg scanner_version "$scanner_version" --arg scanner_path "$MOLTEN_AUDIT_SCANNER" \
  --arg scanner_ref "blake3:$(hash "$MOLTEN_AUDIT_SCANNER")" --arg database_revision "$revision" \
  --arg normalizer_path "$MOLTEN_AUDIT_NORMALIZER_BIN" --arg normalizer_ref "blake3:$(hash "$MOLTEN_AUDIT_NORMALIZER_BIN")" \
  --arg database_date "$database_date" --argjson files "$files" \
  '{schema:"molten.dependency-audit-binding.v2",profile:"molten-default",features:[],default_features:true,target:$target,
    scanner_version:$scanner_version,scanner_path:$scanner_path,scanner_ref:$scanner_ref,
    normalizer_path:$normalizer_path,normalizer_ref:$normalizer_ref,
    database_revision:$database_revision,database_date:$database_date,files:$files}' > "$out/binding.json"
ulimit -f 32768
exec "$MOLTEN_AUDIT_NORMALIZER_BIN" --bundle "$out" --root "$root" --as-of "$as_of"
