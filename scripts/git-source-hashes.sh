#!/usr/bin/env bash
# Bind every Cargo git source to the metadata-free NAR hash that `pkgs.fetchgit` produces.
#
# unit2nix prefetches missing git hashes with `nix-prefetch-git --leave-dotGit`, but its
# `fetch-source.nix` builds with `pkgs.fetchgit` without `leaveDotGit`. Those hashes include `.git`
# and never match on a clean builder. This script records revision-qualified hashes in
# `crate-hashes.json` so unit2nix applies them and never falls back to its own prefetch.
#
# Usage:
#   scripts/git-source-hashes.sh --write   # prefetch each Cargo.lock git source and rewrite crate-hashes.json
#   scripts/git-source-hashes.sh --check   # fail if crate-hashes.json or a generated plan drifts from those hashes
#
# Regenerate the plans after `--write` with the pinned unit2nix:
#   unit2nix --workspace -o build-plan.json
#   unit2nix -p molten-release-policy --bin molten-release-policy -o release-policy-build-plan.json
#
set -euo pipefail

mode="${1:-}"
root="${MOLTEN_ROOT:-$(git rev-parse --show-toplevel)}"
lock="$root/Cargo.lock"
flake_lock="$root/flake.lock"
hashes="$root/crate-hashes.json"

sources() {
  # One "url rev crate@version" line per distinct git url and revision in Cargo.lock.
  # `git+rad://` sources are feature-gated and outside the generated plans, so they stay excluded.
  awk '
    /^name = / { name = $3; gsub(/"/, "", name) }
    /^version = / { version = $3; gsub(/"/, "", version) }
    /^source = "git\+/ {
      source = $3
      gsub(/"/, "", source)
      sub(/^git\+/, "", source)
      if (source ~ /^rad:/) next
      split(source, parts, "#")
      split(parts[1], query, /[?]rev=/)
      key = query[1] " " query[2]
      if (!(key in seen)) {
        seen[key] = 1
        print query[1], query[2], name "@" version
      }
    }
  ' "$lock" | sort
}

prefetch() {
  nix-prefetch-git --url "$1" --rev "$2" --fetch-submodules --quiet | jq -r '.hash'
}

locked_input_hash() {
  # A flake input locked at the same revision was fetched by Nix without `.git`, so its narHash is
  # the same metadata-free identity. Inputs that fetch submodules are excluded.
  jq -r --arg rev "$1" '
    [.nodes[] | .locked? // empty | select(.rev == $rev and (.submodules // false) == false) | .narHash]
    | unique | if length == 1 then .[0] else "" end
  ' "$flake_lock"
}

case "$mode" in
  --write)
    entries='{}'
    while read -r url rev crate; do
      if ! hash="$(prefetch "$url" "$rev")"; then
        hash="$(locked_input_hash "$rev")"
        if [ -z "$hash" ]; then
          echo "cannot prefetch $url at $rev and no flake input is locked at that revision" >&2
          exit 1
        fi
        echo "prefetch unavailable for $url; using the flake.lock narHash locked at $rev" >&2
      fi
      case "$hash" in
        sha256-*) ;;
        *)
          echo "prefetch returned no SRI hash for $url at $rev" >&2
          exit 1
          ;;
      esac
      entries="$(jq --arg key "$url?rev=$rev#$crate" --arg hash "$hash" '. + {($key): $hash}' <<<"$entries")"
    done < <(sources)
    jq -S . <<<"$entries" > "$hashes"
    echo "wrote $(jq length "$hashes") revision-qualified git source hashes to crate-hashes.json"
    ;;
  --check)
    status=0
    while read -r url rev _crate; do
      if ! jq -e --arg prefix "$url?rev=$rev#" \
        '[to_entries[] | select((.key | startswith($prefix)) and (.value | startswith("sha256-")))] | length == 1' \
        "$hashes" >/dev/null; then
        echo "crate-hashes.json lacks exactly one revision-qualified SRI hash for $url at $rev" >&2
        status=1
        continue
      fi
      locked="$(locked_input_hash "$rev")"
      if [ -n "$locked" ] && ! jq -e --arg prefix "$url?rev=$rev#" --arg locked "$locked" \
        '[to_entries[] | select(.key | startswith($prefix)) | .value] == [$locked]' "$hashes" >/dev/null; then
        echo "crate-hashes.json hash for $url at $rev differs from the flake.lock narHash $locked" >&2
        status=1
      fi
    done < <(sources)
    for plan in build-plan.json release-policy-build-plan.json; do
      # Every generated git source hash must be the revision-qualified crate-hashes.json value.
      drift="$(jq -r --slurpfile hashes "$hashes" '
        ($hashes[0] | to_entries | map({key: (.key | split("#")[0]), value}) | from_entries) as $bound
        | .crates[] | .source? // empty | select(.type == "git")
        | (.url + "?rev=" + .rev) as $key
        | select($bound[$key] != .sha256)
        | "\($key) plan=\(.sha256) bound=\($bound[$key] // "missing")"
      ' "$root/$plan" | sort -u)"
      if [ -n "$drift" ]; then
        echo "$plan git source hashes differ from crate-hashes.json:" >&2
        echo "$drift" >&2
        status=1
      fi
    done
    if jq -e '[keys[] | select(contains("?rev=") | not)] | length > 0' "$hashes" >/dev/null; then
      echo "crate-hashes.json contains keys without ?rev=; they would apply to any revision" >&2
      status=1
    fi
    exit "$status"
    ;;
  *)
    echo "usage: $0 --write|--check" >&2
    exit 2
    ;;
esac
