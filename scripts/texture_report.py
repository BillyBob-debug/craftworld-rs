"""C1 metadata-only sample selection and field parity report; no asset bytes read.

Results require a validated census and pinned external oracle. Pixel digests are
future regression expectations, not evidence that a Rust pixel decoder exists.
"""
import argparse
from collections import Counter
import json
from pathlib import Path


def load(path):
    with open(path, encoding="utf-8") as file:
        return json.load(file)


def write(path, value):
    path = Path(path)
    if not path.parent.resolve().is_relative_to(Path("local-analysis").resolve()):
        raise ValueError("output must be under ignored local-analysis")
    with path.open("x", encoding="utf-8", newline="\n") as file:
        json.dump(value, file, sort_keys=True, indent=2)
        file.write("\n")


def select(census):
    families = {}
    for row in census["textures"]:
        m = row.get("metadata", {})
        h = m.get("gtf") or m.get("dds")
        if not h or m["chunks"]["expanded_bytes"] > 32 * 1024**2:
            continue
        if not (0 < h["width"] <= 4096 and 0 < h["height"] <= 4096
                and h["width"] * h["height"] <= 4 * 1024**2):
            continue
        if m["gtf"] and (h["dimension"] != 2 or h["cubemap"] or h["depth"] != 1):
            continue
        if m["dds"] and h["caps2"] & 0x20FE00:
            continue
        family = (m["wrapper"], str(h.get("format_raw", h.get("fourcc"))), h.get("rgb_bits", 0))
        families.setdefault(family, []).append(row)
    selected = {}
    for family in sorted(families):
        rows = sorted(families[family], key=lambda r: (r["size"], r["sha1"]))
        # Smallest, largest mip count, rectangular, bump, and linear flag variants.
        h = lambda r: r["metadata"]["gtf"] or r["metadata"]["dds"]
        choices = [rows[0], min(rows, key=lambda r: (-h(r)["mips"], r["size"], r["sha1"]))]
        for predicate in [lambda v: v["width"] != v["height"],
                          lambda v: v.get("flags", 0) & 1]:
            match = next((r for r in rows if predicate(h(r))), None)
            if match:
                choices.append(match)
        for row in choices:
            selected[row["sha1"]] = {k: row[k] for k in ("sha1", "source_path", "offset", "size")}
    samples = sorted(selected.values(), key=lambda r: r["sha1"])
    if len(samples) > 48:
        raise ValueError("oracle sample budget exceeded")
    return {"schema_version": 1, "selection": "smallest/max-mips/rectangular/bump per present 2D format family",
            "samples": samples}


def verify(census, manifest, oracle):
    expected = {r["sha1"]: r for r in census["textures"]}
    samples = {r["sha1"]: r for r in oracle["samples"]}
    mismatches, failures, fields = [], [], 0
    if oracle["toolkit_commit"] != census["toolkit_commit"]:
        raise ValueError("oracle source pin differs")
    if set(samples) != {r["sha1"] for r in manifest["samples"]}:
        raise ValueError("sample coverage differs")
    for sha, result in sorted(samples.items()):
        if result["status"] != "decoded_top_mip":
            failures.append({"sha1": sha, "reason": result.get("reason")})
            continue
        m = expected[sha]["metadata"]
        h = m["gtf"] or m["dds"]
        pairs = [("width", h["width"], result["image_width"]),
                 ("height", h["height"], result["image_height"])]
        if m["gtf"]:
            for key in ("width", "height", "depth", "mips", "format_base"):
                pairs.append(("gtf." + key, h[key], result["gtf"][key]))
            pairs.extend([("gtf.cubemap", bool(h["cubemap"]), result["gtf"]["cubemap"]),
                          ("gtf.volume", h["dimension"] > 2, result["gtf"]["volume"])])
        else:
            for key in ("width", "height", "mips", "fourcc", "rgb_bits", "caps2"):
                pairs.append(("dds." + key, h[key], result["dds"][key]))
        for key, actual, reference in pairs:
            fields += 1
            if actual != reference:
                mismatches.append({"sha1": sha, "field": key, "census": actual, "oracle": reference})
    return {"schema_version": 1, "samples": len(samples), "fields_compared": fields,
            "mismatches": mismatches, "oracle_failures": failures,
            "census_summary": summarize(census),
            "pixel_comparison": "Toolkit top-mip RGBA digests recorded; Rust pixels not implemented"}


