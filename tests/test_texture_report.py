"""Synthetic metadata only: parity failure and coverage must remain visible."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("texture_report", Path(__file__).parents[1] / "scripts/texture_report.py")
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)


class TextureReportTests(unittest.TestCase):
    def inputs(self):
        header = {"width": 4, "height": 4, "mips": 0, "fourcc": "DXT1", "rgb_bits": 0, "caps2": 0}
        census = {"toolkit_commit": "synthetic", "textures": [{"sha1": "a", "metadata": {"gtf": None, "dds": header}}]}
        manifest = {"samples": [{"sha1": "a"}]}
        oracle = {"toolkit_commit": "synthetic", "samples": [{"sha1": "a", "status": "decoded_top_mip", "dds": dict(header), "image_width": 4, "image_height": 4}]}
        return census, manifest, oracle

    def test_mismatch_is_reported(self):
        c, m, o = self.inputs()
        o["samples"][0]["image_width"] = 8
        self.assertEqual(report.verify(c, m, o)["mismatches"][0]["field"], "width")

    def test_failure_is_not_silently_dropped(self):
        c, m, o = self.inputs()
        o["samples"][0].update(status="failed_or_excluded", reason="synthetic")
        self.assertEqual(len(report.verify(c, m, o)["oracle_failures"]), 1)

    def test_missing_sample_and_changed_pin_fail(self):
        c, m, o = self.inputs()
        o["toolkit_commit"] = "wrong"
        with self.assertRaises(ValueError):
            report.verify(c, m, o)
        o["toolkit_commit"] = "synthetic"
        o["samples"] = []
        with self.assertRaises(ValueError):
            report.verify(c, m, o)

    def test_bc3_selection_is_deterministic_and_excludes_other_formats(self):
        rows = []
        for code in ("DXT1", "DXT5"):
            for size in (10, 30, 20):
                rows.append({"sha1": f"{code}-{size}", "size": size, "offset": 100,
                             "source_path": "synthetic.farc", "metadata": {
                                 "wrapper": "TEX ", "gtf": None,
                                 "chunks": {"expanded_bytes": 144},
                                 "dds": {"width": 4, "height": 4, "mips": 0,
                                         "fourcc": code, "caps2": 0}}})
        census = {"textures": rows}
        a = report.select_bc3(census)
        self.assertEqual([r["sha1"] for r in a["samples"]], ["DXT5-20"])
        self.assertEqual(a, report.select_bc3({"textures": list(reversed(rows))}))
        rows[-1]["metadata"]["dds"]["caps2"] = 0x200
        self.assertEqual(len(report.select_bc3(census)["samples"]), 1)


if __name__ == "__main__":
    unittest.main()
