#!/usr/bin/env python3
"""Check local links and anchors in the public documentation entry points (issue #29).

Default scope, relative to the repository root: ``README.md``, ``CONTRIBUTING.md``,
``SECURITY.md`` and every ``docs/**/*.md``. Each Markdown inline link, image,
reference definition and HTML ``href``/``src`` attribute outside fenced code
blocks and inline code spans is checked:

- targets with a URL scheme (``https:``, ``mailto:``, ``issue:`` ...) or a
  network path (``//host``) are external and skipped;
- a relative or root-relative path must name an existing file or directory
  inside the repository;
- a ``#fragment`` on a Markdown file must match a GitHub heading slug or an
  explicit ``id``/``name`` attribute; on an HTML file an ``id`` attribute; on
  any other text file a GitHub line anchor (``#L12`` or ``#L12-L20``) within
  its line count.

A passing check is link hygiene, not semantic review: it does not prove a
linked page still describes the behaviour the linking text claims.

Usage:
    check-doc-links.py [--root DIR] [FILE...]

Exit status: 0 clean, 1 findings, 2 input error.
"""
# r[impl molten.project.documentation_links.local_targets]
import argparse
import html
import re
import sys
import urllib.parse
from pathlib import Path

ENTRY_FILES = ("README.md", "CONTRIBUTING.md", "SECURITY.md")
DOCS_ROOT = "docs"

FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
CODE_SPAN = re.compile(r"(`+)(?:(?!\1).)+?\1")
INLINE_LINK = re.compile(r"!?\[(?:[^\[\]\\]|\\.|\[[^\[\]]*\])*\]\(\s*(<[^>\n]*>|[^\s()]*(?:\([^\s()]*\)[^\s()]*)*)(?:\s+(?:\"[^\"]*\"|'[^']*'|\([^)]*\)))?\s*\)")
REFERENCE_DEFINITION = re.compile(r"^ {0,3}\[[^\]]+\]:\s*(<[^>\n]*>|\S+)")
HTML_ATTRIBUTE = re.compile(r"""\b(?:href|src)\s*=\s*(?:"([^"]*)"|'([^']*)')""", re.IGNORECASE)
HTML_ID = re.compile(r"""\b(?:id|name)\s*=\s*(?:"([^"]*)"|'([^']*)')""", re.IGNORECASE)
SCHEME = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
ATX_HEADING = re.compile(r"^ {0,3}(#{1,6})(?:[ \t]+(.*?))?(?:[ \t]+#+)?[ \t]*$")
SETEXT_UNDERLINE = re.compile(r"^ {0,3}(=+|-+)[ \t]*$")
LINE_ANCHOR = re.compile(r"^L(\d+)(?:-L(\d+))?$")
HTML_TAG = re.compile(r"<[^>]+>")
IMAGE_OR_LINK = re.compile(r"!?\[((?:[^\[\]]|\[[^\]]*\])*)\]\([^)]*\)")
NON_SLUG = re.compile(r"[^\w\- ]", re.UNICODE)


def prose_lines(text: str):
    """Yield (line number, line) outside fenced code blocks."""
    fence = None
    for number, line in enumerate(text.splitlines(), start=1):
        match = FENCE.match(line)
        if fence is None:
            if match:
                fence = match.group(1)
                continue
            yield number, line
        elif match and match.group(1)[0] == fence[0] and len(match.group(1)) >= len(fence) and not line.strip()[len(match.group(1)):].strip():
            fence = None


def link_targets(text: str):
    """Yield (line number, raw target) for every link-like reference outside code."""
    for number, line in prose_lines(text):
        visible = CODE_SPAN.sub(lambda match: " " * len(match.group(0)), line)
        for match in INLINE_LINK.finditer(visible):
            yield number, match.group(1)
        definition = REFERENCE_DEFINITION.match(visible)
        if definition:
            yield number, definition.group(1)
        for match in HTML_ATTRIBUTE.finditer(visible):
            yield number, match.group(1) if match.group(1) is not None else match.group(2)


def heading_text(raw: str) -> str:
    text = IMAGE_OR_LINK.sub(lambda match: match.group(1), raw)
    text = HTML_TAG.sub("", text)
    text = html.unescape(text)
    return text.replace("`", "").replace("*", "").replace("~~", "")


def github_slug(text: str) -> str:
    return NON_SLUG.sub("", heading_text(text).strip().lower()).replace(" ", "-")


