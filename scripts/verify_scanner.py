"""Compare A1 JSON with the earlier local Python table probe, without asset reads.

This is independent-reader corroboration, not a community Java oracle.
Reports belong under ignored local-analysis; no game/reference data is bundled.
"""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('inventory', type=Path)
    ap.add_argument('--repeat', type=Path, action='append', default=[])
    ap.add_argument('--reference', type=Path, required=True)
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    root = Path(__file__).resolve().parents[1]
    output = args.output.resolve()
    if not output.is_relative_to((root / 'local-analysis').resolve()):
        ap.error('Report must be inside ignored local-analysis/')
    if output.exists():
        ap.error('Report exists; choose a new filename')
    raw = args.inventory.read_bytes()
    for repeat in args.repeat:
        if repeat.read_bytes() != raw:
            raise ValueError(f'Inventory differs: {repeat}')
    actual = json.loads(raw)
    reference = json.loads(args.reference.read_text(encoding='utf-8-sig'))
    if actual['schema_version'] != 1 or actual['milestone'] != 'A1_directory_tables':
        raise ValueError('Unsupported scanner schema/milestone')
    rows = sorted(actual['resources'], key=lambda r: r['map_row']['row_index'])
    normalized = [dict(guid=int(r['map_row']['guid'], 16), path=r['map_row']['path'],
                       sha1=r['map_row']['sha1'], size=r['map_row']['declared_size'],
                       timestamp=r['map_row']['timestamp_raw']) for r in rows]
    if normalized != reference['map_rows']:
        raise ValueError('MAP fields differ')
    if len(actual['archives']) != 1:
        raise ValueError('This local acceptance check expects one archive')
    archive = actual['archives'][0]
    entries = [dict(offset=r['offset'], sha1=r['sha1'], size=r['size'])
               for r in archive['entries']]
    if entries != reference['farc_rows'] or archive['table_offset'] != reference['farc_table_offset']:
        raise ValueError('FARC fields differ')
    if actual['map']['sha256'] != reference['map_sha256'] or actual['map']['revision'] != reference['map_revision']:
        raise ValueError('MAP identity differs')
    if actual['map_extension_counts'] != reference['map_extensions']:
        raise ValueError('Extension counts differ')
    if actual['counts']['map_rows_with_archive_candidates'] != reference['map_rows_resolved_in_farc']:
        raise ValueError('Archive joins differ')
    report = dict(schema_version=1, verification='independent_python_probe_not_java_oracle',
                  matched_map_rows=len(normalized), matched_farc_rows=len(entries),
                  byte_identical_inventories=1 + len(args.repeat), inventory_bytes=len(raw),
                  inventory_sha256=hashlib.sha256(raw).hexdigest(), counts=actual['counts'])
    with output.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(report, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(f'Full field parity: {len(normalized)} MAP rows, {len(entries)} FARC rows; '
          f'{1 + len(args.repeat)} byte-identical inventories')


if __name__ == '__main__':
    main()
