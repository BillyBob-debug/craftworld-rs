"""D15 metadata-only graph-count/boundary selection and oracle coverage audit.

Uses the D13 census and pinned external tail reports, never reads game payloads.
Count-shape coverage does not establish topology, shader or animation semantics.
"""
import argparse
import json
from material_prefix_report import PIN
from texture_report import load, write


def accepted(row):
    return row["status"] == "stage_inspection_completed" and all(
        row["stages"].get(k, {}).get("status") == "parsed" for k in ("prefix", "graph", "tail"))


def observed(census):
    if (census.get("schema_version") != 1 or
            census.get("scope") != "selected_snapshot_material_metadata_coverage_not_full_game" or
            census["counts"]["cancelled"]):
        raise ValueError("incomplete or unexpected census")
    return [r for r in census["locations"] if accepted(r)]


def shape(row):
    return row["stages"]["graph"]["box_count"], row["stages"]["graph"]["wire_count"]


def maximum(row):
    return max(row["stages"]["tail"]["component_value_counts"], default=0)


def validated(reports):
    samples = {}
    for report in reports:
        if (report.get("schema_version") != 1 or report.get("toolkit_commit") != PIN or
                report.get("scope") != "PS3_GMT_tail_metadata_no_shader_bytes"):
            raise ValueError("oracle pin/scope/schema differs")
        for row in report["samples"]:
            if row.get("status") != "parsed_graph" or row.get("tail_status") != "parsed_tail":
                raise ValueError("unparsed oracle sample")
            key = row["sha1"]
            if key in samples and samples[key] != row:
                raise ValueError("conflicting oracle identifier")
            samples[key] = row
    shapes = {(r["graph"]["box_count"], r["graph"]["wire_count"]) for r in samples.values()}
    largest = max((a["component_value_count"] for r in samples.values()
                   for a in r["tail"]["parameter_animations"]), default=0)
    return samples, shapes, largest


def select(census, reports):
    rows = observed(census)
    prior, covered, prior_max = validated(reports)
    wanted = {shape(r) for r in rows} - covered
    target_max = max(map(maximum, rows), default=0)
    # Any ambiguous/failed location contaminates its identifiers. Agreement here
    # means identical stored size and emitted census stage metadata, not full bytes.
    by_hash, blocked = {}, set()
    for row in census["locations"]:
        hashes = row["sha1_candidates"]
        if len(hashes) != 1 or not accepted(row):
            blocked.update(hashes)
        else:
            by_hash.setdefault(hashes[0], []).append(row)
    candidates = []
    for key, aliases in by_hash.items():
        signatures = {(r["size"], json.dumps(r["stages"], sort_keys=True)) for r in aliases}
        if key in blocked or len(signatures) != 1:
            blocked.add(key)
            continue
        candidates.append(min(aliases, key=lambda r: (r["source_path"], r["offset"])))
    candidates.sort(key=lambda r: (r["size"], r["sha1_candidates"][0], r["source_path"], r["offset"]))
    chosen = {}
    for count_shape in sorted(wanted):
        row = next((r for r in candidates if shape(r) == count_shape), None)
        if row is None:
            raise ValueError("unrepresented shape has no unambiguous candidate")
        chosen[row["sha1_candidates"][0]] = row
    if target_max > max([prior_max] + [maximum(r) for r in chosen.values()]):
        row = next((r for r in candidates if maximum(r) == target_max), None)
        if row is None:
            raise ValueError("largest array has no unambiguous candidate")
        chosen[row["sha1_candidates"][0]] = row
    samples = [{"sha1": key, **{k: r[k] for k in ("source_path", "offset", "size")}}
               for key, r in sorted(chosen.items())]
    if (len(samples) > 48 or any(not 0 < r["size"] <= 1024**2 for r in samples) or
            sum(r["size"] for r in samples) > 16 * 1024**2):
        raise ValueError("oracle sample/resource/aggregate budget exceeded")
    return {"schema_version": 1, "scope": "material_graph_shape_boundary_selection_not_validation",
            "selection": "smallest_unambiguous_per_missing_count_shape_then_largest_array_max48",
            "prior_unique_samples": len(prior), "missing_graph_shapes": [list(s) for s in sorted(wanted)],
            "excluded_identifiers": sorted(blocked), "target_component_value_maximum": target_max,
            "sample_bytes": sum(r["size"] for r in samples), "samples": samples}


def coverage(census, reports):
    rows = observed(census)
    samples, shapes, largest = validated(reports)
    observed_shapes = {shape(r) for r in rows}
    target_max = max(map(maximum, rows), default=0)
    if observed_shapes - shapes or largest != target_max:
        raise ValueError("oracle union does not cover census shapes/array maximum")
    return {"schema_version": 1, "scope": "material_sample_oracle_count_shape_coverage_not_full_corpus",
            "toolkit_commit": PIN, "unique_samples": len(samples),
            "observed_graph_shapes": len(observed_shapes), "covered_graph_shapes": len(shapes & observed_shapes),
            "component_value_maximum": largest, "missing_graph_shapes": []}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("census")
    parser.add_argument("output")
    parser.add_argument("oracles", nargs="+")
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    result = (coverage if args.verify else select)(load(args.census), [load(p) for p in args.oracles])
    write(args.output, result)
    print({k: v for k, v in result.items() if k != "samples"})
    if "samples" in result:
        print("Selected", len(result["samples"]), "samples")


if __name__ == "__main__":
    main()
