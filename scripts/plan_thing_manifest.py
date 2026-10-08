"""D119: build a PlanThingOracle sample manifest from the backdrop census plan list.

Usage: python scripts/plan_thing_manifest.py SNAPSHOT BACKDROP-CENSUS NEW-MANIFEST
Writes a new JSON file under local-analysis/ only (refuses to overwrite).
"""
import json
import pathlib
import sys

PIN = "d6a2ef80b5a09999f08f23a803b516096055e5c9"
SCOPE = "plnb_toolkit_plan_things_v1"


def main(argv):
    if len(argv) != 4:
        raise SystemExit(__doc__)
    snapshot, census, out = (pathlib.Path(a) for a in argv[1:])
    allowed = pathlib.Path("local-analysis").resolve()
    if out.exists() or allowed not in out.resolve().parents:
        raise SystemExit("new manifest under local-analysis/ required")
    snap = json.loads(snapshot.read_text(encoding="utf-8"))
    by_guid = {r["map_row"]["guid"].lower(): r for r in snap["resources"]}
    plans = json.loads(census.read_text(encoding="utf-8"))["plans"]
    samples = []
    for p in sorted(plans, key=lambda p: p["guid"]):
        r = by_guid[p["guid"]]
        cands = r["archive_candidates"]
        if len(cands) != 1:
            raise SystemExit(f"{p['guid']}: {len(cands)} archive candidates")
        c = cands[0]
        samples.append({
            "guid": p["guid"],
            "map_path": r["map_row"]["path"],
            "sha1": r["map_row"]["sha1"],
            "source_path": c["archive"],
            "offset": c["offset"],
            "size": c["size"],
        })
    doc = {"toolkit_commit": PIN, "scope": SCOPE, "samples": samples}
    with open(out, "x", encoding="utf-8", newline="\n") as f:
        json.dump(doc, f, indent=1, sort_keys=True)
        f.write("\n")
    print(f"{len(samples)} plan samples")


if __name__ == "__main__":
    main(sys.argv)
