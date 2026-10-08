"""D106 level oracle helper (authored; reads only existing local JSON).

manifest: snapshot -> LevelOracle sample manifest (every confirmed `level` resource).
summary:  LevelOracle JSON + snapshot + mesh corpus census + diffuse-route census
          -> deterministic L0 report (counts only, no game bytes).
Outputs must be new files under local-analysis/ (refuses to overwrite).
"""
import argparse
import collections
import json
import os
import sys

PIN = "d6a2ef80b5a09999f08f23a803b516096055e5c9"
SCOPE = "lvlb_toolkit_rlevel_summary_and_scene_v1"


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def write_new(path, value):
    real = os.path.realpath(os.path.dirname(os.path.abspath(path)))
    root = os.path.realpath("local-analysis")
    if not path.endswith(".json") or os.path.exists(path) or os.path.commonpath([real, root]) != root:
        raise SystemExit("new ignored JSON under local-analysis/ required: " + path)
    with open(path, "x", encoding="utf-8", newline="\n") as f:
        json.dump(value, f, indent=1, sort_keys=True, ensure_ascii=False)
        f.write("\n")


def prefix(path, depth=3):
    return "/".join(path.split("/")[:depth])


def level_rows(snapshot):
    inspections = snapshot["resource_inspections"]
    rows = []
    for r in snapshot["resources"]:
        if r.get("confirmed_type") != "level":
            continue
        cands = r.get("archive_candidates") or []
        row = {"map_path": r["map_row"]["path"], "guid": r["map_row"]["guid"], "sha1": r["map_row"]["sha1"],
               "candidate_count": len(cands)}
        if len(cands) == 1:
            c = cands[0]
            row.update(source_path=c["archive"], offset=c["offset"], size=c["size"])
            ins = inspections[c["inspection_index"]]["inspection"]
            row["snapshot_header"] = ins.get("header")
        rows.append(row)
    rows.sort(key=lambda x: (x["map_path"], x["sha1"]))
    return rows


def cmd_manifest(a):
    snap = load(a.snapshot)
    scene = set(a.scene or [])
    samples, skipped = [], []
    for r in level_rows(snap):
        if r["candidate_count"] != 1:
            skipped.append({"map_path": r["map_path"], "reason": "archive candidates %d" % r["candidate_count"]})
            continue
        samples.append({"sha1": r["sha1"], "map_path": r["map_path"], "guid": r["guid"],
                        "source_path": r["source_path"], "offset": r["offset"], "size": r["size"],
                        "scene": r["map_path"] in scene})
    missing = sorted(scene - {s["map_path"] for s in samples})
    if missing:
        raise SystemExit("scene paths not in manifest: %s" % missing)
    if a.only_scene:
        samples = [s for s in samples if s["scene"]]
        skipped = []
    write_new(a.output, {"schema_version": 1, "toolkit_commit": PIN, "scope": SCOPE,
                         "snapshot_source_scope": snap.get("source_scope"), "samples": samples,
                         "skipped": skipped})
    print("manifest: %d samples, %d scene, %d skipped" % (len(samples), sum(s["scene"] for s in samples), len(skipped)))


def guid_of(key):
    return key[1:] if key.startswith("g") else None


