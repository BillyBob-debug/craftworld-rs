"""Mesh metadata cohort selection; never opens asset files. Outputs ignored diagnostics."""
import argparse
from collections import Counter
from texture_report import load, write


def select(snapshot):
    groups = {}
    for r in snapshot["resource_inspections"]:
        i = r["inspection"]
        if (i.get("confirmed_type") or {}).get("id") != 2 or i["status"] != "metadata_parsed":
            continue
        h = i["header"]
        groups.setdefault((h["revision"], h["compression_flags"], h["branch_id"]), []).append(r)
    chosen = set(sorted(groups, key=lambda k: (-len(groups[k]), k))[:16])
    for flags in sorted({k[1] for k in groups}):
        cohort = [k for k in groups if k[1] == flags]
        chosen.update([min(cohort), max(cohort)])
    chosen.update(k for k in groups if k[2])
    samples = {}
    for k in sorted(chosen):
        rows = sorted((r for r in groups[k] if r["size"] <= 2 * 1024**2), key=lambda r: (r["size"], r["sha1"]))
        if not rows:
            continue
        for r in [rows[0], rows[len(rows) // 2]]:
            samples[r["sha1"]] = {v: r[v] for v in ("sha1", "source_path", "offset", "size")}
    if len(samples) > 48:
        raise ValueError("sample budget exceeded")
    return {"schema_version": 1, "selection": "smallest/median in 16 most frequent revision/flags/branch cohorts plus flag revision extrema and branch cohorts",
            "known_mesh_locations": sum(len(v) for v in groups.values()),
            "revision_cohorts": len(groups), "chosen_cohorts": len(chosen),
            "flags_counts": dict(Counter({str(f): sum(len(v) for k, v in groups.items() if k[1] == f) for f in {k[1] for k in groups}})),
            "samples": [samples[k] for k in sorted(samples)]}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("snapshot")
    p.add_argument("output")
    args = p.parse_args()
    result = select(load(args.snapshot))
    write(args.output, result)
    print({k: v for k, v in result.items() if k != "samples"})
    print("Selected", len(result["samples"]), "mesh prefixes")


if __name__ == "__main__":
    main()
