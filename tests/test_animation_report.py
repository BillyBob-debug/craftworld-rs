"""Authored synthetic catalog metadata; no proprietary fixture bytes."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("animation_report", Path(__file__).parents[1] / "scripts/animation_report.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def location(size, revision, kind="animation", offset=0):
    return {"source_path": "USRDIR/DATA.FARC", "offset": offset, "size": size, "sha1": f"{offset:040x}",
            "inspection": {"confirmed_type": {"name": kind}, "magic_hex": "414e4d62", "header":
                           {"revision": revision, "branch_id": 0, "branch_revision": 0,
                            "compression_flags": 7, "compressed": True}}}


class CensusTests(unittest.TestCase):
    def test_aliases_are_distinct_from_confirmed_locations_and_selection_is_deterministic(self):
        rows = [location(40, 891, offset=1), location(80, 891, offset=2), location(60, 891, offset=3), location(50, 844, offset=4)]
        snapshot = {"resources": [{"map_row": {"path": "authored/a.anim", "sha1": rows[0]["sha1"]}}] * 2,
                    "resource_inspections": rows + [location(50, 891, kind="mesh", offset=5)]}
        a, selected = module.census(snapshot)
        self.assertEqual((a["extension_alias_rows"], a["extension_unique_hashes"], a["confirmed_locations"]), (2, 1, 4))
        self.assertEqual({s["offset"] for s in selected["samples"]}, {1, 2, 4})
        snapshot["resource_inspections"].reverse()
        self.assertEqual((a, selected), module.census(snapshot))

    def test_out_of_budget_locations_are_counted_without_becoming_samples(self):
        report, manifest = module.census({"resources": [], "resource_inspections": [location(3*1024**2, 891)]})
        self.assertEqual(report["confirmed_locations"], 1)
        self.assertEqual(manifest["samples"], [])

    def test_excess_cohorts_require_a_refined_selection_and_output_stays_private(self):
        with self.assertRaisesRegex(ValueError, "sample count"):
            module.census({"resources": [], "resource_inspections": [location(40, i, offset=i) for i in range(49)]})
        with self.assertRaises(ValueError):
            module.write("docs/authored-not-an-output.json", {})


if __name__ == "__main__":
    unittest.main()
