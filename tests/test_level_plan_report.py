"""Synthetic snapshot metadata only; no original game fixtures."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('level_plan_report', Path(__file__).parents[1] / 'scripts/level_plan_report.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def setUpModule():
    # Ignored local-analysis/ is absent in a fresh clone; the private writers require it.
    Path('local-analysis').mkdir(exist_ok=True)


def row(kind='plan', revision=919, size=30, offset=1, status='parsed'):
    return {'source_path': 'USRDIR/DATA.FARC', 'offset': offset, 'size': size, 'sha1': f'{offset:040x}',
            'inspection': {'confirmed_type': {'name': kind}, 'magic_hex': '504c4e62' if kind == 'plan' else '4c564c62',
                           'status': 'metadata_parsed', 'dependency_status': status,
                           'dependencies': [{'type_id': 1}] if status == 'parsed' else [],
                           'header': {'revision': revision, 'branch_id': 0, 'branch_revision': 0,
                                      'compression_flags': 7, 'compressed': True}}}


class CensusTests(unittest.TestCase):
    def test_aliases_magic_dependencies_and_order_are_distinct(self):
        rows = [row(), row(size=60, offset=2), row(kind='level', offset=3, status='unsupported')]
        s = {'resources': [{'map_row': {'path': 'authored/not-a-level.bin', 'sha1': 'a'*40}}]*2,
             'resource_inspections': rows}
        a, manifest = module.census(s)
        self.assertEqual(a['types']['level']['extension_alias_rows'], 2)
        self.assertEqual(a['types']['level']['extension_unique_hashes'], 1)
        self.assertEqual(a['types']['level']['confirmed_locations'], 1)
        self.assertEqual(a['types']['level']['parsed_dependency_entries'], 0)
        self.assertEqual(a['types']['plan']['parsed_dependency_entries'], 2)
        self.assertEqual([r['offset'] for r in manifest['samples']], [1, 2])
        s['resource_inspections'].reverse()
        self.assertEqual((a, manifest), module.census(s))

    def test_old_branch_budget_and_wrong_magic_are_counted_without_sampling(self):
        rows = [row(revision=622), row(size=module.MAX_SAMPLE+1, offset=2), row(offset=3), row(offset=4)]
        rows[2]['inspection']['header']['branch_id'] = 1
        rows[3]['inspection']['magic_hex'] = '504c4e65'
        a, manifest = module.census({'resources': [], 'resource_inspections': rows})
        self.assertEqual(a['types']['plan']['confirmed_locations'], 4)
        self.assertEqual(manifest['samples'], [])

    def test_sample_count_budget_and_omitted_coverage_are_explicit(self):
        rows = [row(revision=i, offset=i) for i in range(893, 952)]
        a, manifest = module.census({'resources': [], 'resource_inspections': rows})
        self.assertEqual(a['types']['plan']['confirmed_locations'], 59)
        self.assertEqual(len(manifest['samples']), 48)
        self.assertEqual(manifest['represented_cohorts'], 48)
        self.assertEqual(len(manifest['omitted_eligible_cohorts']), 11)
        rows = [row(revision=i, size=module.MAX_SAMPLE, offset=i) for i in range(893, 952)]
        _, manifest = module.census({'resources': [], 'resource_inspections': rows})
        self.assertEqual(manifest['selected_stored_bytes'], module.MAX_TOTAL)
        self.assertEqual(len(manifest['samples']), 32)

    def test_private_json_new_only_writer(self):
        with self.assertRaises(ValueError):
            module.write('docs/not-an-output.json', {})
        with tempfile.TemporaryDirectory(dir='local-analysis') as folder:
            path = Path(folder)/'synthetic.json'
            module.write(path, {'synthetic': True})
            with self.assertRaises(FileExistsError):
                module.write(path, {})
            with self.assertRaises(ValueError):
                module.write(Path(folder)/'wrong.txt', {})


if __name__ == '__main__':
    unittest.main()