def cmd_summary(a):
    oracle, snap, corpus, routes = load(a.oracle), load(a.snapshot), load(a.corpus), load(a.routes)
    if oracle.get("toolkit_commit") != PIN or oracle.get("scope") != SCOPE:
        raise SystemExit("oracle pin/scope differs")
    viewable = set(corpus["viewable_guid_list"])
    diffuse = set(corpus["all_diffuse_guid_list"])
    guid_sha = {}
    type_by_guid = {}
    for r in snap["resources"]:
        g = r["map_row"].get("guid")
        if g:
            guid_sha[g] = r["map_row"]["sha1"]
            type_by_guid[g] = r.get("confirmed_type")
    route_class = {m["sha1"]: m["class"] for m in routes["materials_detail"]}
    bevel_rows = [r for r in snap["resources"] if r.get("confirmed_type") == "bevel"]
    bevel_headers = collections.Counter()
    for r in bevel_rows:
        c = (r.get("archive_candidates") or [None])[0]
        h = snap["resource_inspections"][c["inspection_index"]]["inspection"].get("header") if c else None
        bevel_headers["%s/%s" % ((h or {}).get("revision"), (h or {}).get("branch_id"))] += 1

    status = collections.Counter()
    by_prefix = collections.defaultdict(lambda: collections.Counter())
    head_hist = collections.Counter()
    part_things, part_levels = collections.Counter(), collections.Counter()
    mesh_all, gfx_all, bevel_all, shape_mat_all, plan_all = (collections.Counter() for _ in range(5))
    bone_len = collections.Counter()
    levels = []
    for lv in oracle["levels"]:
        st = lv["status"]
        status[st] += 1
        p = prefix(lv["map_path"])
        by_prefix[p][st] += 1
        ok = st.startswith("parsed")
        h = lv.get("header") or {}
        if h:
            head_hist["head=%d branch=0x%x flags=%d crossplay_array=%s" % (
                h["head"], h["branch_id"], h["compression_flags"], h["head"] > 0x3e6)] += 1
        row = {"map_path": lv["map_path"], "guid": lv["guid"], "sha1": lv["sha1"], "status": st,
               "stored_bytes": lv["stored_bytes"], "reason": lv.get("reason")}
        if ok:
            for k, v in lv["part_histogram"].items():
                part_things[k] += v
                part_levels[k] += 1
            meshes = {guid_of(k): v for k, v in lv["render_mesh_descriptors"].items()}
            mesh_all.update({k: v for k, v in lv["render_mesh_descriptors"].items()})
            gfx_all.update(lv["gfx_material_descriptors"])
            bevel_all.update(lv["bevel_descriptors"])
            shape_mat_all.update(lv["shape_material_descriptors"])
            plan_all.update(lv["ref_plan_descriptors"])
            bone_len.update(lv["bone_things_length_histogram"])
            mesh_guids = [g for g in meshes if g]
            inst = sum(meshes.values())
            inst_viewable = sum(v for g, v in meshes.items() if g in viewable)
            inst_diffuse = sum(v for g, v in meshes.items() if g in diffuse)
            gfx = [guid_of(k) for k in lv["gfx_material_descriptors"]]
            gfx_route = collections.Counter(route_class.get(guid_sha.get(g), "no_route_row") if g else "non_guid"
                                            for g in gfx)
            c = lv["counts"]
            row.update(
                head=h.get("head"), branch_id=h.get("branch_id"), compression_flags=h.get("compression_flags"),
                expanded_bytes=lv["expanded_bytes"], trailing_bytes=lv["trailing_bytes"],
                thing_count=lv["thing_count"], max_thing_depth=lv["max_thing_depth"],
                reference_tag_count=lv["reference_tag_count"], part_type_count=len(lv["part_histogram"]),
                render_mesh_things=c["render_mesh"], shape_things=c["shape"], generated_mesh_things=c["generated_mesh"],
                shape_and_generated_mesh=c["shape_and_generated_mesh"], shape_vertices_total=c["shape_vertices_total"],
                distinct_render_mesh_guids=len(mesh_guids),
                distinct_render_mesh_guids_viewable=sum(g in viewable for g in mesh_guids),
                distinct_render_mesh_guids_all_diffuse=sum(g in diffuse for g in mesh_guids),
                render_mesh_instances_viewable=inst_viewable, render_mesh_instances_all_diffuse=inst_diffuse,
                render_mesh_instances=inst,
                distinct_gfx_materials=len(gfx), gfx_material_route_classes=dict(sorted(gfx_route.items())),
                distinct_shape_materials=c["distinct_shape_materials"], distinct_bevels=c["distinct_bevels"],
                distinct_ref_plans=c["distinct_ref_plans"],
                streaming_parts=lv["part_histogram"].get("STREAMING_DATA", 0) + lv["part_histogram"].get("STREAMING_HINT", 0),
                cross_play_hash_count=lv["cross_play_hash_count"])
        levels.append(row)

    def resolve(counter, kind):
        out = collections.Counter()
        for k in counter:
            g = guid_of(k)
            if g is None:
                out["hash_or_null:" + k[:1]] += 1
            elif g not in guid_sha:
                out["guid_not_in_catalog"] += 1
            else:
                out["catalog_type:" + str(type_by_guid[g])] += 1
        return dict(sorted(out.items()))

    mesh_guids_all = [guid_of(k) for k in mesh_all if guid_of(k)]
    gfx_classes = collections.Counter(route_class.get(guid_sha.get(guid_of(k)), "no_route_row") if guid_of(k) else "non_guid"
                                      for k in gfx_all)
    report = {
        "schema_version": 1, "toolkit_commit": PIN, "tool": "level_report.summary",
        "unit": "distinct level resource (MAP row with confirmed_type level) on the 01.33 overlay snapshot",
        "status_counts": dict(sorted(status.items())),
        "status_by_prefix": {k: dict(sorted(v.items())) for k, v in sorted(by_prefix.items())},
        "header_histogram": dict(sorted(head_hist.items())),
        "part_histogram_things": dict(sorted(part_things.items())),
        "part_histogram_levels": dict(sorted(part_levels.items())),
        "bone_things_length_histogram": dict(sorted(bone_len.items(), key=lambda kv: int(kv[0]))),
        "render_mesh": {"distinct_guids": len(mesh_guids_all),
                        "viewable": sum(g in viewable for g in mesh_guids_all),
                        "all_diffuse": sum(g in diffuse for g in mesh_guids_all),
                        "instances": sum(mesh_all.values()),
                        "instances_viewable": sum(v for k, v in mesh_all.items() if guid_of(k) in viewable),
                        "descriptor_resolution": resolve(mesh_all, "mesh")},
        "gfx_materials": {"distinct": len(gfx_all), "route_classes": dict(sorted(gfx_classes.items())),
                          "descriptor_resolution": resolve(gfx_all, "gmat")},
        "bevels": {"distinct_used": len(bevel_all), "descriptor_resolution": resolve(bevel_all, "bevel"),
                   "catalog_bevel_resources": len(bevel_rows), "catalog_bevel_head_branch": dict(sorted(bevel_headers.items()))},
        "shape_materials": {"distinct": len(shape_mat_all), "descriptor_resolution": resolve(shape_mat_all, "mat")},
        "ref_plans": {"distinct": len(plan_all), "descriptor_resolution": resolve(plan_all, "plan")},
        "max_thing_depth": max((r.get("max_thing_depth", 0) for r in levels), default=0),
        "levels": levels,
    }
    write_new(a.output, report)
    print("summary: %s" % dict(sorted(status.items())))


def main(argv=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    m = sub.add_parser("manifest")
    m.add_argument("snapshot")
    m.add_argument("output")
    m.add_argument("--scene", action="append", help="MAP path to include a full scene dump for")
    m.add_argument("--only-scene", action="store_true", help="keep only the --scene levels (L1 dump manifest)")
    m.set_defaults(fn=cmd_manifest)
    s = sub.add_parser("summary")
    s.add_argument("oracle")
    s.add_argument("snapshot")
    s.add_argument("corpus")
    s.add_argument("routes")
    s.add_argument("output")
    s.set_defaults(fn=cmd_summary)
    a = ap.parse_args(argv)
    a.fn(a)


if __name__ == "__main__":
    sys.exit(main())
