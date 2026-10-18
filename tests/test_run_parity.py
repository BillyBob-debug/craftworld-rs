import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import run_parity  # noqa: E402

HAVE_IMAGING = all(importlib.util.find_spec(m) for m in ("numpy", "PIL"))


def check(**overrides):
    base = {"id": "c1", "reference": "local-analysis/ref.png", "region": "viewport", "max_mean_abs": 5}
    return {**base, **overrides}


class ManifestTests(unittest.TestCase):
    def test_committed_manifest_is_valid(self):
        manifest = run_parity.load_manifest(ROOT / run_parity.DEFAULT_MANIFEST)
        self.assertTrue(manifest["checks"])

    def test_rejects_bad_checks(self):
        bad = [
            check(reference="captures/ref.png"),
            check(region={"box": [0, 0, 0, 4]}),
            check(region={"mask": "C:/elsewhere/mask.png"}),
            check(region={"circle": 3}),
            check(max_mean_abs=-1),
            check(preset="missing"),
        ]
        for entry in bad:
            with self.assertRaises(run_parity.ManifestError, msg=entry):
                run_parity.validate_manifest({"checks": [entry]})
        with self.assertRaises(run_parity.ManifestError):
            run_parity.validate_manifest({"checks": [check(), check()]})

    def test_viewer_args_append_capture(self):
        manifest = {"presets": {"p": ["--a", "1"]}}
        args = run_parity.viewer_args(manifest, check(preset="p", args=["--b"]), "local-analysis/x.png")
        self.assertEqual(args, ["--a", "1", "--b", "--qa-capture", "local-analysis/x.png"])

    def test_missing_reference_is_skipped(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = run_parity.run_check({}, check(), Path(tmp), "t", None)
        self.assertEqual(result["status"], "skip")


@unittest.skipUnless(HAVE_IMAGING, "numpy and Pillow not installed")
class CompareTests(unittest.TestCase):
    def write(self, root, rel, rgb):
        import numpy as np
        from PIL import Image

        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        Image.fromarray(np.asarray(rgb, np.uint8)).save(path)

    def test_box_and_mask_regions(self):
        import numpy as np

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            ref = np.zeros((4, 6, 3))
            out = ref.copy()
            out[0:2, 0:2] = 30
            self.write(root, "local-analysis/ref.png", ref)
            self.write(root, "local-analysis/parity/renders/t/c1.png", out)
            mask = np.zeros((4, 6, 3))
            mask[0, 0] = 255
            self.write(root, "local-analysis/mask.png", mask)
            cases = [("viewport", 5.0, "fail"), ({"box": [0, 0, 2, 2]}, 30.0, "fail"),
                     ({"box": [3, 0, 3, 4]}, 0.0, "pass"), ({"mask": "local-analysis/mask.png"}, 30.0, "fail")]
            for region, value, status in cases:
                result = run_parity.run_check({}, check(region=region, max_mean_abs=4), root, "t", None)
                self.assertEqual((result["mean_abs"], result["status"]), (value, status), region)

    def test_size_mismatch_is_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write(root, "local-analysis/ref.png", [[[0, 0, 0]] * 4] * 4)
            self.write(root, "local-analysis/parity/renders/t/c1.png", [[[0, 0, 0]] * 2] * 2)
            result = run_parity.run_check({}, check(), root, "t", None)
        self.assertEqual(result["status"], "error")

    def test_main_exit_status(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write(root, "local-analysis/ref.png", [[[10, 10, 10]] * 2] * 2)
            self.write(root, "local-analysis/parity/renders/t/c1.png", [[[12, 12, 12]] * 2] * 2)
            manifest = root / "m.json"
            for limit, code in ((2, 0), (1, 1)):
                manifest.write_text(json.dumps({"checks": [check(max_mean_abs=limit)]}))
                args = ["--root", tmp, "--manifest", str(manifest), "--tag", "t", "--no-render"]
                self.assertEqual(run_parity.main(args), code)


if __name__ == "__main__":
    unittest.main()
