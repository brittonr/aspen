#!/usr/bin/env python3
"""Render the Molten system atlas from its single Nickel-sourced JSON value."""

from __future__ import annotations

import argparse
import html
import json
import pathlib
import sys
from typing import Any

SCHEMA = "molten-system-atlas.v1"


def fail(message: str) -> "NoReturn":
    raise SystemExit(message)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def load(path: pathlib.Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text())
    except OSError as error:
        fail(f"{path}: {error}")
    except json.JSONDecodeError as error:
        fail(f"{path}: invalid JSON: {error}")
    require(isinstance(value, dict), "atlas root must be an object")
    require(value.get("schema") == SCHEMA, f"atlas schema must be {SCHEMA}")
    for field in ("title", "summary"):
        require(isinstance(value.get(field), str) and value[field], f"atlas {field} must be a non-empty string")
    for field in ("chapters", "components", "edges", "questions"):
        require(isinstance(value.get(field), list), f"atlas {field} must be a list")
    return value


def unique(rows: list[dict[str, Any]], field: str, label: str) -> dict[str, dict[str, Any]]:
    index: dict[str, dict[str, Any]] = {}
    for row in rows:
        require(isinstance(row, dict), f"{label} entry must be an object")
        key = row.get(field)
        require(isinstance(key, str) and key, f"{label} entry needs a non-empty {field}")
        require(key not in index, f"duplicate {label} {field}: {key}")
        index[key] = row
    return index


def validate(atlas: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]], dict[str, dict[str, Any]]]:
    components = unique(atlas["components"], "id", "component")
    chapters = unique(atlas["chapters"], "id", "chapter")
    questions = unique(atlas["questions"], "id", "question")

    for component in components.values():
        for field in ("label", "summary", "source", "chapter"):
            require(isinstance(component.get(field), str) and component[field], f"component {component['id']} needs {field}")
        require(component["chapter"] in chapters, f"component {component['id']} names unknown chapter {component['chapter']}")

    for chapter in chapters.values():
        for field in ("title", "summary"):
            require(isinstance(chapter.get(field), str) and chapter[field], f"chapter {chapter['id']} needs {field}")
        require(isinstance(chapter.get("components"), list), f"chapter {chapter['id']} needs components")
        require(isinstance(chapter.get("questions"), list), f"chapter {chapter['id']} needs questions")
        for component in chapter["components"]:
            require(component in components, f"chapter {chapter['id']} names unknown component {component}")
        for question in chapter["questions"]:
            require(question in questions, f"chapter {chapter['id']} names unknown question {question}")

    for edge in atlas["edges"]:
        require(isinstance(edge, dict), "edge entry must be an object")
        for field in ("from", "to", "message", "chapter"):
            require(isinstance(edge.get(field), str) and edge[field], f"edge needs {field}")
        require(edge["from"] in components, f"edge names unknown source {edge['from']}")
        require(edge["to"] in components, f"edge names unknown target {edge['to']}")
        require(edge["chapter"] in chapters, f"edge names unknown chapter {edge['chapter']}")

    for question in questions.values():
        for field in ("text", "status", "link"):
            require(isinstance(question.get(field), str) and question[field], f"question {question['id']} needs {field}")
        require(
            question["status"] in {"open", "answered", "blocked", "deferred"},
            f"question {question['id']} has unsupported status {question['status']}",
        )
    return components, chapters, questions


def markdown(atlas: dict[str, Any]) -> str:
    components, chapters, questions = validate(atlas)
    lines = [
        "# " + atlas["title"],
        "",
        atlas["summary"],
        "",
        "<!-- Generated from docs/atlas.ncl by scripts/generate-atlas.py. Do not edit by hand. -->",
        "",
        "## Chapters",
        "",
    ]
    for chapter in atlas["chapters"]:
        lines.extend([f"### {chapter['title']}", "", chapter["summary"], "", "Components:"])
        for component in chapter["components"]:
            row = components[component]
            lines.append(f"- `{component}` — {row['label']}: {row['summary']} (`{row['source']}`)")
        chapter_edges = [edge for edge in atlas["edges"] if edge["chapter"] == chapter["id"]]
        if chapter_edges:
            lines.extend(["", "Messages:"])
            for edge in chapter_edges:
                lines.append(f"- `{edge['from']}` → `{edge['to']}`: {edge['message']}")
        chapter_questions = [questions[question] for question in chapter["questions"]]
        if chapter_questions:
            lines.extend(["", "Questions:"])
            for question in chapter_questions:
                lines.append(f"- **{question['status']}** {question['text']} ([ref]({question['link']}))")
        lines.append("")

    lines.extend(["## Components", "", "| Component | Role | Source | Chapter |", "|---|---|---|---|"])
    for component in atlas["components"]:
        lines.append(
            f"| `{component['id']}` | {component['label']}: {component['summary']} | "
            f"`{component['source']}` | {chapters[component['chapter']]['title']} |"
        )

    lines.extend(["", "## Edges", "", "| From | To | Message | Chapter |", "|---|---|---|---|"])
    for edge in atlas["edges"]:
        lines.append(f"| `{edge['from']}` | `{edge['to']}` | {edge['message']} | {chapters[edge['chapter']]['title']} |")

    lines.extend(["", "## Open questions", "", "| Status | Question | Reference |", "|---|---|---|"])
    for question in atlas["questions"]:
        lines.append(f"| {question['status']} | {question['text']} | [`{question['link']}`]({question['link']}) |")
    lines.append("")
    return "\n".join(lines)


