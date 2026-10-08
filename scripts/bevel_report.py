"""D122 bevel oracle helper (authored; reads only existing local JSON).

manifest: snapshot -> BevelOracle sample manifest: every unique payload (SHA-1) whose
          inspection confirmed type `bevel`, with its archive location and the MAP GUID
          when a MAP row names it.
Output must be a new file under local-analysis/ (refuses to overwrite).
Usage: python -I scripts/bevel_report.py manifest SNAPSHOT NEW-JSON
"""
import argparse
import json
import os

PIN = "d6a2ef80b5a09999f08f23a803b516096055e5c9"
SCOPE = "bevb_toolkit_rbevel_fields_v1"


def write_new(path, value):
    real = os.path.realpath(os.path.dirname(os.path.abspath(path)))
    root = os.path.realpath("local-analysis")
    if not path.endswith(".json") or os.path.exists(path) or os.path.commonpath([real, root]) != root:
        raise SystemExit("new ignored JSON under local-analysis/ required: " + path)
    with open(path, "x", encoding="utf-8", newline="\n") as f:
        json.dump(value, f, indent=1, sort_keys=True, ensure_ascii=False)
        f.write("\n")


def cmd_manifest(a):
    with open(a.snapshot, encoding="utf-8") as f:
        snap = json.load(f)
    guids = {}
    for r in snap["resources"]:
        row = r["map_row"]
        guids.setdefault(row["sha1"].lower(), set()).add(row["guid"])
    seen = {}
    locations = 0
    for r in snap["resource_inspections"]:
        if ((r.get("inspection") or {}).get("confirmed_type") or {}).get("name") != "bevel":
            continue
        locations += 1
        sha = (r.get("sha1") or "").lower()
        if sha and sha not in seen:
            seen[sha] = r
    samples = []
    for sha, r in sorted(seen.items()):
        g = sorted(x for x in guids.get(sha, ()) if x)
        samples.append({"sha1": sha, "source_path": r["source_path"], "offset": r["offset"],
                        "size": r["size"], "guid": g[0] if len(g) == 1 else None,
                        "guid_count": len(g)})
    write_new(a.output, {"schema_version": 1, "toolkit_commit": PIN, "scope": SCOPE,
                         "snapshot_source_scope": snap.get("source_scope"),
                         "bevel_locations": locations, "samples": samples})
    print("manifest: %d unique bevel payloads from %d locations" % (len(samples), locations))


def main():
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest="cmd", required=True)
    m = sub.add_parser("manifest")
    m.add_argument("snapshot")
    m.add_argument("output")
    m.set_defaults(fn=cmd_manifest)
    a = p.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
