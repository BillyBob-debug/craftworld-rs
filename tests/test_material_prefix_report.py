"""Authored metadata-only oracle consistency cases; no copyrighted fixtures."""
import copy
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from material_prefix_report import PIN, summarize


def recipe():
    textures = [{"kind": 0, "guid": None, "sha1": None, "flags": 0} for _ in range(8)]
    textures[0] = {"kind": 3, "guid": 17, "sha1": "a" * 40, "flags": 0}
    textures[1] = {"kind": 2, "guid": 0, "sha1": None, "flags": 0}
    prefix = {"revision": 950, "parsed_prefix_bytes": 100, "unparsed_bytes": 10,
              "shader_count": 2, "shader_offset_count": 2, "shader_end_offsets": [2, 4],
              "blob_bytes": 6, "blob_start": 20, "uninterpreted_after_shader_bytes": 2,
              "textures": textures, "wrap_s_raw": [1, 255], "wrap_t_raw": [4]}
    rows = {"toolkit_commit": PIN, "schema_version": 1, "scope": "PS3_GMT_prefix_only_no_shader_bytes",
            "samples": [{"sha1": "synthetic", "status": "parsed_prefix", "compression_flags": 7,
                         "expanded_bytes": 110, "prefix": prefix}]}
    headers = {"toolkit_commit": PIN, "samples": [{"sha1": "synthetic", "revision": 950,
               "compression_flags": 7, "type_id": 7, "magic": "GMTb", "branch_id": 0,
               "dependencies": [{"guid": "00000011", "sha1": "a" * 40, "type_id": 1}]}]}
    return rows, headers


class MaterialPrefixReportTests(unittest.TestCase):
    def test_raw_null_both_identity_unknown_wrap_preserved(self):
        rows, headers = recipe()
        original = copy.deepcopy(rows)
        report = summarize(rows, headers)
        self.assertEqual(report["matched_normalized_texture_references"], 1)
        self.assertEqual(report["raw_descriptor_kind_counts"], {"0": 6, "2": 1, "3": 1})
        self.assertEqual(report["raw_wrap_value_counts"]["255"], 1)
        self.assertEqual(rows, original)

    def test_reject_inconsistent_boundaries_presence_and_deps(self):
        for mutation in (lambda p: p.update(shader_end_offsets=[4, 2]),
                         lambda p: p.update(unparsed_bytes=-1),
                         lambda p: p["textures"][0].update(kind=1),
                         lambda p: p["textures"][0].update(guid=18)):
            rows, headers = recipe()
            mutation(rows["samples"][0]["prefix"])
            with self.assertRaises(ValueError): summarize(rows, headers)

    def test_reject_wrong_pin_duplicate_and_missing_samples(self):
        rows, headers = recipe()
        rows["toolkit_commit"] = "wrong"
        with self.assertRaises(ValueError): summarize(rows, headers)
        rows, headers = recipe()
        rows["samples"].append(copy.deepcopy(rows["samples"][0]))
        with self.assertRaises(ValueError): summarize(rows, headers)
        rows, headers = recipe()
        headers["samples"].clear()
        with self.assertRaises(ValueError): summarize(rows, headers)


if __name__ == "__main__": unittest.main()
