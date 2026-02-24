"""Merge Cargo.lock files: add missing [[package]] entries from source to target."""

import sys
import re


def parse_packages(content: str) -> dict[tuple[str, str], str]:
    """Parse [[package]] entries from Cargo.lock content.
    Returns dict of (name, version) -> full text block."""
    packages = {}
    # Split on [[package]] headers
    blocks = re.split(r'^(\[\[package\]\])', content, flags=re.MULTILINE)

    i = 0
    while i < len(blocks):
        if blocks[i].strip() == '[[package]]' and i + 1 < len(blocks):
            block_text = blocks[i] + blocks[i + 1]
            # Extract name and version (handles both v3 and v4 formats)
            name_match = re.search(r'^name\s*=\s*"([^"]+)"', block_text, re.MULTILINE)
            ver_match = re.search(r'^version\s*=\s*"([^"]+)"', block_text, re.MULTILINE)
            if name_match and ver_match:
                packages[(name_match.group(1), ver_match.group(1))] = block_text.rstrip()
            i += 2
        else:
            i += 1

    return packages


def main():
    target_path = sys.argv[1]
    source_path = sys.argv[2]

    with open(target_path) as f:
        target_content = f.read()
    with open(source_path) as f:
        source_content = f.read()

    target_pkgs = parse_packages(target_content)
    source_pkgs = parse_packages(source_content)

    # Find missing packages
    missing = {k: v for k, v in source_pkgs.items() if k not in target_pkgs}

    if missing:
        with open(target_path, 'a') as f:
            for (name, ver), block in sorted(missing.items()):
                f.write(f'\n{block}\n')

        print(f"Merged {len(missing)} packages: {', '.join(f'{n} {v}' for n, v in sorted(missing.keys()))}")
    else:
        print("No missing packages to merge")


if __name__ == '__main__':
    main()
