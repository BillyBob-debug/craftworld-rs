"""Authored metadata only, no original fixtures."""
import copy
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from material_tail_report import PIN, summarize
from test_material_graph_report import recipe as graph_recipe


def recipe():
    g, p = graph_recipe()
    t = copy.deepcopy(g); t["scope"] = "PS3_GMT_tail_metadata_no_shader_bytes"
    t["samples"][0].update(tail_status="parsed_tail", tail={"sound_enum_raw": 0xffffffff,
        "tail_start": 90, "parsed_tail_bytes": 8, "unparsed_bytes": 2, "parameter_animation_count": 0,
        "parameter_animations": []})
    return t, g, p


class TailReportTests(unittest.TestCase):
    def test_unknown_sound_trailing_bytes_and_determinism(self):
        t, g, p = recipe(); before = copy.deepcopy(t)
        a = summarize(t, g, p)
        self.assertEqual(a, summarize(t, g, p)); self.assertEqual(t, before)
        self.assertEqual(a["sound_enum_counts"], {"4294967295": 1})
        self.assertEqual(a["unparsed_tail_counts"], {"2": 1})

    def test_rejected_tail_retains_graph_prefix(self):
        t, g, p = recipe(); del t["samples"][0]["tail"]
        t["samples"][0].update(tail_status="rejected", tail_error="authored unsupported")
        a = summarize(t, g, p)
        self.assertEqual(a["matched_graph_prefix_samples"], 1)
        self.assertEqual(a["parsed_tail_samples"], 0)

    def test_inconsistent_fields_pins_and_duplicates_rejected(self):
        for change in (lambda t: t["samples"][0]["tail"].update(tail_start=89),
                       lambda t: t["samples"][0]["tail"].update(unparsed_bytes=3),
                       lambda t: t["samples"][0]["tail"].update(parameter_animation_count=1),
                       lambda t: t.update(toolkit_commit="wrong"),
                       lambda t: t["samples"].append(copy.deepcopy(t["samples"][0]))):
            t, g, p = recipe(); change(t)
            with self.assertRaises(ValueError): summarize(t, g, p)

    def test_animation_shapes_counts_masks_and_raw_bits(self):
        t, g, p = recipe()
        # Authored compact metadata: sound/count two bytes; animation 38 bytes.
        g["samples"][0]["expanded_bytes"] = 132
        p["samples"][0]["expanded_bytes"] = 132
        g["samples"][0]["graph"]["unparsed_bytes"] = 42
        t["samples"][0]["expanded_bytes"] = 132
        t["samples"][0]["graph"]["unparsed_bytes"] = 42
        t["samples"][0]["tail"].update(parsed_tail_bytes=40, unparsed_bytes=2, sound_enum_raw=0)
        a = {"base_value_bits": [0x7fc00001, 0x80000000, 0, 0], "component_value_count": 4,
             "components_raw": 5, "derived_key_count": 2, "name_hex": "41005a", "component_bytes_sha256": "a"*64}
        t["samples"][0]["tail"].update(parameter_animation_count=1, parameter_animations=[a])
        self.assertEqual(summarize(t, g, p)["derived_key_total"], 2)
        for change in (lambda a: a.update(components_raw=0), lambda a: a.update(derived_key_count=3),
                       lambda a: a.update(name_hex="0"), lambda a: a.update(component_bytes_sha256="wrong")):
            bad = copy.deepcopy(t); change(bad["samples"][0]["tail"]["parameter_animations"][0])
            with self.assertRaises(ValueError): summarize(bad, g, p)


if __name__ == "__main__": unittest.main()
