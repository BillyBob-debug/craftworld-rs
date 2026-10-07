"""Sample known FARC resources read-only; keep only diagnostic metadata.

Source: Toolkit d6a2ef80 SerializedResource.process/processDependencies,
ResourceType and Compressor.decompressData; see docs/formats.md.
Encryption markers are reported without payload processing. No game-byte output.
The optional texture oracle loads only lbptoolspy-lite's tex_tools.py in isolation.
"""
import argparse
import collections
import hashlib
import importlib.util
import json
import struct
import zipfile
import zlib
from pathlib import Path


def describe(raw):
    info = dict(magic=raw[:4].decode('ascii',errors='replace'))
    if len(raw) < 4:
        return dict(info, status='truncated')
    if raw[3:4] == b'e':
        return dict(info,status='blocked_encrypted')
    if raw[3:4] == b'b' and len(raw) >= 12:
        revision, dep_offset = struct.unpack_from('>II',raw,4)
        info.update(revision=revision,dependency_offset=dep_offset)
        if revision < 0x109:
            return dict(info,status='unsupported_revision')
        if dep_offset+4 > len(raw) or dep_offset < 12:
            return dict(info,status='invalid_dependency_offset')
        count = struct.unpack_from('>I',raw,dep_offset)[0]
        pos, deps = dep_offset+4, []
        if count > len(raw)//5:
            return dict(info,status='invalid_dependency_count')
        for _ in range(count):
            if pos >= len(raw):
                raise ValueError('Truncated dependency flags')
            flags = raw[pos]; pos += 1
            if flags & ~3:
                raise ValueError('Unknown dependency flags')
            needed = (4 if flags & 2 else 0)+(20 if flags & 1 else 0)+4
            if pos+needed > len(raw):
                raise ValueError('Truncated dependency')
            dep = dict(flags=flags)
            if flags & 2:
                dep['guid'] = struct.unpack_from('>I',raw,pos)[0]; pos += 4
            if flags & 1:
                dep['sha1'] = raw[pos:pos+20].hex(); pos += 20
            dep['type_id'] = struct.unpack_from('>I',raw,pos)[0]; pos += 4
            deps.append(dep)
        info.update(dependency_count=count,dependencies=deps,status='header_and_dependencies_only')
    else:
        info['status'] = 'magic_only'
    return info


