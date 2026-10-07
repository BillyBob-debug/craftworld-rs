"""Authored diagnostic metadata shapes; no original fixtures or shader bytes."""
import copy
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from material_graph_report import PIN, summarize


def recipe():
    prefix = {"parsed_prefix_bytes": 20}
    common = {"sha1": "synthetic", "prefix": prefix, "compression_flags": 7, "expanded_bytes": 100}
    previous = {"toolkit_commit": PIN, "schema_version": 1, "scope": "PS3_GMT_prefix_only_no_shader_bytes",
                "samples": [dict(common, status="parsed_prefix")]}
    anim = {"component_value_count": 0, "components_raw": 255, "derived_key_count": 0}
    box = {"type_raw": 1, "declared_parameter_count": 8, "parameters_raw": [0,0,0,0,0,99,0,0],
           "position_bits": [0]*4, "animations": [anim, copy.deepcopy(anim)], "texture_slot_raw": 99,
           "texture_slot_in_range": False}
    wire = {"box_from": -1, "box_to": 0, "from_in_range": False, "to_in_range": True,
            "port_from_raw": 255, "port_to_raw": 254, "swizzle_raw": [255]*5}
    graph = {"graph_start": 20, "parsed_graph_bytes": 70, "unparsed_bytes": 10, "box_count": 1,
             "wire_count": 1, "boxes": [box], "wires": [wire]}
    actual = {"toolkit_commit": PIN, "schema_version": 1, "scope": "PS3_GMT_graph_metadata_no_shader_bytes",
              "samples": [dict(common, status="parsed_graph", graph=graph)]}
    return actual, previous


class GraphReportTests(unittest.TestCase):
    def test_raw_unknowns_out_of_range_links_and_determinism(self):
        actual, previous = recipe(); before = copy.deepcopy(actual)
        r = summarize(actual, previous)
        self.assertEqual(r["out_of_range_slots"], 1); self.assertEqual(r["out_of_range_wire_records"], 1)
        self.assertEqual(r["wire_port_pair_counts"], {"255:254": 1}); self.assertEqual(actual, before)

    def test_reject_inconsistent_fields(self):
        for mutate in (lambda g: g.update(unparsed_bytes=-1), lambda g: g.update(box_count=2),
                       lambda g: g["boxes"][0].update(texture_slot_raw=1),
                       lambda g: g["wires"][0].update(from_in_range=True),
                       lambda g: g["boxes"][0]["animations"][0].update(component_value_count=1)):
            a, p = recipe(); mutate(a["samples"][0]["graph"])
            with self.assertRaises(ValueError): summarize(a, p)

    def test_reject_changed_prefix_pins_and_duplicate_samples(self):
        a, p = recipe(); a["samples"][0]["compression_flags"] = 0
        with self.assertRaises(ValueError): summarize(a, p)
        a, p = recipe(); a["toolkit_commit"] = "wrong"
        with self.assertRaises(ValueError): summarize(a, p)
        a, p = recipe(); a["samples"].append(copy.deepcopy(a["samples"][0]))
        with self.assertRaises(ValueError): summarize(a, p)


if __name__ == "__main__": unittest.main()
