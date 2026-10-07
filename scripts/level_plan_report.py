"""Authored snapshot-only level/plan census and bounded plan sample selection.

No original game reads. Prefix evidence/selection limits: docs/level-plan-recon.md.
"""
import argparse
from collections import Counter
import json
from pathlib import Path

PIN = "d6a2ef80b5a09999f08f23a803b516096055e5c9"
MAX_SAMPLE = 2 * 1024**2
MAX_TOTAL = 64 * 1024**2


def census(snapshot):
    reports = {}
    plan_groups = []
    for kind, extension, magic in [('level', '.bin', '4c564c62'), ('plan', '.plan', '504c4e62')]:
        aliases = [r['map_row'] for r in snapshot['resources']
                   if r['map_row']['path'].lower().endswith(extension)]
        groups = {}
        rows = [r for r in snapshot['resource_inspections']
                if (r['inspection'].get('confirmed_type') or {}).get('name') == kind]
        for row in rows:
            i = row['inspection']; h = i.get('header') or {}
            key = (i.get('magic_hex'), h.get('revision'), h.get('branch_id'),
                   h.get('branch_revision'), h.get('compression_flags'), h.get('compressed'))
            groups.setdefault(key, []).append(row)
        cohorts = []
        for key, members in sorted(groups.items(), key=lambda item: json.dumps(item[0])):
            ordered = sorted(members, key=lambda r: (r['size'], r.get('sha1') or '', r['source_path'], r['offset']))
            c = dict(zip(('magic_hex', 'revision', 'branch_id', 'branch_revision',
                          'compression_flags', 'compressed'), key),
                     locations=len(members), stored_bytes=sum(r['size'] for r in members),
                     min_stored_bytes=ordered[0]['size'], max_stored_bytes=ordered[-1]['size'])
            cohorts.append(c)
            # Proposed first oracle scope is modern clear ordinary branch-zero plans.
            if kind == 'plan' and key[0] == magic and isinstance(key[1], int) and 893 <= key[1] <= 951 \
                    and key[2:4] == (0, 0) and key[4] in (0, 7):
                eligible = [r for r in ordered if r.get('sha1') and r['source_path'] == 'USRDIR/DATA.FARC'
                            and 18 <= r['size'] <= MAX_SAMPLE]
                if eligible:
                    plan_groups.append((key, c['locations'], eligible))
        parsed = [r for r in rows if r['inspection'].get('dependency_status') == 'parsed']
        reports[kind] = {
            'extension': extension, 'extension_alias_rows': len(aliases),
            'extension_unique_hashes': len({r['sha1'] for r in aliases}),
            'extension_warning': 'Naming evidence only; .bin is not uniquely a level extension.' if kind == 'level' else 'Naming evidence only.',
            'confirmed_locations': len(rows), 'stored_bytes': sum(r['size'] for r in rows),
            'inspection_statuses': dict(sorted(Counter(r['inspection'].get('status', 'missing') for r in rows).items())),
            'dependency_statuses': dict(sorted(Counter(r['inspection'].get('dependency_status', 'missing') for r in rows).items())),
            'parsed_dependency_entries': sum(len(r['inspection'].get('dependencies') or []) for r in parsed),
            'parsed_dependency_type_ids': dict(sorted(Counter(str(d['type_id']) for r in parsed
                                                   for d in r['inspection'].get('dependencies') or []).items())),
            'cohorts': cohorts,
        }
    ordered_groups = sorted(plan_groups, key=lambda g: (-g[1], json.dumps(g[0])))
    selected = {}; represented = set(); total = 0
    # One minimum per common cohort first; then maxima. Explicit omitted coverage.
    for side in (0, -1):
        for key, _, rows in ordered_groups:
            r = rows[side]; identity = (r['sha1'], r['source_path'], r['offset'], r['size'])
            if identity in selected or len(selected) >= 48 or total + r['size'] > MAX_TOTAL:
                continue
            selected[identity] = {k: r[k] for k in ('sha1', 'source_path', 'offset', 'size')}
            represented.add(key); total += r['size']
    manifest = {'schema_version': 1, 'toolkit_commit': PIN,
                'scope': 'clear_plan_prefix_revision_893_951_branch_zero',
                'selection': 'minimum_per_cohort_by_frequency_then_maximum_until_48_samples_64MiB',
                'eligible_cohorts': len(plan_groups), 'represented_cohorts': len(represented),
                'omitted_eligible_cohorts': [list(k) for k, _, _ in ordered_groups if k not in represented],
                'selected_stored_bytes': total,
                'samples': sorted(selected.values(), key=lambda r: (r['sha1'], r['source_path'], r['offset'], r['size']))}
    return {'schema_version': 1, 'scope': 'snapshot_level_plan_metadata_only', 'types': reports}, manifest


def write(path, value):
    path = Path(path); allowed = Path('local-analysis').resolve(strict=True)
    if path.suffix != '.json' or not path.parent.resolve(strict=True).is_relative_to(allowed):
        raise ValueError('new ignored JSON output required')
    with path.open('x', encoding='utf-8', newline='\n') as file:
        json.dump(value, file, sort_keys=True, indent=2); file.write('\n')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('snapshot'); p.add_argument('new_census'); p.add_argument('new_plan_manifest')
    a = p.parse_args()
    with open(a.snapshot, encoding='utf-8') as file:
        report, manifest = census(json.load(file))
    write(a.new_census, report); write(a.new_plan_manifest, manifest)
    for kind, r in report['types'].items():
        print(f'{kind}: {r["confirmed_locations"]} confirmed locations / {len(r["cohorts"])} cohorts')
    print(f'{len(manifest["samples"])} plan samples / {manifest["represented_cohorts"]} represented cohorts')


if __name__ == '__main__':
    main()
