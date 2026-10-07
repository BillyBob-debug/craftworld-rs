"""Compare scanner tables and selected clear metadata with pinned unmodified cwlib.

No asset reads. Raw multiplicity is retained by Rust; comparison normalizes exactly
the Toolkit null/HashSet semantics. Full inventories/results stay ignored.
"""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('inventory', type=Path)
    ap.add_argument('oracle', type=Path)
    ap.add_argument('--repeat', type=Path)
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    root = Path(__file__).resolve().parents[1]
    output = args.output.resolve()
    if not output.is_relative_to((root / 'local-analysis').resolve()) or output.exists():
        ap.error('Use a new ignored local-analysis report')
    raw = args.inventory.read_bytes()
    if args.repeat and args.repeat.read_bytes() != raw:
        raise ValueError('Repeated inventory differs')
    actual = json.loads(raw)
    oracle = json.loads(args.oracle.read_text(encoding='utf-8-sig'))
    pin = 'd6a2ef80b5a09999f08f23a803b516096055e5c9'
    if oracle['toolkit_commit'] != pin:
        raise ValueError('Oracle source pin differs')
    if actual['schema_version'] != 2 or actual['milestone'] != 'A2_resource_metadata':
        raise ValueError('Unsupported scanner schema')
    rows = sorted(actual['resources'], key=lambda r: r['map_row']['row_index'])
    normalized = [dict(guid=r['map_row']['guid'], path=r['map_row']['path'],
                       sha1=r['map_row']['sha1'], size=r['map_row']['declared_size'],
                       timestamp=r['map_row']['timestamp_raw']) for r in rows]
    if normalized != oracle['map_rows']:
        raise ValueError('Toolkit MAP fields differ')
    if len(actual['archives']) != 1:
        raise ValueError('Local acceptance expects one archive')
    entries = [dict(offset=r['offset'], sha1=r['sha1'], size=r['size'])
               for r in actual['archives'][0]['entries']]
    expected = sorted(oracle['farc_rows'], key=lambda r: (r['sha1'], r['offset']))
    if entries != expected:
        raise ValueError('Toolkit FARC fields differ')
    inspected = {r['sha1']: r['inspection'] for r in actual['resource_inspections'] if r['sha1']}
    sample_dependencies = 0
    revisions = set()
    for sample in oracle['samples']:
        info = inspected[sample['sha1']]
        if info['confirmed_type']['id'] != sample['type_id'] or bytes.fromhex(info['magic_hex']).decode('ascii') != sample['magic']:
            raise ValueError('Resource magic/type differs')
        header = info['header']
        for field in ['revision', 'branch_id', 'branch_revision', 'compression_flags', 'compressed', 'dependency_offset']:
            # Static-mesh header-specific values are deliberately not parsed in A2.
            if header[field] is not None and header[field] != sample[field]:
                raise ValueError(f'Header {field} differs for {sample["sha1"]}')
        revisions.add(header['revision'])
        normalized_deps = {}
        for dep in info['dependencies']:
            guid = None if dep['guid'] == '00000000' else dep['guid']
            sha1 = None if dep['sha1'] == '0' * 40 else dep['sha1']
            if guid is None and sha1 is None:
                continue
            # ResourceDescriptor.equals/hashCode ignores type. Keep the first type,
            # matching HashSet insertion; unknown ResourceType becomes INVALID=0.
            normalized_deps.setdefault((guid, sha1), dep['type_id'] if dep['known_type'] else 0)
        actual_deps = {(g, h, t) for (g, h), t in normalized_deps.items()}
        expected_deps = {(r['guid'], r['sha1'], r['type_id']) for r in sample['dependencies']}
        if actual_deps != expected_deps:
            raise ValueError(f'Dependencies differ for {sample["sha1"]}')
        sample_dependencies += len(expected_deps)
    report = dict(schema_version=1, toolkit_commit=pin, matched_map_rows=len(rows),
                  matched_farc_rows=len(entries), matched_binary_samples=len(oracle['samples']),
                  matched_normalized_sample_dependencies=sample_dependencies,
                  sample_revisions=sorted(revisions), repeat_byte_identical=bool(args.repeat),
                  inventory_sha256=hashlib.sha256(raw).hexdigest(), inventory_bytes=len(raw),
                  metadata_status_counts=actual['metadata_status_counts'],
                  dependency_resolution_counts=actual['dependency_resolution_counts'],
                  counts=actual['counts'], partial_metadata=actual['partial_metadata'])
    with output.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(report, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(f'Toolkit parity: {len(rows)} MAP rows, {len(entries)} FARC entries, '
          f'{len(oracle["samples"])} binary samples, {sample_dependencies} normalized dependencies')


if __name__ == '__main__':
    main()
