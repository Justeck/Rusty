"""Regression checks for task-owned source preparation and source-cache reuse."""
import importlib.util
import io
import pathlib
import shutil
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("native_build", pathlib.Path(__file__).with_name("native.py"))
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)

class SourceCacheTests(unittest.TestCase):
    def setUp(self):
        (native.ROOT / "target").mkdir(exist_ok=True)
        self.scratch = tempfile.TemporaryDirectory(prefix="source-cache-test-", dir=native.ROOT / "target")
        self.addCleanup(self.scratch.cleanup)
        self.previous = native.TARGET
        native.TARGET = pathlib.Path(self.scratch.name) / "target"
        self.addCleanup(setattr, native, "TARGET", self.previous)
        native.TARGET.mkdir()
        self.revision = "2" * 40
        manifest = patch.dict(native.MANIFEST, {"horizon": self.revision})
        manifest.start()
        self.addCleanup(manifest.stop)
        self.archive = pathlib.Path(self.scratch.name) / "source.tar.gz"
        with tarfile.open(self.archive, "w:gz") as tar:
            for name, data in {"upstream/Cargo.toml": b"[workspace]\nmembers=[]\n", "upstream/fresh.rs": b"// upstream source\n"}.items():
                item = tarfile.TarInfo(name)
                item.size = len(data)
                tar.addfile(item, io.BytesIO(data))

    def download(self, url, destination):
        self.assertIn(native.MANIFEST["horizon"], url)
        shutil.copyfile(self.archive, destination)

    def test_partial_cached_tree_is_replaced_then_completed_tree_is_reused(self):
        partial = native.TARGET / ("horizon-source-" + self.revision)
        partial.mkdir()
        (partial / "stale.rs").write_text("partial cache")
        with patch.object(native.urllib.request, "urlretrieve", side_effect=self.download) as download:
            prepared = native.source()
            self.assertEqual(prepared, partial)
            self.assertTrue((prepared / "fresh.rs").is_file())
            self.assertFalse((prepared / "stale.rs").exists())
            self.assertEqual((prepared / ".horizon-revision").read_text().strip(), native.MANIFEST["horizon"])
            self.assertEqual(native.source(), prepared)
            self.assertEqual(download.call_count, 1)

    def test_corrupt_marker_in_matching_revision_directory_is_preserved_and_rejected(self):
        prepared = native.TARGET / ("horizon-source-" + self.revision)
        prepared.mkdir()
        marker = prepared / ".horizon-revision"
        marker.write_text("another-revision")
        with patch.object(native.urllib.request, "urlretrieve", side_effect=self.download) as download:
            with self.assertRaises(AssertionError):
                native.source()
            download.assert_not_called()
        self.assertEqual(marker.read_text(), "another-revision")

    def test_upgrade_prepares_and_reuses_new_revision_without_replacing_old_cache(self):
        old_revision = "1" * 40
        with patch.object(native.urllib.request, "urlretrieve", side_effect=self.download) as download:
            with patch.dict(native.MANIFEST, {"horizon": old_revision}):
                old = native.source()
                self.assertEqual(old, native.TARGET / ("horizon-source-" + old_revision))
                sentinel = old / "keep-local-build-state"
                sentinel.write_bytes(b"preserve old revision artifacts\n")
                old_files = {p.relative_to(old): p.read_bytes() for p in old.rglob("*") if p.is_file()}
            upgraded = native.source()
            self.assertEqual(upgraded, native.TARGET / ("horizon-source-" + self.revision))
            self.assertNotEqual(upgraded, old)
            self.assertEqual((upgraded / ".horizon-revision").read_text().strip(), self.revision)
            self.assertTrue((upgraded / "fresh.rs").is_file())
            self.assertFalse((upgraded / "keep-local-build-state").exists())
            self.assertEqual(native.source(), upgraded)
            with patch.dict(native.MANIFEST, {"horizon": old_revision}):
                self.assertEqual(native.source(), old)
            self.assertEqual({p.relative_to(old): p.read_bytes() for p in old.rglob("*") if p.is_file()}, old_files)
            self.assertEqual(download.call_count, 2)

    def test_legacy_unqualified_cache_is_preserved_when_new_pin_is_prepared(self):
        legacy = native.TARGET / "horizon-source"
        legacy.mkdir()
        (legacy / ".horizon-revision").write_text("9aa4be30f8ebde1b681843000d046306b9b6b957\n")
        (legacy / "original.rs").write_bytes(b"legacy source remains intact\n")
        before = {p.name: p.read_bytes() for p in legacy.iterdir()}
        with patch.object(native.urllib.request, "urlretrieve", side_effect=self.download) as download:
            prepared = native.source()
            self.assertEqual(prepared, native.TARGET / ("horizon-source-" + self.revision))
            self.assertEqual(native.source(), prepared)
            self.assertEqual(download.call_count, 1)
        self.assertEqual({p.name: p.read_bytes() for p in legacy.iterdir()}, before)

if __name__ == "__main__":
    unittest.main()
