#!/usr/bin/env python3
"""Payload and answered-question tracking checks for scripts/generate-atlas.py.

r[verify molten.project.system_atlas.payloads_and_tracking]
"""
import copy
import importlib.util
import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
SCRIPT = Path(__file__).with_name("generate-atlas.py")
SPEC = importlib.util.spec_from_file_location("generate_atlas", SCRIPT)
atlas_module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(atlas_module)

ATLAS = {
    "schema": atlas_module.SCHEMA,
    "title": "Fixture atlas",
    "summary": "Fixture.",
    "chapters": [
        {"id": "net", "title": "Net", "summary": "Net chapter.", "components": ["a", "b"], "questions": ["q"]},
    ],
    "components": [
        {"id": "a", "label": "A", "summary": "Sender.", "source": "src/a.rs", "chapter": "net"},
        {"id": "b", "label": "B", "summary": "Receiver.", "source": "src/b.rs", "chapter": "net"},
    ],
    "edges": [
        {
            "from": "a",
            "to": "b",
            "message": "frame",
            "chapter": "net",
            "payload": {"type": "Frame", "fields": ["peer", "bytes"], "source": "src/frame.rs"},
        },
    ],
    "questions": [
        {
            "id": "q",
            "text": "Is it gated?",
            "status": "answered",
            "link": "review.md",
            "tracking": [{"label": "issue #7", "link": "https://example.invalid/issues/7"}],
        },
    ],
}


class AtlasTests(unittest.TestCase):
    def test_payload_and_tracking_render_in_both_twins(self):
        markdown = atlas_module.markdown(ATLAS)
        self.assertIn("  - Payload: `Frame` {peer, bytes} in `src/frame.rs`", markdown)
        self.assertIn("tracked by [issue #7](https://example.invalid/issues/7)", markdown)
        document = atlas_module.html_document(ATLAS)
        self.assertIn('<li class="edge" tabindex="0">', document)
        self.assertIn('<div class="payload" role="note"><strong>Payload</strong> <code>Frame</code>', document)
        self.assertIn('<button type="button" class="pin" aria-pressed="false">pin</button>', document)
        self.assertIn(".edge:hover>.payload,.edge:focus-within>.payload,.edge.pinned>.payload{display:block}", document)
        self.assertIn('<a href="https://example.invalid/issues/7">issue #7</a>', document)

    def test_answered_question_without_tracking_is_rejected(self):
        atlas = copy.deepcopy(ATLAS)
        del atlas["questions"][0]["tracking"]
        with self.assertRaisesRegex(SystemExit, "answered question q must link"):
            atlas_module.validate(atlas)
        atlas["questions"][0]["status"] = "open"
        atlas_module.validate(atlas)

    def test_edge_without_payload_fields_is_rejected(self):
        atlas = copy.deepcopy(ATLAS)
        atlas["edges"][0]["payload"]["fields"] = []
        with self.assertRaisesRegex(SystemExit, "payload needs a non-empty list"):
            atlas_module.validate(atlas)
        del atlas["edges"][0]["payload"]
        with self.assertRaisesRegex(SystemExit, "needs a payload"):
            atlas_module.validate(atlas)


if __name__ == "__main__":
    unittest.main(verbosity=2)
