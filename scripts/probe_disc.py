"""Read-only, bounded ISO9660 metadata probe; no extraction/decryption.

This is a reconnaissance tool, not the future LBP resource scanner. ISO directory
records use ECMA-119 offsets. Only the first --prefix-mib of one ISO ZIP member
are decompressed; directory extents outside this window are reported as missing.
"""
import argparse
import collections
import hashlib
import json
import struct
import zipfile
from pathlib import Path


def inventory(data):
    pvd = data[16 * 2048:17 * 2048]
    if pvd[:7] != b'\x01CD001\x01':
        raise ValueError('No ISO9660 primary descriptor; stop, do not decrypt')
    entries, pending, seen, missing = [], [('', pvd[156:190])], set(), []
    while pending:
        parent, rec = pending.pop()
        lba = struct.unpack_from('<I', rec, 2)[0]
        size = struct.unpack_from('<I', rec, 10)[0]
        if (lba, size) in seen:
            continue
        seen.add((lba, size))
        start = lba * 2048
        if start + size > len(data):
            missing.append(parent)
            continue
        buf, pos = data[start:start+size], 0
        while pos < len(buf):
            n = buf[pos]
            if not n:
                pos = (pos // 2048 + 1) * 2048
                continue
            if n < 34 or pos+n > len(buf):
                raise ValueError('Invalid directory record')
            r = buf[pos:pos+n]
            pos += n
            name = r[33:33+r[32]]
            if name in (b'\0', b'\1'):
                continue
            name = name.decode('ascii', errors='replace').split(';')[0]
            path = parent + '/' + name
            item = dict(path=path, lba=struct.unpack_from('<I', r, 2)[0],
                        size=struct.unpack_from('<I', r, 10)[0],
                        directory=bool(r[25] & 2), multi_extent=bool(r[25] & 128))
            entries.append(item)
            if item['directory']:
                pending.append((path, r))
    return dict(volume=pvd[40:72].decode('ascii').strip(),
                entries=sorted(entries, key=lambda x:(x['path'],x['lba'])),
                missing_directories=sorted(missing))


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('zip', type=Path)
    ap.add_argument('--prefix-mib', type=int, default=16)
    args = ap.parse_args()
    if not 1 <= args.prefix_mib <= 64:
        ap.error('prefix budget must be 1..64 MiB')
    with zipfile.ZipFile(args.zip, 'r') as z:
        infos = z.infolist()
        isos = [i for i in infos if i.filename.lower().endswith('.iso')]
        if len(isos) != 1:
            raise ValueError('Expected exactly one ISO')
        member = isos[0]
        if member.flag_bits & 1:
            raise ValueError('Encrypted ZIP: stop')
        with z.open(member, 'r') as f:
            data = f.read(args.prefix_mib * 1024 * 1024)
    result = inventory(data)
    result.update(zip_size=args.zip.stat().st_size, iso_size=member.file_size,
                  iso_member=member.filename, zip_crc32=f'{member.CRC:08x}',
                  prefix_bytes=len(data), prefix_sha256=hashlib.sha256(data).hexdigest())
    result['extensions'] = dict(sorted(collections.Counter(
        Path(e['path']).suffix.lower() or '(none)' for e in result['entries']
        if not e['directory']).items()))
    # Small metadata only, never output file contents or decode protected payload.
    result['metadata_probes'] = []
    for e in result['entries']:
        if e['path'].endswith(('PARAM.SFO','PS3_DISC.SFB','EBOOT.BIN','BLURAYGUIDS.MAP')):
            off = e['lba'] * 2048
            if off + min(e['size'], 16) <= len(data):
                item = dict(path=e['path'], prefix_hex=data[off:off+16].hex())
                if off + e['size'] <= len(data) and e['size'] <= 65536:
                    raw = data[off:off+e['size']]
                    item['sha256'] = hashlib.sha256(raw).hexdigest()
                    if raw[:4] == b'\0PSF':
                        _, _, keys, values, count = struct.unpack_from('<5I', raw)
                        fields = {}
                        for i in range(count):
                            ko, fmt, length, maximum, vo = struct.unpack_from('<HHIII', raw, 20+i*16)
                            key = raw[keys+ko:].split(b'\0')[0].decode('utf8')
                            value = raw[values+vo:values+vo+length]
                            fields[key] = struct.unpack('<I',value)[0] if fmt == 0x404 else value.rstrip(b'\0').decode('utf8',errors='replace')
                        item['sfo'] = fields
                result['metadata_probes'].append(item)
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == '__main__':
    main()
