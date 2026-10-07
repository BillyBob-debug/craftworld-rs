"""Copy exactly the three already readable A1 inputs from the owned ZIP/ISO.

No disc mounting, protected file access, whole-disc extraction or FARC unpacking.
Outputs must be a new directory under this project's ignored game-data/. Original
input is opened read-only. Incomplete preparation remains in an ignored staging dir.
"""
import argparse
import contextlib
import hashlib
import json
import os
import shutil
import stat
import uuid
import zipfile
from pathlib import Path

from probe_disc import inventory
from probe_lbp_tables import parse_map, parse_farc_tail


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('zip', type=Path)
    ap.add_argument('--output', type=Path, default=Path('game-data/a1-disc'))
    args = ap.parse_args()
    root = Path(__file__).resolve().parents[1]
    destination = args.output.resolve()
    game_root = (root / 'game-data').resolve()
    if not destination.is_relative_to(game_root) or destination == game_root or destination.exists():
        ap.error('Use a new subdirectory inside ignored game-data/')
    expected = json.loads((root / 'local-analysis/disc-inventory.json').read_text())
    allowed = ['/PS3_GAME/PARAM.SFO', '/PS3_GAME/USRDIR/OUTPUT/BLURAYGUIDS.MAP', '/PS3_GAME/USRDIR/DATA.FARC']
    with zipfile.ZipFile(args.zip, 'r') as archive:
        member = archive.getinfo(expected['iso_member'])
        if member.flag_bits & 1 or member.file_size != expected['iso_size'] or f'{member.CRC:08x}' != expected['zip_crc32']:
            raise ValueError('Encrypted ZIP or identity differs from reconnaissance')
        with archive.open(member, 'r') as stream:
            actual = inventory(stream.read(40 * 1024 * 1024))
        entries = {e['path']: e for e in actual['entries']}
        selected = [entries[path] for path in allowed]
        if any(e['directory'] or e['multi_extent'] for e in selected):
            raise ValueError('Only single-extent regular inputs supported')
        total = sum(e['size'] for e in selected)
        if total > 2 * 1024**3 or shutil.disk_usage(root).free < total + 64 * 1024**2:
            raise ValueError('Preparation byte/disk budget exceeded')
        game_root.mkdir(exist_ok=True)
        staging = game_root / ('.prepare-' + uuid.uuid4().hex)
        staging.mkdir()
        copied = {e['path']: 0 for e in selected}
        stop = max(e['lba'] * 2048 + e['size'] for e in selected)
        with contextlib.ExitStack() as handles:
            outputs = {}
            for entry in selected:
                target = staging / entry['path'].lstrip('/')
                target.parent.mkdir(parents=True, exist_ok=True)
                outputs[entry['path']] = handles.enter_context(target.open('xb'))
            stream = handles.enter_context(archive.open(member, 'r'))
            position = 0
            while position < stop:
                chunk = stream.read(min(1024 * 1024, stop - position))
                if not chunk:
                    raise ValueError('Truncated ISO')
                end = position + len(chunk)
                for entry in selected:
                    start = entry['lba'] * 2048
                    extent_end = start + entry['size']
                    if position < extent_end and end > start:
                        fragment = chunk[max(0, start-position):min(len(chunk), extent_end-position)]
                        outputs[entry['path']].write(fragment)
                        copied[entry['path']] += len(fragment)
                position = end
        if any(copied[e['path']] != e['size'] for e in selected):
            raise ValueError('Copy length differs from ISO extent')
    sfo = staging / allowed[0].lstrip('/')
    mapping = staging / allowed[1].lstrip('/')
    farc = staging / allowed[2].lstrip('/')
    sfo_hash = hashlib.sha256(sfo.read_bytes()).hexdigest()
    map_bytes = mapping.read_bytes()
    map_hash = hashlib.sha256(map_bytes).hexdigest()
    if sfo_hash != 'b6491a83d66901a7fc18bb494dc2448811046d959de7a8cb9aa85ef4e9b99d39' or map_hash != '51de580f2badf74db647999b212afbc3fb509d4c389adeb0ef14008cb4398350':
        raise ValueError('Metadata fingerprints differ from recorded owned build')
    revision, rows = parse_map(map_bytes)
    with farc.open('rb') as f:
        f.seek(-16 * 1024 * 1024, 2)
        table_offset, archive_rows = parse_farc_tail(f.read(), farc.stat().st_size)
    for path in [sfo, mapping, farc]:
        path.chmod(stat.S_IREAD)
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        raise ValueError('Destination appeared during preparation; not overwriting')
    os.rename(staging, destination)
    print(json.dumps(dict(output=str(destination),copied_bytes=total,files=allowed,
                         sfo_sha256=sfo_hash,map_sha256=map_hash,map_revision=revision,
                         map_rows=len(rows),farc_entries=len(archive_rows),table_offset=table_offset),indent=2))


if __name__ == '__main__':
    main()
