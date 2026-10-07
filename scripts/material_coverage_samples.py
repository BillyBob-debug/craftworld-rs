"""Select a bounded oracle follow-up from census metadata; never read asset bytes."""
import argparse
from material_prefix_report import PIN
from texture_report import load, write


def select(census, baseline):
    if census.get("schema_version") != 1 or census.get("scope") != "selected_snapshot_material_metadata_coverage_not_full_game":
        raise ValueError("census scope/schema differs")
    if census["counts"]["cancelled"] or baseline.get("toolkit_commit") != PIN or baseline.get("scope") != "PS3_GMT_tail_metadata_no_shader_bytes":
        raise ValueError("incomplete census or baseline differs")
    covered = set()
    for r in baseline["samples"]:
        if r.get("status") != "parsed_graph" or r.get("tail_status") != "parsed_tail":
            raise ValueError("unparsed baseline")
        covered.update((("revision", r["prefix"]["revision"]), ("sound", r["tail"]["sound_enum_raw"]),
                        ("animations", r["tail"]["parameter_animation_count"])))
    candidates = []
    excluded = 0
    for r in census["locations"]:
        s = r.get("stages", {})
        if r["status"] != "stage_inspection_completed" or any(s.get(k, {}).get("status") != "parsed" for k in ("prefix", "graph", "tail")) or len(r["sha1_candidates"]) != 1:
            excluded += 1; continue
        f = {("revision", s["prefix"]["revision"]), ("sound", s["tail"]["sound_enum_raw"]),
             ("animations", s["tail"]["parameter_animation_count"])}
        shape = (s["graph"]["box_count"], s["graph"]["wire_count"])
        candidates.append((r, f, shape))
    candidates.sort(key=lambda c: (c[0]["size"], c[0]["sha1_candidates"][0], c[0]["source_path"], c[0]["offset"]))
    wanted = set().union(*(f for _, f, _ in candidates)) - covered
    chosen, shapes = [], set()
    while candidates and len(chosen) < 40:
        scores = [len(f & wanted) for _, f, _ in candidates]
        best = max(scores)
        if not best:
            unseen = next((i for i, (_, _, shape) in enumerate(candidates) if shape not in shapes), None)
            if unseen is None: break
            index = unseen
        else: index = scores.index(best)
        r, f, shape = candidates.pop(index); chosen.append(r); shapes.add(shape); wanted -= f
    samples = sorted(({"sha1": r["sha1_candidates"][0], **{k: r[k] for k in ("source_path", "offset", "size")}} for r in chosen), key=lambda r: (r["sha1"], r["source_path"], r["offset"]))
    if sum(r["size"] for r in samples) > 16 * 1024**2:
        raise ValueError("follow-up byte budget")
    return {"schema_version": 1, "scope": "material_census_selected_tail_oracle_followup_not_validation",
            "selection": "greedy_new_revision_sound_animation_counts_then_graph_shapes_max40",
            "excluded_locations": excluded, "selected_graph_shapes": len(shapes),
            "uncovered_primary_features": [list(f) for f in sorted(wanted)], "samples": samples}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ("census", "baseline", "output"): p.add_argument(name)
    a = p.parse_args(); result = select(load(a.census), load(a.baseline)); write(a.output, result)
    print({k: v for k, v in result.items() if k != "samples"}); print("Selected", len(result["samples"]), "samples")


if __name__ == "__main__": main()
