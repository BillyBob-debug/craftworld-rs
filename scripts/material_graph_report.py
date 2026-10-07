"""Metadata-only graph oracle census and consistency with the verified prefix oracle."""
import argparse
from collections import Counter
from material_prefix_report import PIN
from texture_report import load, write


def summarize(graphs, prefixes):
    if graphs.get("toolkit_commit") != PIN or prefixes.get("toolkit_commit") != PIN:
        raise ValueError("source pin differs")
    if graphs.get("scope") != "PS3_GMT_graph_metadata_no_shader_bytes" or graphs.get("schema_version") != 1:
        raise ValueError("graph scope differs")
    if prefixes.get("scope") != "PS3_GMT_prefix_only_no_shader_bytes" or prefixes.get("schema_version") != 1:
        raise ValueError("prefix scope differs")
    expected = {r["sha1"]: r for r in prefixes["samples"]}
    actual = {r["sha1"]: r for r in graphs["samples"]}
    if actual.keys() != expected.keys() or len(actual) != len(graphs["samples"]) or len(expected) != len(prefixes["samples"]):
        raise ValueError("sample identities differ or duplicate")
    kinds, slots, ports, masks, tails = Counter(), Counter(), Counter(), Counter(), Counter()
    boxes = wires = bad_slots = bad_endpoints = 0
    for sha in sorted(actual):
        r, p = actual[sha], expected[sha]
        if r["status"] != "parsed_graph" or p["status"] != "parsed_prefix":
            raise ValueError("unparsed sample")
        if any(r[k] != p[k] for k in ("prefix", "expanded_bytes", "compression_flags")):
            raise ValueError("previous prefix evidence differs")
        g = r["graph"]
        if g["graph_start"] != p["prefix"]["parsed_prefix_bytes"] or g["unparsed_bytes"] < 0 or g["parsed_graph_bytes"] < 0:
            raise ValueError("graph boundary differs")
        if g["graph_start"] + g["parsed_graph_bytes"] + g["unparsed_bytes"] != r["expanded_bytes"]:
            raise ValueError("graph lengths differ")
        if g["box_count"] != len(g["boxes"]) or g["wire_count"] != len(g["wires"]):
            raise ValueError("graph counts differ")
        for b in g["boxes"]:
            if b["declared_parameter_count"] != 8 or len(b["parameters_raw"]) != 8 or len(b["position_bits"]) != 4 or len(b["animations"]) != 2:
                raise ValueError("box shape differs")
            slot = b["parameters_raw"][5] if b["type_raw"] == 1 else None
            if b["texture_slot_raw"] != slot or b["texture_slot_in_range"] != (slot < 8 if slot is not None else None):
                raise ValueError("texture slot evidence differs")
            kinds[str(b["type_raw"])] += 1
            if slot is not None:
                slots[str(slot)] += 1; bad_slots += int(slot >= 8)
            for a in b["animations"]:
                count, mask = a["component_value_count"], a["components_raw"]
                components = (mask & 15).bit_count()
                if count and (not components or count % components):
                    raise ValueError("animation count/mask differs")
                if a["derived_key_count"] != (count // components if count else 0):
                    raise ValueError("animation key count differs")
                masks[str(mask)] += 1
        for w in g["wires"]:
            valid = [0 <= w[k] < g["box_count"] for k in ("box_from", "box_to")]
            if valid != [w["from_in_range"], w["to_in_range"]] or len(w["swizzle_raw"]) != 5:
                raise ValueError("wire range evidence differs")
            bad_endpoints += int(not all(valid)); ports[f'{w["port_from_raw"]}:{w["port_to_raw"]}'] += 1
        boxes += g["box_count"]; wires += g["wire_count"]; tails[str(g["unparsed_bytes"])] += 1
    return {"schema_version": 1, "scope": "external_graph_metadata_consistency_not_independent_parser_parity",
            "toolkit_commit": PIN, "matched_prefix_samples": len(actual), "box_count": boxes, "wire_count": wires,
            "box_type_counts": dict(sorted(kinds.items())), "texture_slot_counts": dict(sorted(slots.items())),
            "wire_port_pair_counts": dict(sorted(ports.items())), "animation_mask_counts": dict(sorted(masks.items())),
            "unparsed_tail_counts": dict(sorted(tails.items())), "out_of_range_slots": bad_slots,
            "out_of_range_wire_records": bad_endpoints}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("graphs"); p.add_argument("prefixes"); p.add_argument("output")
    a = p.parse_args(); result = summarize(load(a.graphs), load(a.prefixes)); write(a.output, result); print(result)


if __name__ == "__main__": main()
