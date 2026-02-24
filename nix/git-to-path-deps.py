"""Convert git dependencies pointing at the aspen GitHub repo to path deps.

After workspace-deps inlining, some sibling-repo crates still have git
references like:
  aspen-core = { git = "https://github.com/brittonr/aspen.git", branch = "main" }

These need to become path deps pointing at the actual crate location in the
unified workspace tree.
"""

import os
import re
import sys

ROOT = sys.argv[1]

# Map crate names to their paths relative to ROOT
CRATE_PATHS: dict[str, str] = {}

# Scan for all crates in the workspace
for dirpath, dirnames, filenames in os.walk(ROOT):
    if "Cargo.toml" in filenames and "target" not in dirpath:
        toml_path = os.path.join(dirpath, "Cargo.toml")
        with open(toml_path) as f:
            content = f.read()
        # Extract package name
        m = re.search(r'^name\s*=\s*"([^"]+)"', content, re.MULTILINE)
        if m:
            name = m.group(1)
            rel_path = os.path.relpath(dirpath, ROOT)
            CRATE_PATHS[name] = rel_path

# Pattern for git deps pointing at the aspen repo
GIT_DEP_PATTERN = re.compile(
    r'^([a-zA-Z0-9_-]+)\s*=\s*\{[^}]*git\s*=\s*"https://github\.com/brittonr/aspen\.git"[^}]*\}',
    re.MULTILINE,
)

# Also match git deps to other brittonr repos
GIT_DEP_PATTERN2 = re.compile(
    r'^([a-zA-Z0-9_-]+)\s*=\s*\{[^}]*git\s*=\s*"https://github\.com/brittonr/[^"]*"[^}]*\}',
    re.MULTILINE,
)

count = 0

for dirpath, dirnames, filenames in os.walk(ROOT):
    if "Cargo.toml" in filenames and "target" not in dirpath:
        toml_path = os.path.join(dirpath, "Cargo.toml")
        with open(toml_path) as f:
            content = f.read()

        modified = False

        for pattern in [GIT_DEP_PATTERN, GIT_DEP_PATTERN2]:
            for m in pattern.finditer(content):
                dep_name = m.group(1)
                if dep_name in CRATE_PATHS:
                    crate_dir = os.path.relpath(dirpath, ROOT)
                    rel_to_crate = os.path.relpath(CRATE_PATHS[dep_name], crate_dir)

                    # Preserve features and optional flags from the original
                    original = m.group(0)
                    features_match = re.search(r'features\s*=\s*\[[^\]]*\]', original)
                    optional_match = re.search(r'optional\s*=\s*true', original)
                    default_features_match = re.search(r'default-features\s*=\s*(true|false)', original)

                    parts = [f'path = "{rel_to_crate}"']
                    if features_match:
                        parts.append(features_match.group(0))
                    if optional_match:
                        parts.append("optional = true")
                    if default_features_match:
                        parts.append(default_features_match.group(0))

                    replacement = f'{dep_name} = {{ {", ".join(parts)} }}'
                    content = content.replace(original, replacement)
                    modified = True
                    count += 1

        if modified:
            with open(toml_path, "w") as f:
                f.write(content)

print(f"Converted {count} git deps to path deps")
