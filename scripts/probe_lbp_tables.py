"""Read-only reconnaissance of MAP and FARC tables inside a ZIP/ISO.

Knowledge source: ennuo/toolkit d6a2ef80, cwlib/types/databases/FileDB.java
(process) and cwlib/types/archives/FileArchive.java (constructor), MIT.
This is an independent bounded metadata probe, not a resource decoder.
Streams only as far as DATA.FARC's end; retains MAP and at most 16 MiB tail.
No game payload is written, decrypted, or exported. JSON remains local.
"""
import argparse
import collections
import hashlib
import json
import struct
import zipfile
from pathlib import Path


def parse_map(raw):
    if len(raw) < 8:
        raise ValueError('Truncated MAP')
    revision, count = struct.unpack_from('>II', raw)
    if revision != 0x100:
        raise ValueError(f'Unsupported MAP revision {revision:#x}')
    if count > len(raw) // 40:
        raise ValueError('Impossible MAP count')
    pos, rows = 8, []
    for _ in range(count):
        if pos + 4 > len(raw):
            raise ValueError('Truncated path length')
        length = struct.unpack_from('>I', raw, pos)[0]
        pos += 4
        if length > 65536 or pos + length + 36 > len(raw):
            raise ValueError('Invalid MAP row length')
        path = raw[pos:pos+length].decode('utf-8', errors='strict')
        pos += length
        timestamp, size = struct.unpack_from('>qI', raw, pos)
        sha1 = raw[pos+12:pos+32].hex()
        guid = struct.unpack_from('>I', raw, pos+32)[0]
        pos += 36
        rows.append(dict(path=path, timestamp=timestamp, size=size, sha1=sha1, guid=guid))
    if pos != len(raw):
        raise ValueError('Unexpected trailing MAP bytes')
    return revision, rows


def parse_farc_tail(tail, size):
    if len(tail) < 8 or tail[-4:] != b'FARC':
        raise ValueError('FARC footer unavailable/invalid; stop without decryption')
    count = struct.unpack_from('>I', tail, len(tail)-8)[0]
    table_size = count * 28
    if table_size + 8 > len(tail) or table_size + 8 > size:
        raise ValueError('FARC table exceeds bounded tail budget')
    table_offset = size - 8 - table_size
    table = tail[len(tail)-8-table_size:len(tail)-8]
    rows = []
    for pos in range(0, len(table), 28):
        offset, length = struct.unpack_from('>II', table, pos+20)
        if offset + length > table_offset:
            raise ValueError('FARC payload range crosses table')
        rows.append(dict(sha1=table[pos:pos+20].hex(), offset=offset, size=length))
    return table_offset, rows


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('zip', type=Path)
    ap.add_argument('--inventory', type=Path, default=Path('local-analysis/disc-inventory.json'))
    ap.add_argument('--output', type=Path, default=Path('local-analysis/lbp-tables.json'))
    args = ap.parse_args()
    root = Path(__file__).resolve().parents[1]
    output = args.output.resolve()
    if not output.is_relative_to(root / 'local-analysis'):
        ap.error('Output must be inside project local-analysis/')
    if output.exists():
        ap.error('Output exists; use a new local-analysis filename')
    inv = json.loads(args.inventory.read_text())
    entries = {e['path']: e for e in inv['entries']}
    m = entries['/PS3_GAME/USRDIR/OUTPUT/BLURAYGUIDS.MAP']
    a = entries['/PS3_GAME/USRDIR/DATA.FARC']
    if m['multi_extent'] or a['multi_extent']:
        raise ValueError('Multi-extent resource containers unsupported')
    map_start, map_end = m['lba']*2048, m['lba']*2048+m['size']
    archive_end = a['lba']*2048+a['size']
    if m['size'] > 16*1024*1024 or archive_end > 2*1024**3:
        raise ValueError('Reconnaissance budget exceeded')
    raw_map, tail, position = bytearray(), bytearray(), 0
    with zipfile.ZipFile(args.zip, 'r') as z:
        member = z.getinfo(inv['iso_member'])
        if member.flag_bits & 1 or member.file_size != inv['iso_size'] or f'{member.CRC:08x}' != inv['zip_crc32']:
            raise ValueError('Encrypted ZIP or input identity differs from inventory')
        with z.open(member, 'r') as f:
            while position < archive_end:
                chunk = f.read(min(1024*1024, archive_end-position))
                if not chunk:
                    raise ValueError('Truncated ISO stream')
                end = position+len(chunk)
                if position < map_end and end > map_start:
                    raw_map.extend(chunk[max(0,map_start-position):min(len(chunk),map_end-position)])
                tail.extend(chunk)
                if len(tail) > 16*1024*1024:
                    del tail[:-16*1024*1024]
                position = end
    revision, maps = parse_map(raw_map)
    table_offset, archives = parse_farc_tail(tail, a['size'])
    hashes = collections.Counter(r['sha1'] for r in archives)
    result = dict(schema_version=1, map_revision=revision, map_count=len(maps),
        map_sha256=hashlib.sha256(raw_map).hexdigest(), farc_count=len(archives),
        farc_table_offset=table_offset, streamed_iso_bytes=position,
        map_extensions=dict(sorted(collections.Counter(Path(r['path']).suffix.lower() or '(none)' for r in maps).items())),
        map_duplicate_guids=len(maps)-len({r['guid'] for r in maps}),
        farc_duplicate_hashes=len(archives)-len(hashes),
        map_rows_resolved_in_farc=sum(r['sha1'] in hashes for r in maps),
        map_rows=maps, farc_rows=archives)
    output.parent.mkdir(exist_ok=True)
    output.write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('map_rows','farc_rows')},indent=2,sort_keys=True))


if __name__ == '__main__':
    main()
