"""Regression checks for the revision-scoped, test-only SDK correction."""
import hashlib
import pathlib
import tempfile
import unittest
from unittest.mock import patch
import upstream_test_fixture as fixture

class UpstreamFixtureTests(unittest.TestCase):
    def setUp(self):
        self.source = ("// unchanged production code\n#[cfg(test)]\nmod tests {\n"
                       "fn a_composite_program_blends_onto_its_buffer() {\n" + fixture.OLD + "}\n}\n")
        self.corrected = self.source.replace(fixture.OLD, fixture.NEW, 1)
        self.patches = patch.multiple(fixture,
            BEFORE=hashlib.sha256(self.source.encode()).hexdigest(),
            AFTER=hashlib.sha256(self.corrected.encode()).hexdigest())
        self.patches.start()
        self.addCleanup(self.patches.stop)

    def test_only_test_initializer_changes_and_second_application_is_identical(self):
        corrected = fixture.patch_text(self.source)
        self.assertEqual(corrected, self.corrected)
        self.assertEqual(corrected.split("#[cfg(test)]")[0], self.source.split("#[cfg(test)]")[0])
        self.assertEqual(fixture.patch_text(corrected), corrected)

    def test_unknown_source_is_rejected_without_guessing(self):
        with self.assertRaises(ValueError):
            fixture.patch_text(self.source + "// changed SDK\n")

    def test_latest_sdk_with_upstream_fix_is_validated_without_modification(self):
        with tempfile.TemporaryDirectory() as tmp:
            stage = pathlib.Path(tmp) / "stage"
            source = stage / fixture.REL
            source.parent.mkdir(parents=True)
            source.write_bytes(self.corrected.encode())
            (stage / ".horizon-revision").write_text(fixture.FIXED_PIN)
            with patch.object(fixture, "FIXED_HASH", hashlib.sha256(self.corrected.encode()).hexdigest()):
                receipt = fixture.apply_stage(stage, fixture.FIXED_PIN)
                self.assertEqual(receipt["change"], "none; upstream fixture already fixed")
                self.assertEqual(source.read_bytes(), self.corrected.encode())
                source.write_bytes(self.source.encode())
                with self.assertRaises(ValueError):
                    fixture.apply_stage(stage, fixture.FIXED_PIN)
                self.assertEqual(source.read_bytes(), self.source.encode())

    def test_stage_pin_and_path_rejections_preserve_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            stage = pathlib.Path(tmp) / "stage"
            source = stage / fixture.REL
            source.parent.mkdir(parents=True)
            source.write_bytes(self.source.encode())
            (stage / ".horizon-revision").write_text(fixture.PIN)
            with self.assertRaises(ValueError):
                fixture.apply_stage(stage, "another-pin")
            self.assertEqual(source.read_bytes(), self.source.encode())
            fixture.apply_stage(stage, fixture.PIN)
            self.assertEqual(source.read_bytes(), self.corrected.encode())
            fixture.apply_stage(stage, fixture.PIN)
            self.assertEqual(source.read_bytes(), self.corrected.encode())
            pristine = pathlib.Path(tmp) / "horizon-source"
            pristine.mkdir()
            (pristine / ".horizon-revision").write_text(fixture.PIN)
            with self.assertRaises(ValueError):
                fixture.apply_stage(pristine, fixture.PIN)

if __name__ == "__main__":
    unittest.main()
