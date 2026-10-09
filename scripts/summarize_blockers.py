"""Summarize a corpus_mesh_census --blockers report as a Markdown table (stdout).

Usage: python scripts/summarize_blockers.py REPORT.json [TOP]
Ranking only; not an asset count.
"""
import collections
import json
import sys


def main() -> None:
    report = json.load(open(sys.argv[1], encoding="utf-8"))
    top = int(sys.argv[2]) if len(sys.argv) > 2 else 40
    rows = report["blocking_materials"]
    t = report["totals"]
    print(f"scope {report['scope']}: {t['catalog_guids']} GUIDs, {t['all_diffuse_guids']} all-diffuse, "
          f"{len(rows)} blocking material keys")
    by_reason = collections.Counter()
    for r in rows:
        for reason in r["reasons"]:
            by_reason[reason[:90]] += r["sole_blocked_mesh_guids"]
    print("\n| sole-blocked GUIDs | reason (first 90 chars) |\n|---|---|")
    for reason, n in by_reason.most_common(25):
        print(f"| {n} | {reason.replace('|', '/')} |")
    print("\n| # | material key | sole-blocked | blocked | reasons |\n|---|---|---|---|---|")
    for i, r in enumerate(rows[:top], 1):
        reasons = "; ".join(x[:70] for x in r["reasons"]).replace("|", "/")
        print(f"| {i} | {r['key']} | {r['sole_blocked_mesh_guids']} | {r['blocked_mesh_guids']} | {reasons} |")


if __name__ == "__main__":
    main()
