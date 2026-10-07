"""Synthetic metadata recipes; no game data or original fixtures."""
import copy
import unittest
from test_material_coverage_samples import recipe
from material_shape_samples import select, coverage


def inputs(n=3):
    census, baseline = recipe(n)
    baseline["schema_version"] = 1
    for row in census["locations"]:
        row["stages"]["tail"]["component_value_counts"] = [row["offset"] + 1]
    return census, baseline


def oracle(row):
    return {"sha1": row["sha1_candidates"][0], "status": "parsed_graph", "tail_status": "parsed_tail",
            "graph": {k: row["stages"]["graph"][k] for k in ("box_count", "wire_count")},
            "tail": {"parameter_animations": [{"component_value_count": n}
                     for n in row["stages"]["tail"]["component_value_counts"]]}}


class ShapeSamplesTests(unittest.TestCase):
    def test_determinism_boundary_and_union_coverage(self):
        c, b = inputs()
        b["samples"] = [oracle(c["locations"][0])]
        extra = copy.deepcopy(c["locations"][0])
        extra["sha1_candidates"] = ["f" * 40]
        extra["offset"] = 999
        extra["stages"]["tail"]["component_value_counts"] = [1143]
        c["locations"].append(extra)
        before = copy.deepcopy(c)
        selected = select(c, [b])
        c["locations"].reverse()
        self.assertEqual(selected, select(c, [b]))
        self.assertEqual(len(selected["samples"]), 3)
        self.assertEqual(selected["target_component_value_maximum"], 1143)
        self.assertEqual(before["locations"][-1], extra)
        followup = copy.deepcopy(b)
        followup["samples"] = [oracle(r) for r in c["locations"]]
        self.assertEqual(coverage(c, [b, followup])["covered_graph_shapes"], 3)
        with self.assertRaises(ValueError): coverage(c, [b])

    def test_conflicting_aliases_and_reports_fail_explicitly(self):
        c, b = inputs(1)
        alias = copy.deepcopy(c["locations"][0])
        alias["offset"] = 300
        c["locations"].append(alias)
        self.assertEqual(len(select(c, [b])["samples"]), 1)
        alias["stages"]["tail"]["component_value_counts"] = [17]
        with self.assertRaises(ValueError): select(c, [b])
        c, b = inputs(1)
        b["samples"] = [oracle(c["locations"][0])]
        other = copy.deepcopy(b)
        other["samples"][0]["graph"]["wire_count"] = 9
        with self.assertRaises(ValueError): select(c, [b, other])
        b["toolkit_commit"] = "wrong"
        with self.assertRaises(ValueError): select(c, [b])

    def test_budgets_and_failed_location_contaminate_identifier(self):
        for n in (1, 49):
            c, b = inputs(n)
            if n == 1: c["locations"][0]["size"] = 1024**2 + 1
            with self.assertRaises(ValueError): select(c, [b])
        c, b = inputs(17)
        for r in c["locations"]: r["size"] = 1024**2
        with self.assertRaises(ValueError): select(c, [b])
        c, b = inputs(1)
        alias = copy.deepcopy(c["locations"][0])
        alias["status"] = "blocked_protected_no_read"
        c["locations"].append(alias)
        with self.assertRaises(ValueError): select(c, [b])


if __name__ == "__main__": unittest.main()
