"""Authored metadata selection, no original fixtures."""
import copy
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from material_coverage_samples import PIN, select


def recipe(n=40):
    rows = [{"status": "stage_inspection_completed", "sha1_candidates": [f"{i:040x}"],
             "source_path": "authored.farc", "offset": i*100, "size": 80,
             "stages": {"prefix": {"status": "parsed", "revision": 950},
                        "graph": {"status": "parsed", "box_count": i+1, "wire_count": i},
                        "tail": {"status": "parsed", "sound_enum_raw": i, "parameter_animation_count": i%3}}} for i in range(n)]
    return {"schema_version": 1, "scope": "selected_snapshot_material_metadata_coverage_not_full_game",
            "counts": {"cancelled": False}, "locations": rows}, {"toolkit_commit": PIN, "scope": "PS3_GMT_tail_metadata_no_shader_bytes", "samples": []}


class CoverageSamplesTests(unittest.TestCase):
    def test_deterministic_order_budget_and_uncovered_features(self):
        c,b=recipe(60); before=copy.deepcopy(c); a=select(c,b)
        c["locations"].reverse(); self.assertEqual(a,select(c,b))
        self.assertEqual(len(a["samples"]),40); self.assertTrue(a["uncovered_primary_features"])
        self.assertEqual(before["locations"][0]["size"],80)

    def test_conflicts_failures_and_incomplete_inputs_not_silently_selected(self):
        c,b=recipe(3); c["locations"][0]["sha1_candidates"].append("f"*40)
        c["locations"][1]["status"]="blocked_protected_no_read"
        a=select(c,b); self.assertEqual(a["excluded_locations"],2); self.assertEqual(len(a["samples"]),1)
        c["counts"]["cancelled"]=True
        with self.assertRaises(ValueError): select(c,b)
        c,b=recipe(); b["toolkit_commit"]="wrong"
        with self.assertRaises(ValueError): select(c,b)


if __name__ == "__main__": unittest.main()
