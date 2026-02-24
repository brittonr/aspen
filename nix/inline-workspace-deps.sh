#!/usr/bin/env bash
# Inline all `workspace = true` references in Cargo.toml files under $DEPS.
#
# When the main aspen workspace resolves a path dep from a sibling repo,
# cargo resolves `workspace = true` against the MAIN workspace, not the
# sibling's own. This script resolves every `workspace = true` by reading
# values from each crate's actual workspace root.

set -euo pipefail

DEPS="$1"

# Find all workspace roots (Cargo.toml files with [workspace] section)
find "$DEPS" -name Cargo.toml -not -path "*/target/*" | while read -r toml; do
  if grep -q '^\[workspace\]' "$toml" 2>/dev/null; then
    echo "$toml"
  fi
done | sort -r > /tmp/_ws_roots  # Deeper paths first

# AWK program that inlines workspace = true references
# Usage: awk -v ws_pkg_file=... -v ws_deps_file=... -f inline.awk crate.toml
cat > /tmp/_inline.awk << 'AWKEOF'
BEGIN {
  # Read workspace.package key = value pairs
  while ((getline line < ws_pkg_file) > 0) {
    idx = index(line, " = ")
    if (idx > 0) {
      name = substr(line, 1, idx-1)
      val = substr(line, idx+3)
      gsub(/^ +| +$/, "", name)
      ws_pkg[name] = val
    }
  }
  close(ws_pkg_file)

  # Read workspace.dependencies (pre-flattened: name=value per line)
  while ((getline line < ws_deps_file) > 0) {
    idx = index(line, "=")
    if (idx > 0) {
      name = substr(line, 1, idx-1)
      val = substr(line, idx+1)
      gsub(/^ +/, "", name); gsub(/ +$/, "", name)
      gsub(/^ +/, "", val)
      ws_dep[name] = val
    }
  }
  close(ws_deps_file)
}

# Helper: fix path values to be relative to crate instead of workspace root
function fix_path(val,    result) {
  result = val
  # Replace path = "X" with path = "prefix/X"
  if (path_prefix != "" && result ~ /path *= *"/) {
    gsub(/path *= *"/, "path = \"" path_prefix "/", result)
  }
  return result
}

# Package field: key = { workspace = true }
/^[a-zA-Z].*= *\{ *workspace *= *true *\}/ {
  match($0, /^([a-zA-Z0-9_-]+)/, m)
  key = m[1]
  if (key in ws_dep) {
    print key " = " fix_path(ws_dep[key])
    next
  }
  if (key in ws_pkg) {
    print key " = " ws_pkg[key]
    next
  }
}

# Complex: dep = { workspace = true, optional = true, ... }
/^[a-zA-Z].*= *\{ *workspace *= *true *,/ {
  match($0, /^([a-zA-Z0-9_-]+)/, m)
  key = m[1]
  if (key in ws_dep) {
    ws_val = ws_dep[key]
    # Get everything after "workspace = true, "
    extra = $0
    sub(/^[^{]*\{ *workspace *= *true *, */, "", extra)

    if (ws_val ~ /^"/) {
      # Simple version string
      print key " = { version = " ws_val ", " extra
    } else {
      # Table value: fix paths, remove trailing }
      ws_val = fix_path(ws_val)
      sub(/} *$/, "", ws_val)

      # Handle feature merging: if both ws_val and extra have features,
      # remove features from ws_val (crate-level features override)
      if (extra ~ /features *=/ && ws_val ~ /features *=/) {
        sub(/, *features *= *\[[^\]]*\]/, "", ws_val)
      }
      # Also handle default-features conflict
      if (extra ~ /default-features *=/ && ws_val ~ /default-features *=/) {
        sub(/, *default-features *= *[a-z]+/, "", ws_val)
      }

      print key " = " ws_val ", " extra
    }
    next
  }
}

# Default: pass through
{ print }
AWKEOF

while read -r ws_root; do
  ws_dir=$(dirname "$ws_root")

  # Extract [workspace.package] with multi-line flattening
  awk '
    /^\[workspace\.package\]/ { in_pkg=1; next }
    /^\[/ && in_pkg { in_pkg=0 }
    !in_pkg { next }
    /^[a-zA-Z]/ {
      if (name != "") print name " = " val
      n = $0; sub(/ *=.*/, "", n); gsub(/^ +| +$/, "", n); name = n
      v = $0; sub(/^[^=]+= */, "", v); val = v
      next
    }
    /^[^[]/ && name != "" { gsub(/^ +/, " "); val = val $0; next }
    END { if (name != "") print name " = " val }
  ' "$ws_root" > /tmp/_ws_pkg

  # Extract [workspace.dependencies] with multi-line flattening
  awk '
    /^\[workspace\.dependencies\]/ { in_deps=1; next }
    /^\[/ && in_deps { in_deps=0 }
    !in_deps { next }
    /^#/ || /^$/ {
      # Comment or blank line — flush current entry
      if (name != "") { print name "=" val; name = "" }
      next
    }
    /^[a-zA-Z]/ {
      if (name != "") print name "=" val
      n = $0; sub(/ *=.*/, "", n); gsub(/^ +| +$/, "", n); name = n
      v = $0; sub(/^[^=]+= */, "", v); val = v
      next
    }
    /^[^[]/ && name != "" { gsub(/^ +/, " "); val = val $0; next }
    END { if (name != "") print name "=" val }
  ' "$ws_root" > /tmp/_ws_deps

  # Process every Cargo.toml under this workspace
  find "$ws_dir" -name Cargo.toml -not -path "*/target/*" | while read -r crate_toml; do
    [ "$crate_toml" = "$ws_root" ] && continue
    grep -q 'workspace = true' "$crate_toml" 2>/dev/null || continue

    # Calculate relative path prefix from crate to workspace root
    crate_dir=$(dirname "$crate_toml")
    rel_to_ws=$(realpath --relative-to="$crate_dir" "$ws_dir")
    # rel_to_ws is like "../.." — we need to prepend this to workspace-relative paths

    awk -v ws_pkg_file=/tmp/_ws_pkg -v ws_deps_file=/tmp/_ws_deps \
      -v path_prefix="$rel_to_ws" \
      -f /tmp/_inline.awk "$crate_toml" > "${crate_toml}.tmp"
    mv "${crate_toml}.tmp" "$crate_toml"
  done
done < /tmp/_ws_roots

rm -f /tmp/_ws_roots /tmp/_ws_pkg /tmp/_ws_deps /tmp/_inline.awk
