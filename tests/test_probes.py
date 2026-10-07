"""Authored synthetic structures only; no game-byte fixtures or reference code."""
import hashlib
import importlib.util
import io
import struct
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from probe_lbp_tables import parse_map, parse_farc_tail
from probe_resource_samples import describe, texture_metadata


class TableTests(unittest.TestCase):
    def test_map_aliases_preserved_and_unsigned_guid(self):
        rows = []
        for name, guid in [(b'a.tex', 0x80000000), (b'b.tex', 0xffffffff)]:
            rows.append(struct.pack('>I', len(name))+name+struct.pack('>qI', 123, 3)+bytes(20)+struct.pack('>I', guid))
        raw = struct.pack('>II', 0x100, 2)+b''.join(rows)
        rev, parsed = parse_map(raw)
        self.assertEqual(rev, 0x100)
        self.assertEqual([r['guid'] for r in parsed], [0x80000000, 0xffffffff])
        self.assertEqual(parsed[0]['sha1'], parsed[1]['sha1'])
        with self.assertRaises(ValueError): parse_map(raw[:-1])
        with self.assertRaises(ValueError): parse_map(raw+b'x')

    def test_farc_rows_and_bad_ranges(self):
        payload = b'abc'
        digest = hashlib.sha1(payload).digest()
        raw = payload+digest+struct.pack('>III', 0, 3, 1)+b'FARC'
        off, rows = parse_farc_tail(raw, len(raw))
        self.assertEqual(off, 3)
        self.assertEqual(rows, [dict(sha1=digest.hex(), offset=0, size=3)])
        bad = payload+digest+struct.pack('>III', 0, 4, 1)+b'FARC'
        with self.assertRaises(ValueError): parse_farc_tail(bad, len(bad))
        with self.assertRaises(ValueError): parse_farc_tail(struct.pack('>I', 0xffffffff)+b'FARC', 8)
        self.assertEqual(parse_farc_tail(bytes(4)+b'FARC', 8), (0, []))

    def test_revision_and_length_rejection(self):
        for raw in [b'', struct.pack('>II', 0x101, 0), struct.pack('>II', 0x100, 0xffffffff)]:
            with self.assertRaises(ValueError): parse_map(raw)


class ResourceTests(unittest.TestCase):
    def test_both_dependency_ids(self):
        dep = b'\x03'+struct.pack('>I', 0xffffffff)+bytes(range(20))+struct.pack('>I', 2)
        raw = b'PLNb'+struct.pack('>II', 0x338, 12)+struct.pack('>I', 1)+dep
        info = describe(raw)
        self.assertEqual(info['dependencies'], [dict(flags=3, guid=0xffffffff, sha1=bytes(range(20)).hex(), type_id=2)])
        with self.assertRaises(ValueError): describe(raw[:-1])

    def test_encrypted_marker_stops(self):
        self.assertEqual(describe(b'LVLe'+bytes(50))['status'], 'blocked_encrypted')

    def test_stored_tex_chunk_and_expansion_length(self):
        # Synthetic DDS header, no game data or third-party fixture.
        dds = bytearray(128)
        dds[:4] = b'DDS '
        struct.pack_into('<III', dds, 4, 124, 0, 2)
        struct.pack_into('<I', dds, 16, 3)
        dds[84:88] = b'DXT1'
        raw = b'TEX '+struct.pack('>HHHH', 1, 1, 128, 128)+dds
        meta = texture_metadata(raw)
        self.assertEqual((meta['width'], meta['height']), (3, 2))
        with self.assertRaises(ValueError): texture_metadata(raw[:-1])

    def test_external_tex_oracle_synthetic_only(self):
        oracle = ROOT / 'references/lbptoolspy-lite/src/lbptoolspy/tex_tools.py'
        if not oracle.exists() or importlib.util.find_spec('PIL') is None:
            self.skipTest('Optional ignored oracle checkout/Pillow unavailable')
        from PIL import Image
        image = Image.new('RGBA', (2, 3), (30, 60, 90, 255))
        stream = io.BytesIO(); image.save(stream, format='DDS')
        spec = importlib.util.spec_from_file_location('tex_oracle', oracle)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        wrapped = module.compress_dds_lbp(stream.getvalue())
        ours = texture_metadata(wrapped)
        theirs = module.tex2image(wrapped); theirs.load()
        self.assertEqual((ours['width'], ours['height']), theirs.size)
        self.assertEqual(ours['dds_sha256'], hashlib.sha256(stream.getvalue()).hexdigest())
        self.assertEqual(theirs.convert('RGBA').tobytes(), image.tobytes())


if __name__ == '__main__': unittest.main()
