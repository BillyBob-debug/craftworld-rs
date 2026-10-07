"""Material header/dependency census from existing validated scanner JSON; no asset bytes."""
from collections import Counter
import argparse
from texture_report import load, write


def select(snapshot):
    materials=[r for r in snapshot["resource_inspections"]
               if (r["inspection"].get("confirmed_type") or {}).get("id")==7]
    groups={}
    for r in materials:
        i=r["inspection"]
        if i["status"]!="metadata_parsed":
            continue
        h=i["header"]
        groups.setdefault((h["revision"],h["compression_flags"],h["branch_id"]),[]).append(r)
    chosen=set(sorted(groups,key=lambda k:(-len(groups[k]),k))[:16])
    for flags in {k[1] for k in groups}:
        cohort=[k for k in groups if k[1]==flags]
        chosen.update([min(cohort),max(cohort)])
    samples={}
    for key in sorted(chosen):
        rows=sorted((r for r in groups[key] if 12<=r["size"]<=1048576),key=lambda r:(r["size"],r["sha1"]))
        if rows:
            for r in (rows[0],rows[len(rows)//2]):
                samples[r["sha1"]]={k:r[k] for k in ("sha1","source_path","offset","size")}
    if len(samples)>48:
        raise ValueError("sample budget exceeded")
    dependencies=Counter(str(d["type_id"]) for r in materials for d in r["inspection"]["dependencies"])
    return {"schema_version":1,"scope":"graphics_material_headers_dependencies_only",
            "known_material_locations":len(materials),"revision_cohorts":len(groups),
            "chosen_cohorts":len(chosen),"dependency_type_counts":dict(sorted(dependencies.items())),
            "status_counts":dict(Counter(r["inspection"]["status"] for r in materials)),
            "samples":[samples[k] for k in sorted(samples)]}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("snapshot");p.add_argument("output");a=p.parse_args()
    result=select(load(a.snapshot));write(a.output,result)
    print({k:v for k,v in result.items() if k!="samples"})
    print("Selected",len(result["samples"]),"material header samples")


if __name__=="__main__":
    main()