def select_bc1(census):
    groups = {}
    for row in census["textures"]:
        h = row.get("metadata", {}).get("gtf")
        if (not h or h["format_raw"] != 0x86 or h["dimension"] != 2 or h["cubemap"]
                or h["depth"] != 1 or h["width"] * h["height"] > 4 * 1024**2):
            continue
        groups.setdefault((h["mips"], h["flags"], h["width"] == h["height"]), []).append(row)
    samples = []
    for group in sorted(groups):
        rows = sorted(groups[group], key=lambda r: (r["size"], r["sha1"]))
        row = rows[len(rows) // 2]
        samples.append({k: row[k] for k in ("sha1", "source_path", "offset", "size")})
    if len(samples) > 48:
        raise ValueError("sample budget exceeded")
    return {"schema_version": 1, "selection": "C2 median archived size per GTF BC1 mip/flags/square group",
            "samples": sorted(samples, key=lambda r: r["sha1"])}


def summarize(census):
    gtf, dds, deltas, errors = Counter(), Counter(), Counter(), Counter()
    for row in census["textures"]:
        m = row.get("metadata", {})
        h = m.get("gtf")
        if h:
            gtf[str(h["format_base"])] += 1
            if h["format_base"] in (0x86, 0x88) and h["dimension"] == 2 and not h["cubemap"]:
                block = 8 if h["format_base"] == 0x86 else 16
                size = sum(((max(1, h["width"] >> i) + 3) // 4)
                           * ((max(1, h["height"] >> i) + 3) // 4) * block for i in range(h["mips"]))
                deltas[str(m["chunks"]["expanded_bytes"] - size)] += 1
        h = m.get("dds")
        if h:
            dds[f"{h['fourcc']!r}/{h['rgb_bits']}bit"] += 1
        if m.get("dds_error"):
            errors[m["dds_error"]] += 1
    return {"gtf_base_formats": dict(gtf), "tex_dds_formats": dict(dds), "dds_errors": dict(errors),
            "simple_gtf_dxt_expanded_surplus_over_standard_mip_chain": dict(deltas)}


def select_tex_bc1(census):
    groups = {}
    for row in census["textures"]:
        h = row.get("metadata", {}).get("dds")
        if not h or h["fourcc"] != "DXT1" or h["caps2"] or h["width"] * h["height"] > 4 * 1024**2:
            continue
        groups.setdefault((h["mips"], h["width"] == h["height"], h["width"] % 4, h["height"] % 4), []).append(row)
    samples = []
    for group in sorted(groups):
        rows = sorted(groups[group], key=lambda r: (r["size"], r["sha1"]))
        for row in (rows[0], rows[len(rows) // 2]):
            samples.append({k: row[k] for k in ("sha1", "source_path", "offset", "size")})
    samples = {r["sha1"]: r for r in samples}
    if len(samples) > 48:
        raise ValueError("sample budget exceeded")
    return {"schema_version": 1, "selection": "C3 smallest/median per TEX BC1 mip/square/partial-block group",
            "samples": [samples[k] for k in sorted(samples)]}


def select_bc3(census):
    groups = {}
    for row in census["textures"]:
        m = row.get("metadata", {})
        h = m.get("gtf") or m.get("dds")
        if not h or row["size"] > 16 * 1024**2 or m["chunks"]["expanded_bytes"] > 32 * 1024**2:
            continue
        if not (0 < h["width"] <= 4096 and 0 < h["height"] <= 4096
                and h["width"] * h["height"] <= 4 * 1024**2):
            continue
        if m.get("gtf"):
            if h["format_raw"] != 0x88 or h["dimension"] != 2 or h["cubemap"] or h["depth"] != 1:
                continue
        elif h["fourcc"] != "DXT5" or h["caps2"]:
            continue
        group = (m["wrapper"], h["mips"], h.get("flags", 0), h["width"] == h["height"],
                 h["width"] % 4, h["height"] % 4)
        groups.setdefault(group, []).append(row)
    samples = {}
    for group in sorted(groups):
        rows = sorted(groups[group], key=lambda r: (r["size"], r["sha1"]))
        row = rows[len(rows) // 2]
        samples[row["sha1"]] = {k: row[k] for k in ("sha1", "source_path", "offset", "size")}
    if len(samples) > 48:
        raise ValueError("BC3 sample budget exceeded")
    return {"schema_version": 1, "selection": "C4 median per BC3 wrapper/mip/flags/square/partial-block group",
            "samples": [samples[k] for k in sorted(samples)]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("select", "select-bc1", "select-tex-bc1", "select-bc3", "verify"))
    parser.add_argument("census")
    parser.add_argument("output")
    parser.add_argument("--manifest")
    parser.add_argument("--oracle")
    args = parser.parse_args()
    census = load(args.census)
    selectors = {"select": select, "select-bc1": select_bc1, "select-tex-bc1": select_tex_bc1, "select-bc3": select_bc3}
    value = selectors[args.mode](census) if args.mode != "verify" else verify(census, load(args.manifest), load(args.oracle))
    write(args.output, value)
    print(json.dumps({k: v for k, v in value.items() if k != "samples"}, sort_keys=True))
    if args.mode != "verify":
        print(f"Selected {len(value['samples'])} texture samples")
    elif value["mismatches"] or value["oracle_failures"]:
        raise SystemExit(2)


if __name__ == "__main__":
    main()
