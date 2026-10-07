"""Authored snapshot-only animation census/sample selector. No game reads."""
import argparse
import json
from pathlib import Path

PIN = "d6a2ef80b5a09999f08f23a803b516096055e5c9"


def census(snapshot):
    aliases = [r["map_row"] for r in snapshot["resources"]
               if r["map_row"]["path"].lower().endswith(".anim")]
    groups = {}
    for row in snapshot["resource_inspections"]:
        inspection = row["inspection"]
        if (inspection.get("confirmed_type") or {}).get("name") != "animation":
            continue
        header = inspection.get("header") or {}
        key = (inspection.get("magic_hex"), header.get("revision"),
               header.get("branch_id"), header.get("branch_revision"),
               header.get("compression_flags"), header.get("compressed"))
        groups.setdefault(key, []).append(row)
    cohorts, selected = [], {}
    for key, rows in sorted(groups.items(), key=lambda item: json.dumps(item[0])):
        ordered = sorted(rows, key=lambda r: (r["size"], r.get("sha1") or "",
                                             r["source_path"], r["offset"]))
        cohorts.append(dict(zip(("magic_hex", "revision", "branch_id", "branch_revision",
                                 "compression_flags", "compressed"), key),
                            locations=len(rows), stored_bytes=sum(r["size"] for r in rows),
                            min_stored_bytes=ordered[0]["size"], max_stored_bytes=ordered[-1]["size"]))
        # Preserve unsupported old cohorts as explicit oracle outcomes.
        candidates = [r for r in ordered if r.get("sha1") and r["source_path"] == "USRDIR/DATA.FARC"
                      and 18 <= r["size"] <= 2 * 1024**2 and key[0] == "414e4d62"]
        if candidates:
            for row in (candidates[0], candidates[-1]):
                sample = {k: row[k] for k in ("sha1", "source_path", "offset", "size")}
                selected[(row["sha1"], row["source_path"], row["offset"], row["size"])] = sample
    samples = sorted(selected.values(), key=lambda r: (r["sha1"], r["source_path"], r["offset"]))
    if len(samples) > 48:
        raise ValueError("cohort sample count exceeds oracle budget; refine selection")
    return ({"schema_version": 1, "scope": "snapshot_animation_metadata_only",
             "extension_alias_rows": len(aliases), "extension_unique_hashes": len({r["sha1"] for r in aliases}),
             "confirmed_locations": sum(c["locations"] for c in cohorts),
             "stored_bytes": sum(c["stored_bytes"] for c in cohorts), "cohorts": cohorts},
            {"schema_version": 1, "toolkit_commit": PIN,
             "selection": "minimum_and_maximum_stored_size_per_snapshot_header_cohort",
             "samples": samples})


def write(path, value):
    path = Path(path)
    allowed = Path("local-analysis").resolve(strict=True)
    if not path.parent.resolve(strict=True).is_relative_to(allowed):
        raise ValueError("new ignored output required")
    with path.open("x", encoding="utf-8", newline="\n") as file:
        json.dump(value, file, sort_keys=True, indent=2)
        file.write("\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("snapshot")
    parser.add_argument("new_census")
    parser.add_argument("new_manifest")
    args = parser.parse_args()
    with open(args.snapshot, encoding="utf-8") as file:
        report, manifest = census(json.load(file))
    write(args.new_census, report)
    write(args.new_manifest, manifest)
    print(f'{report["confirmed_locations"]} confirmed locations, {len(report["cohorts"])} cohorts, '
          f'{len(manifest["samples"])} bounded samples')


if __name__ == "__main__":
    main()
