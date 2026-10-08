"""Synthetic snapshot/oracle metadata only; no original game fixtures."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
import uuid

spec = importlib.util.spec_from_file_location('level_report', Path(__file__).parents[1] / 'scripts/level_report.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def setUpModule():
    Path('local-analysis').mkdir(exist_ok=True)


def resource(path, guid, sha, kind='level', cands=1):
    return {'map_row': {'path': path, 'guid': guid, 'sha1': sha}, 'confirmed_type': kind,
            'archive_candidates': [{'archive': 'USRDIR/DATA.FARC', 'offset': 10, 'size': 20,
                                    'inspection_index': 0}] * cands}


SNAPSHOT = {
    'source_scope': 'synthetic',
    'resource_inspections': [{'inspection': {'header': {'revision': 950, 'branch_id': 0}}}],
    'resources': [resource('b/level.bin', '00000002', 'b' * 40), resource('a/level.bin', '00000001', 'a' * 40),
                  resource('c/dup.bin', '00000003', 'c' * 40, cands=2),
                  resource('m/mesh.mol', '00000010', 'd' * 40, kind='mesh'),
                  resource('g/mat.gmat', '00000020', 'e' * 40, kind='graphics_material'),
                  resource('v/edge.bev', '00000030', 'f' * 40, kind='bevel')],
}


def level(path, status='parsed_exact'):
    row = {'map_path': path, 'guid': '00000001', 'sha1': 'a' * 40, 'stored_bytes': 20, 'status': status}
    if status != 'parsed_exact':
        row['reason'] = 'synthetic'
        return row
    row.update(header={'head': 950, 'branch_id': 0, 'compression_flags': 7}, expanded_bytes=99, trailing_bytes=0,
               thing_count=3, max_thing_depth=2, reference_tag_count=5, cross_play_hash_count=0,
               part_histogram={'POS': 3, 'RENDER_MESH': 2, 'SHAPE': 1, 'GENERATED_MESH': 1},
               render_mesh_descriptors={'g00000010': 1, 'g00000099': 1}, gfx_material_descriptors={'g00000020': 1},
               bevel_descriptors={'g00000030': 1}, shape_material_descriptors={'null': 1}, ref_plan_descriptors={},
               bone_things_length_histogram={'0': 1, '1': 1},
               counts={'render_mesh': 2, 'shape': 1, 'generated_mesh': 1, 'shape_and_generated_mesh': 1,
                       'shape_vertices_total': 4, 'distinct_shape_materials': 1, 'distinct_bevels': 1,
                       'distinct_ref_plans': 0})
    return row


class LevelReportTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.outputs = []

    def tearDown(self):
        for p in self.outputs:
            if os.path.exists(p):
                os.remove(p)
        self.tmp.cleanup()

    def put(self, name, value):
        p = os.path.join(self.tmp.name, name)
        with open(p, 'w', encoding='utf-8') as f:
            json.dump(value, f)
        return p

    def out(self):
        p = 'local-analysis/test-d106-%s.json' % uuid.uuid4().hex
        self.outputs.append(p)
        return p

    def test_manifest_sorted_scene_flag_and_ambiguous_skipped(self):
        snap = self.put('snap.json', SNAPSHOT)
        o = self.out()
        module.main(['manifest', snap, o, '--scene', 'b/level.bin'])
        m = json.load(open(o, encoding='utf-8'))
        self.assertEqual(m['toolkit_commit'], module.PIN)
        self.assertEqual([s['map_path'] for s in m['samples']], ['a/level.bin', 'b/level.bin'])
        self.assertEqual([s['scene'] for s in m['samples']], [False, True])
        self.assertEqual(m['skipped'], [{'map_path': 'c/dup.bin', 'reason': 'archive candidates 2'}])
        with self.assertRaises(SystemExit):
            module.main(['manifest', snap, o])  # refuses to overwrite
        only = self.out()
        module.main(['manifest', snap, only, '--scene', 'b/level.bin', '--only-scene'])
        self.assertEqual([s['map_path'] for s in json.load(open(only, encoding='utf-8'))['samples']], ['b/level.bin'])
        with self.assertRaises(SystemExit):
            module.main(['manifest', snap, self.out(), '--scene', 'missing.bin'])

    def test_summary_counts_and_resolution(self):
        snap = self.put('snap.json', SNAPSHOT)
        oracle = self.put('oracle.json', {'toolkit_commit': module.PIN, 'scope': module.SCOPE,
                                          'levels': [level('x/a.bin'), level('x/b.bin', 'failed')]})
        corpus = self.put('corpus.json', {'viewable_guid_list': ['00000010'], 'all_diffuse_guid_list': []})
        routes = self.put('routes.json', {'materials_detail': [{'sha1': 'e' * 40, 'class': 'texture_route'}]})
        o = self.out()
        module.main(['summary', oracle, snap, corpus, routes, o])
        r = json.load(open(o, encoding='utf-8'))
        self.assertEqual(r['status_counts'], {'failed': 1, 'parsed_exact': 1})
        self.assertEqual(r['render_mesh']['distinct_guids'], 2)
        self.assertEqual(r['render_mesh']['viewable'], 1)
        self.assertEqual(r['render_mesh']['descriptor_resolution'], {'catalog_type:mesh': 1, 'guid_not_in_catalog': 1})
        self.assertEqual(r['gfx_materials']['route_classes'], {'texture_route': 1})
        self.assertEqual(r['bevels']['catalog_bevel_resources'], 1)
        self.assertEqual(r['levels'][0]['render_mesh_instances_viewable'], 1)
        # Deterministic: same inputs give byte-identical output.
        o2 = self.out()
        module.main(['summary', oracle, snap, corpus, routes, o2])
        self.assertEqual(open(o, 'rb').read(), open(o2, 'rb').read())
        bad = self.put('bad.json', {'toolkit_commit': 'x', 'scope': module.SCOPE, 'levels': []})
        with self.assertRaises(SystemExit):
            module.main(['summary', bad, snap, corpus, routes, self.out()])


if __name__ == '__main__':
    unittest.main()
