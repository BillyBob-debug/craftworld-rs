"""Authored material metadata cohorts; no game reads."""
import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).parents[1]/"scripts"))
from material_report import select

class MaterialReportTests(unittest.TestCase):
    def test_selection_scope_and_order(self):
        rows=[]
        for sha,size,kind in [("b",30,7),("a",16,7),("c",20,7),("d",1,2)]:
            rows.append({"sha1":sha,"size":size,"offset":0,"source_path":"synthetic",
                "inspection":{"confirmed_type":{"id":kind},"status":"metadata_parsed",
                    "header":{"revision":950,"compression_flags":7,"branch_id":0},
                    "dependencies":[{"type_id":1}]}})
        a=select({"resource_inspections":rows})
        self.assertEqual(a["known_material_locations"],3)
        self.assertEqual(a["dependency_type_counts"],{"1":3})
        self.assertEqual([r["sha1"] for r in a["samples"]],["a","c"])
        self.assertEqual(a,select({"resource_inspections":list(reversed(rows))}))

if __name__=="__main__":unittest.main()