def texture_metadata(raw):
    if raw[:4] != b'TEX ':
        return dict(status='unsupported_texture_wrapper')
    if len(raw) < 8:
        raise ValueError('Truncated TEX header')
    flag,count = struct.unpack_from('>HH',raw,4)
    if count == 0 or count > (len(raw)-8)//4:
        return dict(status='unsupported_chunk_layout')
    pos, total, decoded = 8+count*4, 0, bytearray()
    for i in range(count):
        compressed,expanded = struct.unpack_from('>HH',raw,8+i*4)
        total += expanded
        if total > 16*1024*1024 or pos+compressed > len(raw):
            raise ValueError('Texture chunk budget/range invalid')
        chunk = raw[pos:pos+compressed]; pos += compressed
        if compressed == expanded:
            output = chunk
        else:
            inflater = zlib.decompressobj()
            output = inflater.decompress(chunk,expanded+1)
            if not inflater.eof or inflater.unconsumed_tail or inflater.unused_data:
                raise ValueError('Invalid compressed chunk')
        if len(output) != expanded:
            raise ValueError('Texture chunk length mismatch')
        decoded.extend(output)
    if pos != len(raw) or len(decoded) < 128 or decoded[:4] != b'DDS ':
        raise ValueError('Unexpected DDS layout')
    height,width = struct.unpack_from('<II',decoded,12)
    return dict(status='dds_metadata_only',flag=flag,chunks=count,width=width,height=height,
                fourcc=decoded[84:88].decode('ascii',errors='replace'),dds_sha256=hashlib.sha256(decoded).hexdigest())


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('zip',type=Path)
    ap.add_argument('--output',type=Path,default=Path('local-analysis/resource-samples.json'))
    args=ap.parse_args()
    root=Path(__file__).resolve().parents[1]
    output=args.output.resolve()
    if not output.is_relative_to(root/'local-analysis') or output.exists():
        ap.error('Use a new output filename inside local-analysis/')
    inv=json.loads((root/'local-analysis/disc-inventory.json').read_text())
    tables=json.loads((root/'local-analysis/lbp-tables.json').read_text())
    archive=next(e for e in inv['entries'] if e['path'].endswith('/DATA.FARC'))
    farc={r['sha1']:r for r in tables['farc_rows']}
    choices=[]
    for ext in ['.tex','.mol','.gmat','.anim','.bin','.plan','.mat','.trans','.smh']:
        candidates=sorted((r for r in tables['map_rows'] if r['path'].endswith(ext) and r['sha1'] in farc and 16<=farc[r['sha1']]['size']<=1024*1024),key=lambda r:(farc[r['sha1']]['size'],r['path']))
        choices.extend(candidates[:2])
    # Include ordinary atlas textures as well as smallest placeholder resources.
    choices.extend(r for r in tables['map_rows'] if r['path'] in ('gamedata/data/ui.tex','gamedata/data/ui2.tex','gamedata/scratch/sponge_diffuse.tex') and r['sha1'] in farc)
    choices=list({r['sha1']:r for r in choices}.values())
    choices.sort(key=lambda r:farc[r['sha1']]['offset'])
    results=[]
    with zipfile.ZipFile(args.zip,'r') as z:
        member=z.getinfo(inv['iso_member'])
        if member.flag_bits & 1 or member.file_size != inv['iso_size'] or f'{member.CRC:08x}' != inv['zip_crc32']:
            raise ValueError('Encrypted ZIP or input identity mismatch')
        with z.open(member,'r') as stream:
            position=0
            for row in choices:
                entry=farc[row['sha1']]; start=archive['lba']*2048+entry['offset']
                while position<start:
                    chunk=stream.read(min(1024*1024,start-position))
                    if not chunk:raise ValueError('Truncated ISO')
                    position+=len(chunk)
                raw=stream.read(entry['size']); position+=len(raw)
                if len(raw)!=entry['size']:raise ValueError('Truncated resource')
                item=dict(path=row['path'],sha1=row['sha1'],size=entry['size'],**describe(raw))
                item['hash_matches']=hashlib.sha1(raw).hexdigest()==row['sha1']
                if not item['hash_matches']:
                    raise ValueError('Sample hash mismatch; stop instead of trusting parsed metadata')
                if raw[:4]==b'GTF ' and len(raw)>=28:
                    item['texture']=dict(status='gtf_header_only',format=raw[4],mipmaps=raw[5],dimension=raw[6],cubemap=raw[7],width=struct.unpack_from('>H',raw,12)[0],height=struct.unpack_from('>H',raw,14)[0],depth=struct.unpack_from('>H',raw,16)[0])
                if raw[:4]==b'TEX ':
                    item['texture']=texture_metadata(raw)
                    oracle_path=root/'references/lbptoolspy-lite/src/lbptoolspy/tex_tools.py'
                    if oracle_path.exists():
                        spec=importlib.util.spec_from_file_location('texture_oracle',oracle_path)
                        module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
                        image=module.tex2image(raw); image.load()
                        item['oracle']=dict(tool='Zhaxxy/lbptoolspy-lite tex_tools.tex2image',width=image.width,height=image.height,mode=image.mode,pixel_sha256=hashlib.sha256(image.tobytes()).hexdigest(),dimensions_agree=(image.width,image.height)==(item['texture']['width'],item['texture']['height']))
                results.append(item)
    report=dict(schema_version=1,samples=results,streamed_iso_bytes=position)
    output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
    print(json.dumps(dict(sample_count=len(results),magics=dict(collections.Counter(r['magic'] for r in results)),all_hashes_match=all(r['hash_matches'] for r in results),oracles=[r.get('oracle') for r in results if 'oracle' in r]),indent=2))


if __name__=='__main__':main()
