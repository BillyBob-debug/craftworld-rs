"""Cross-check external material prefixes with prior header/dependency evidence.

This is consistency checking, not independent material-parser parity. No game reads.
"""
import argparse
from collections import Counter
from texture_report import load, write

PIN = "d6a2ef80b5a09999f08f23a803b516096055e5c9"


def summarize(prefixes, headers):
    if prefixes.get("schema_version") != 1 or prefixes.get("scope") != "PS3_GMT_prefix_only_no_shader_bytes":
        raise ValueError("unsupported prefix scope")
    if prefixes.get("toolkit_commit") != PIN or headers.get("toolkit_commit") != PIN:
        raise ValueError("source pin differs")
    expected = {s["sha1"]: s for s in headers["samples"]}
    actual = {s["sha1"]: s for s in prefixes["samples"]}
    if len(actual) != len(prefixes["samples"]) or len(expected) != len(headers["samples"]) or actual.keys() != expected.keys():
        raise ValueError("sample identities differ or duplicate")
    kinds, wraps, revisions = Counter(), Counter(), set()
    references = 0
    for sha in sorted(actual):
        row, header = actual[sha], expected[sha]
        if row["status"] != "parsed_prefix":
            raise ValueError(f"rejected prefix: {sha}")
        p = row["prefix"]
        if header["type_id"] != 7 or header["magic"] != "GMTb" or header["branch_id"] != 0:
            raise ValueError("header outside current scope")
        if p["revision"] != header["revision"] or row["compression_flags"] != header["compression_flags"]:
            raise ValueError("prefix/header fields differ")
        if p["parsed_prefix_bytes"] + p["unparsed_bytes"] != row["expanded_bytes"] or p["unparsed_bytes"] < 0:
            raise ValueError("prefix lengths disagree")
        offsets = p["shader_end_offsets"]
        if len(offsets) != p["shader_count"] or len(offsets) != p["shader_offset_count"] or not offsets:
            raise ValueError("shader table counts disagree")
        if sorted(offsets) != offsets or offsets[0] < 0 or offsets[-1] > p["blob_bytes"]:
            raise ValueError("shader intervals outside blob")
        if p["uninterpreted_after_shader_bytes"] != p["blob_bytes"] - offsets[-1]:
            raise ValueError("uninterpreted blob length disagrees")
        if p["blob_start"] + p["blob_bytes"] > p["parsed_prefix_bytes"]:
            raise ValueError("blob exceeds parsed prefix")
        if len(p["textures"]) != 8 or any(len(p[k]) > 8 for k in ("wrap_s_raw", "wrap_t_raw")):
            raise ValueError("slot budget")
        deps = set()
        for t in p["textures"]:
            if t["kind"] not in (0, 1, 2, 3) or bool(t["kind"] & 2) != (t["guid"] is not None) or bool(t["kind"] & 1) != (t["sha1"] is not None):
                raise ValueError("descriptor presence differs from raw kind")
            guid = f'{t["guid"]:08x}' if t["guid"] else None
            digest = None if t["sha1"] == "0" * 40 else t["sha1"]
            if guid or digest:
                deps.add((guid, digest, 1))
            kinds[str(t["kind"])] += 1
        if deps != {(d["guid"], d["sha1"], d["type_id"]) for d in header["dependencies"]}:
            raise ValueError("slot references and header dependencies differ")
        references += len(deps)
        wraps.update(str(v) for k in ("wrap_s_raw", "wrap_t_raw") for v in p[k])
        revisions.add(p["revision"])
    return {"schema_version": 1, "toolkit_commit": PIN,
            "scope": "external_prefix_header_consistency_not_independent_parser_parity",
            "matched_samples": len(actual), "matched_normalized_texture_references": references,
            "revisions": sorted(revisions), "raw_descriptor_kind_counts": dict(sorted(kinds.items())),
            "raw_wrap_value_counts": dict(sorted(wraps.items()))}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("prefixes"); p.add_argument("headers"); p.add_argument("output")
    a = p.parse_args()
    result = summarize(load(a.prefixes), load(a.headers))
    write(a.output, result)
    print(result)


if __name__ == "__main__":
    main()