def html_document(atlas: dict[str, Any]) -> str:
    components, chapters, questions = validate(atlas)
    escape = html.escape
    parts = [
        "<!doctype html>",
        '<html lang="en">',
        "<head>",
        '<meta charset="utf-8">',
        '<meta name="viewport" content="width=device-width, initial-scale=1">',
        f"<title>{escape(atlas['title'])}</title>",
        "<style>",
        "body{font:16px/1.5 system-ui,sans-serif;max-width:72rem;margin:2rem auto;padding:0 1rem;color:canvastext;background:canvas}",
        "code{font-size:.9em}details{margin:.75rem 0;padding:.5rem .75rem;border:1px solid color-mix(in srgb,canvastext 20%,transparent);border-radius:.4rem}",
        "summary{cursor:pointer;font-weight:600}table{border-collapse:collapse;width:100%}th,td{text-align:left;padding:.35rem .5rem;border-bottom:1px solid color-mix(in srgb,canvastext 15%,transparent);vertical-align:top}",
        "</style>",
        "</head>",
        "<body>",
        f"<h1>{escape(atlas['title'])}</h1>",
        f"<p>{escape(atlas['summary'])}</p>",
        "<p><em>Generated from <code>docs/atlas.ncl</code> by <code>scripts/generate-atlas.py</code>. Do not edit by hand.</em></p>",
        "<h2>Chapters</h2>",
    ]
    for chapter in atlas["chapters"]:
        parts.append("<details open>")
        parts.append(f"<summary>{escape(chapter['title'])}</summary>")
        parts.append(f"<p>{escape(chapter['summary'])}</p>")
        parts.append("<ul>")
        for component in chapter["components"]:
            row = components[component]
            parts.append(
                f"<li><code>{escape(component)}</code> — {escape(row['label'])}: {escape(row['summary'])} "
                f"(<code>{escape(row['source'])}</code>)</li>"
            )
        parts.append("</ul>")
        chapter_edges = [edge for edge in atlas["edges"] if edge["chapter"] == chapter["id"]]
        if chapter_edges:
            parts.append("<h3>Messages</h3><ul>")
            for edge in chapter_edges:
                parts.append(
                    f"<li><code>{escape(edge['from'])}</code> → <code>{escape(edge['to'])}</code>: "
                    f"{escape(edge['message'])}</li>"
                )
            parts.append("</ul>")
        chapter_questions = [questions[question] for question in chapter["questions"]]
        if chapter_questions:
            parts.append("<h3>Questions</h3><ul>")
            for question in chapter_questions:
                parts.append(
                    f"<li><strong>{escape(question['status'])}</strong> {escape(question['text'])} "
                    f'<a href="{escape(question["link"])}">ref</a></li>'
                )
            parts.append("</ul>")
        parts.append("</details>")

    parts.extend(["<h2>Components</h2>", "<table><thead><tr><th>Component</th><th>Role</th><th>Source</th><th>Chapter</th></tr></thead><tbody>"])
    for component in atlas["components"]:
        parts.append(
            f"<tr><td><code>{escape(component['id'])}</code></td>"
            f"<td>{escape(component['label'])}: {escape(component['summary'])}</td>"
            f"<td><code>{escape(component['source'])}</code></td>"
            f"<td>{escape(chapters[component['chapter']]['title'])}</td></tr>"
        )
    parts.extend(["</tbody></table>", "<h2>Edges</h2>", "<table><thead><tr><th>From</th><th>To</th><th>Message</th><th>Chapter</th></tr></thead><tbody>"])
    for edge in atlas["edges"]:
        parts.append(
            f"<tr><td><code>{escape(edge['from'])}</code></td><td><code>{escape(edge['to'])}</code></td>"
            f"<td>{escape(edge['message'])}</td><td>{escape(chapters[edge['chapter']]['title'])}</td></tr>"
        )
    parts.extend(["</tbody></table>", "<h2>Open questions</h2>", "<table><thead><tr><th>Status</th><th>Question</th><th>Reference</th></tr></thead><tbody>"])
    for question in atlas["questions"]:
        parts.append(
            f"<tr><td>{escape(question['status'])}</td><td>{escape(question['text'])}</td>"
            f'<td><a href="{escape(question["link"])}"><code>{escape(question["link"])}</code></a></td></tr>'
        )
    parts.extend(["</tbody></table>", "</body>", "</html>", ""])
    return "\n".join(parts)


def write_or_check(path: pathlib.Path, content: str, check: bool) -> bool:
    if check:
        try:
            existing = path.read_text()
        except OSError as error:
            fail(f"{path}: {error}")
        if existing != content:
            print(f"{path} is stale; run scripts/generate-atlas.py", file=sys.stderr)
            return False
        return True
    path.write_text(content)
    return True


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=pathlib.Path, help="JSON exported from docs/atlas.ncl")
    parser.add_argument("--markdown", type=pathlib.Path, default=pathlib.Path("docs/atlas.md"))
    parser.add_argument("--html", type=pathlib.Path, default=pathlib.Path("docs/atlas.html"))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    atlas = load(args.source)
    ok = write_or_check(args.markdown, markdown(atlas), args.check)
    ok = write_or_check(args.html, html_document(atlas), args.check) and ok
    if not ok:
        raise SystemExit(1)
    if not args.check:
        print(f"wrote {args.markdown} and {args.html}")


if __name__ == "__main__":
    main()
