#!/usr/bin/env bash
# Consumer controls. Fixture observations are never release audit evidence.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"
db= revision= out=
while (($#)); do
  (($# >= 2)) || { echo "missing value: $1" >&2; exit 2; }
  case "$1" in
    --database) db="$2" ;;
    --database-revision) revision="$2" ;;
    --out) out="$2" ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift 2
done
: "${MOLTEN_AUDIT_SCANNER:?Use the dependency-audit-controls Nix app}"
: "${MOLTEN_AUDIT_NORMALIZER_BIN:?Use the dependency-audit-controls Nix app}"
[[ -n "$db" && "$revision" =~ ^[a-f0-9]{40}$ && -n "$out" ]] || {
  echo 'usage: molten-dependency-audit-controls --database DIR --database-revision COMMIT --out NEW-DIR' >&2; exit 2;
}
db="$(realpath "$db")"
out="$(realpath -m "$out")"
mkdir "$out"
mkdir "$out/scanner-home"
[[ "$(git -C "$db" rev-parse HEAD)" == "$revision" && -z "$(git -C "$db" status --porcelain --untracked-files=all)" ]] || {
  echo 'RustSec revision mismatch or dirty snapshot' >&2; exit 2;
}
origin="$(git -C "$db" remote get-url origin)"
[[ "$origin" == 'https://github.com/RustSec/advisory-db' || "$origin" == 'https://github.com/RustSec/advisory-db.git' ]] || {
  echo 'controls require the actual maintained RustSec origin' >&2; exit 2;
}
(ulimit -f 32768; timeout --kill-after=5s 300s "$MOLTEN_AUDIT_NORMALIZER_BIN" --self-test) \
  > "$out/normalizer-controls.txt" 2> "$out/normalizer-controls.stderr"
"$MOLTEN_AUDIT_SCANNER" --version > "$out/scanner.version"
[[ "$(cat "$out/scanner.version")" == 'cargo-audit 0.23.0' ]] || { echo 'scanner pin mismatch' >&2; exit 2; }
status=0
(
  cd "$out"
  ulimit -f 32768
  CARGO_HOME="$out/scanner-home" timeout --kill-after=5s 300s "$MOLTEN_AUDIT_SCANNER" audit \
    --db "$db" --file "$root/config/dependency-audit/fixtures/affected-lockfile.toml" \
    --no-fetch --no-yanked --json
) > "$out/affected.json" 2> "$out/affected.stderr" || status=$?
[[ "$status" == 1 ]] || { echo "affected fixture scanner unavailable/unexpected status: $status" >&2; exit 1; }
[[ "$(git -C "$db" rev-parse HEAD)" == "$revision" && -z "$(git -C "$db" status --porcelain --untracked-files=all)" ]] || {
  echo 'RustSec snapshot changed during scanner execution' >&2; exit 1;
}
jq -e --arg revision "$revision" '(.database["last-commit"] == null or .database["last-commit"] == $revision) and
  (.vulnerabilities.list | any(.advisory.id == "RUSTSEC-2020-0071" and .package.name == "time" and .package.version == "0.1.43"))' \
  "$out/affected.json" > /dev/null
jq -n --arg revision "$revision" --arg raw_ref "blake3:$(b3sum --no-names "$out/affected.json")" \
  '{schema:"molten.dependency-audit-controls.v1",fixture_only:true,database_revision:$revision,affected_raw_ref:$raw_ref,decision:"pass"}' \
  > "$out/controls.json"
printf 'audit controls passed (fixture only); retained output: %s\n' "$out"
