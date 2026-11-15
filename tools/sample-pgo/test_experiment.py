#!/usr/bin/env python3
"""Consumer tests. Supply the real Aspen CLI (`molten`), never a stand-in executable."""

import json
import os
from pathlib import Path
import tempfile
import unittest

import experiment


POLICY = json.loads((Path(__file__).parents[2] / "config/sample-pgo/experiment.json").read_text())


# r[verify molten.sample_pgo.measurement]
# r[verify molten.sample_pgo.non_claim]
class DecisionTests(unittest.TestCase):
    def pairs(self, ratio=0.9, rss=100):
        return [{case: {"baseline": {"elapsed_seconds": 1.0, "max_rss_kib": 100,
                                    "user_seconds": 0.5, "system_seconds": 0.1},
                        "candidate": {"elapsed_seconds": ratio, "max_rss_kib": rss,
                                      "user_seconds": 0.4, "system_seconds": 0.1}}
                 for case in ("success", "denied", "exhausted", "error")} for _ in range(30)]

    def test_predeclared_threshold_rejects_training_only_gain(self):
        report = experiment.summarize(self.pairs(1.0), {"baseline": 100, "candidate": 100}, POLICY)
        self.assertEqual(report["decision"], "stop")

    def test_measured_gain_is_insufficient_when_memory_regresses(self):
        report = experiment.summarize(self.pairs(rss=110), {"baseline": 100, "candidate": 100}, POLICY)
        self.assertEqual(report["decision"], "revise")

    def test_eligible_gain_keeps_only_the_experiment(self):
        report = experiment.summarize(self.pairs(), {"baseline": 100, "candidate": 100}, POLICY)
        self.assertEqual(report["decision"], "keep")
        self.assertIn("not release admission", report["scope"])

    def test_uncertain_gain_never_claims_keep(self):
        pairs = self.pairs(0.95)
        for index, pair in enumerate(pairs):
            for row in pair.values():
                row["candidate"]["elapsed_seconds"] += 0.04 if index % 2 else -0.04
        report = experiment.summarize(pairs, {"baseline": 100, "candidate": 100}, POLICY)
        self.assertEqual(report["decision"], "inconclusive")

    def test_empty_or_non_application_profile_has_no_admitted_samples(self):
        self.assertEqual(experiment.profile_samples(""), 0)
        self.assertEqual(experiment.profile_samples("libc:100:1\n 1:100\n"), 0)
        self.assertEqual(experiment.profile_samples("_RNv6molten3run:120:1\n 1:120\n"), 120)

    def test_associations_exclude_unidentified_temporary_names(self):
        original = "abcd /nix/store/baseline/bin/molten\n     /tmp/perf-vdso.so-first\n"
        regenerated = "abcd /nix/store/baseline/bin/molten\n     /tmp/perf-vdso.so-second\n"
        self.assertEqual(experiment.build_id_associations(original), [("abcd", "/nix/store/baseline/bin/molten")])
        self.assertEqual(experiment.build_id_associations(original), experiment.build_id_associations(regenerated))

    def test_associations_preserve_build_ids_and_exact_paths(self):
        original = experiment.build_id_associations("abcd /nix/store/baseline/bin/molten\n")
        for changed in ("abce /nix/store/baseline/bin/molten\n", "abcd /nix/store/other/bin/molten\n", ""):
            with self.subTest(changed=changed):
                self.assertNotEqual(original, experiment.build_id_associations(changed))

    def test_associations_keep_unidentified_real_paths_and_identified_vdso(self):
        self.assertEqual(experiment.build_id_associations("     /nix/store/library.so\n"),
                         [("", "/nix/store/library.so")])
        self.assertEqual(experiment.build_id_associations("abcd /tmp/perf-vdso.so-first\n"),
                         [("abcd", "/tmp/perf-vdso.so-first")])
        self.assertNotEqual(experiment.build_id_associations("     /nix/store/library.so\n"),
                            experiment.build_id_associations("     /nix/store/other.so\n"))

    def test_associations_reject_malformed_or_conflicting_rows(self):
        for text in ("not-a-build-id /nix/store/library.so\n", "abc /nix/store/library.so\n",
                     "abcd relative-path\n", "missing-path\n",
                     "abcd /nix/store/library.so\nabce /nix/store/library.so\n",
                     "     /nix/store/library.so\nabcd /nix/store/library.so\n"):
            with self.subTest(text=text), self.assertRaises(experiment.Blocked):
                experiment.build_id_associations(text)


# r[verify molten.sample_pgo.replay]
# r[verify molten.sample_pgo.cohort]
# r[verify molten.sample_pgo.capture]
class NativeConsumerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = Path(os.environ["MOLTEN_SAMPLE_PGO_BINARY"]).resolve(strict=True)

    def setUp(self):
        self.workspace = tempfile.TemporaryDirectory(prefix="molten-sample-pgo-consumer-")
        self.addCleanup(self.workspace.cleanup)
        self.root = Path(self.workspace.name)
        self.paths = experiment.datasets(POLICY, self.root / "datasets")

    def test_training_and_held_out_inputs_are_distinct(self):
        self.assertNotEqual(self.paths["training"].read_bytes(), self.paths["success"].read_bytes())
        self.assertNotIn("training-payload", self.paths["success"].read_text())
        self.assertNotIn("held-out-payload", self.paths["training"].read_text())

    def test_real_four_path_canonical_artifacts_and_cross_replay(self):
        rows = experiment.equivalence(self.binary, self.binary, self.paths, self.root / "semantics", 30)
        self.assertEqual(set(rows), {"success", "denied", "exhausted", "error"})
        self.assertTrue(all(row["canonical_equal"] for row in rows.values()))
        for case in ("success", "denied"):
            self.assertEqual(rows[case]["baseline"]["returncode"], 0)
            self.assertEqual(rows[case]["replay"], "passed")
        for case in ("exhausted", "error"):
            self.assertEqual(rows[case]["baseline"]["returncode"], 1)

    def test_owner_replay_rejects_changed_canonical_artifact(self):
        _, report = experiment.run_case(self.binary, "success", self.paths["success"], self.root / "record", 30)
        changed = self.root / "changed.preserves"
        text = report.read_text()
        self.assertIn("held-out-payload", text)
        changed.write_text(text.replace("held-out-payload", "tampered-payload", 1))
        row, _ = experiment.invoke([self.binary, "test", "replay", changed], self.root, "reject-tamper", 30,
                                   expected_success=False)
        self.assertNotEqual(row["returncode"], 0)

    def test_missing_workload_does_not_become_measurement(self):
        with self.assertRaises(experiment.Blocked):
            experiment.run_case(self.binary, "success", self.root / "absent.preserves", self.root / "missing", 30)

    def test_profile_binding_rejects_a_different_real_baseline(self):
        bundle = self.root / "stale-profile"
        bundle.mkdir()
        experiment.save(bundle / "binding.json", {"baseline_sha256": "0" * 64})
        with self.assertRaisesRegex(experiment.Blocked, "stale baseline_sha256"):
            experiment.validate_profile(bundle, self.binary, self.binary, self.root / "validate-stale", 100)


if __name__ == "__main__":
    unittest.main()