def markdown_anchors(text: str) -> set[str]:
    anchors: set[str] = set()
    seen: dict[str, int] = {}
    previous = None
    for number, line in prose_lines(text):
        title = None
        atx = ATX_HEADING.match(line)
        if atx:
            title = atx.group(2) or ""
        elif previous is not None and previous[0] == number - 1 and previous[1].strip() and SETEXT_UNDERLINE.match(line):
            if not ATX_HEADING.match(previous[1]) and not previous[1].lstrip().startswith(("|", "-", "*", ">")):
                title = previous[1]
        if title is not None:
            slug = github_slug(title)
            count = seen.get(slug, 0)
            seen[slug] = count + 1
            anchors.add(slug if count == 0 else f"{slug}-{count}")
        visible = CODE_SPAN.sub("", line)
        for match in HTML_ID.finditer(visible):
            anchors.add(match.group(1) if match.group(1) is not None else match.group(2))
        previous = (number, line)
    return anchors


def html_anchors(text: str) -> set[str]:
    return {match.group(1) if match.group(1) is not None else match.group(2) for match in HTML_ID.finditer(text)}


class Checker:
    def __init__(self, root: Path):
        self.root = root.resolve()
        self.texts: dict[Path, str] = {}
        self.anchor_cache: dict[Path, set[str]] = {}

    def text(self, path: Path) -> str | None:
        if path not in self.texts:
            try:
                self.texts[path] = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                return None
        return self.texts[path]

    def anchors(self, path: Path) -> set[str] | None:
        if path not in self.anchor_cache:
            text = self.text(path)
            if text is None:
                return None
            self.anchor_cache[path] = markdown_anchors(text) if path.suffix == ".md" else html_anchors(text)
        return self.anchor_cache[path]

    def check_target(self, source: Path, raw: str) -> str | None:
        target = raw.strip()
        if target.startswith("<") and target.endswith(">"):
            target = target[1:-1].strip()
        if not target or SCHEME.match(target) or target.startswith("//"):
            return None
        location, _, fragment = target.partition("#")
        location = urllib.parse.unquote(location.partition("?")[0])
        fragment = urllib.parse.unquote(fragment)
        if location:
            base = self.root if location.startswith("/") else source.parent
            resolved = (base / location.lstrip("/")).resolve()
            if resolved != self.root and self.root not in resolved.parents:
                return "target leaves the repository"
            if not resolved.exists():
                return "target does not exist"
        else:
            resolved = source
        if not fragment:
            return None
        if resolved.is_dir():
            return "fragment on a directory target cannot be checked"
        if resolved.suffix in (".md", ".html", ".htm"):
            anchors = self.anchors(resolved)
            if anchors is None:
                return "target is not readable UTF-8 text"
            if fragment not in anchors and fragment.lower() not in anchors:
                return f"anchor #{fragment} is not defined"
            return None
        line_anchor = LINE_ANCHOR.match(fragment)
        text = self.text(resolved)
        if line_anchor is None or text is None:
            return f"anchor #{fragment} cannot be checked in this file type"
        last = int(line_anchor.group(2) or line_anchor.group(1))
        if int(line_anchor.group(1)) < 1 or last > len(text.splitlines()):
            return f"line anchor #{fragment} exceeds the file's line count"
        return None

    def check_file(self, path: Path) -> list[tuple[int, str, str]]:
        text = self.text(path)
        if text is None:
            return [(0, "", "file is not readable UTF-8 text")]
        findings = []
        for number, raw in link_targets(text):
            problem = self.check_target(path, raw)
            if problem:
                findings.append((number, raw, problem))
        return findings


def default_files(root: Path) -> list[Path]:
    files = [root / name for name in ENTRY_FILES if (root / name).is_file()]
    files.extend(sorted(path for path in (root / DOCS_ROOT).rglob("*.md") if path.is_file()))
    return files


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("files", nargs="*", type=Path)
    arguments = parser.parse_args(argv)
    root = arguments.root.resolve()
    if not root.is_dir():
        print(f"repository root {root} is not a directory", file=sys.stderr)
        return 2
    files = [path if path.is_absolute() else root / path for path in arguments.files] or default_files(root)
    missing = [path for path in files if not path.is_file()]
    if missing:
        for path in missing:
            print(f"{path}: not a file", file=sys.stderr)
        return 2
    checker = Checker(root)
    findings = 0
    for path in files:
        relative = path.resolve().relative_to(root)
        for number, raw, problem in checker.check_file(path.resolve()):
            print(f"{relative}:{number}: {raw}: {problem}", file=sys.stderr)
            findings += 1
    print(f"documentation links: {len(files)} files checked, {findings} findings")
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
