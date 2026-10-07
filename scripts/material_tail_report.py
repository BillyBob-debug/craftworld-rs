"""Bounded external tail oracle consistency; no asset bytes or animation keys."""
import argparse
from collections import Counter
import re
from material_graph_report import summarize as graph_summary
from material_prefix_report import PIN
from texture_report import load, write


def summarize(tails, graphs, prefixes):
    if tails.get("toolkit_commit") != PIN or tails.get("schema_version") != 1 or tails.get("scope") != "PS3_GMT_tail_metadata_no_shader_bytes":
        raise ValueError("tail pin/schema/scope differs")
    graph_summary(graphs, prefixes)
    previous = {r["sha1"]: r for r in graphs["samples"]}
    current = {r["sha1"]: r for r in tails["samples"]}
    if len(current) != len(tails["samples"]) or current.keys() != previous.keys() or len(current) > 48:
        raise ValueError("sample identities/count differ")
    statuses, sounds, counts, residuals, masks = (Counter() for _ in range(5))
    animations = values = keys = parsed = 0
    for sha in sorted(current):
        r, p = current[sha], previous[sha]
        if r.get("status") != "parsed_graph" or any(r.get(k) != p[k] for k in ("prefix", "graph", "compression_flags", "expanded_bytes")):
            raise ValueError("previous graph/prefix evidence differs")
        status = r.get("tail_status")
        if status not in ("parsed_tail", "rejected"):
            raise ValueError("tail status differs")
        statuses[status] += 1
        if status == "rejected":
            if not isinstance(r.get("tail_error"), str) or "tail" in r:
                raise ValueError("failed tail outcome differs")
            continue
        t, g = r["tail"], r["graph"]
        if t["tail_start"] != g["graph_start"] + g["parsed_graph_bytes"] or t["parsed_tail_bytes"] < 0 or t["unparsed_bytes"] < 0:
            raise ValueError("tail boundary differs")
        if t["parsed_tail_bytes"] + t["unparsed_bytes"] != g["unparsed_bytes"]:
            raise ValueError("tail lengths differ")
        if not 0 <= t["sound_enum_raw"] <= 0xffffffff or not 0 <= t["parameter_animation_count"] <= 128 or t["parameter_animation_count"] != len(t["parameter_animations"]):
            raise ValueError("tail raw fields/count differs")
        for a in t["parameter_animations"]:
            n, mask = a["component_value_count"], a["components_raw"]
            c = (mask & 15).bit_count()
            if not 0 <= n <= 4096 or not 0 <= mask <= 255 or (n and (not c or n % c)) or a["derived_key_count"] != (n // c if n else 0):
                raise ValueError("animation count/mask differs")
            if len(a["base_value_bits"]) != 4 or any(not 0 <= v <= 0xffffffff for v in a["base_value_bits"]):
                raise ValueError("animation base bits differ")
            if not re.fullmatch(r"[0-9a-f]{64}", a["component_bytes_sha256"]) or not re.fullmatch(r"(?:[0-9a-f]{2}){0,64}", a["name_hex"]):
                raise ValueError("animation digest/name differs")
            masks[str(mask)] += 1; values += n; keys += a["derived_key_count"]
        animations += t["parameter_animation_count"]; parsed += 1
        sounds[str(t["sound_enum_raw"])] += 1; counts[str(t["parameter_animation_count"])] += 1; residuals[str(t["unparsed_bytes"])] += 1
    return {"schema_version": 1, "scope": "external_tail_metadata_consistency_not_independent_parser_parity",
            "toolkit_commit": PIN, "matched_graph_prefix_samples": len(current), "parsed_tail_samples": parsed,
            "tail_status_counts": dict(sorted(statuses.items())), "sound_enum_counts": dict(sorted(sounds.items())),
            "parameter_animation_count_counts": dict(sorted(counts.items())), "parameter_animation_total": animations,
            "component_value_total": values, "derived_key_total": keys, "animation_mask_counts": dict(sorted(masks.items())),
            "unparsed_tail_counts": dict(sorted(residuals.items()))}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ("tails", "graphs", "prefixes", "output"): p.add_argument(name)
    a = p.parse_args()
    result = summarize(load(a.tails), load(a.graphs), load(a.prefixes))
    write(a.output, result); print(result)


if __name__ == "__main__": main()
