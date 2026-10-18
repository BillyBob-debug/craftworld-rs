"""Run the image parity checks listed in tests/parity/manifest.json.

Each check renders one viewer still (or reuses one already rendered), compares it with a
game capture inside a region and fails when the mean absolute difference, in levels of
255 averaged over the region's pixels and RGB channels, is above the check's threshold.
Captures, masks and renders live under the ignored local-analysis/ folder and are never
committed; a check whose reference or mask is missing is reported as "skip", not "fail".

Usage (from the repo root, or pass --root):
  python scripts/run_parity.py --exe target/release/lbp-bevy-viewer.exe --tag pr150
  python scripts/run_parity.py --tag pr150 --no-render          # compare existing renders
  python scripts/run_parity.py --only d324- --tag pr150 --json  # one JSON line per check

Prints one line per check and a summary line; exit status 0 when nothing failed, 1 when
a check failed, 2 on a manifest or usage error.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

DEFAULT_MANIFEST = Path("tests/parity/manifest.json")
RENDER_DIR = "local-analysis/parity/renders"
RENDER_TIMEOUT_S = 240


class ManifestError(ValueError):
    pass


def load_manifest(path: Path) -> dict:
    manifest = json.loads(path.read_text(encoding="utf-8"))
    validate_manifest(manifest)
    return manifest


def validate_manifest(manifest: dict) -> None:
    presets = manifest.get("presets", {})
    if not isinstance(presets, dict):
        raise ManifestError("presets must be an object")
    for name, args in presets.items():
        if not isinstance(args, list) or not all(isinstance(a, str) for a in args):
            raise ManifestError(f"preset {name}: must be a list of strings")
    seen = set()
    for check in manifest.get("checks", []):
        cid = check.get("id")
        if not isinstance(cid, str) or not cid:
            raise ManifestError(f"check without id: {check}")
        if cid in seen:
            raise ManifestError(f"duplicate check id {cid}")
        seen.add(cid)
        preset = check.get("preset")
        if preset is not None and preset not in presets:
            raise ManifestError(f"{cid}: unknown preset {preset}")
        if not isinstance(check.get("args", []), list):
            raise ManifestError(f"{cid}: args must be a list")
        for key in ("reference",):
            if not str(check.get(key, "")).startswith("local-analysis/"):
                raise ManifestError(f"{cid}: {key} must be a path under local-analysis/")
        region = check.get("region", "viewport")
        if region != "viewport":
            if not isinstance(region, dict) or len(region) != 1 or not ({"box", "mask"} & region.keys()):
                raise ManifestError(f'{cid}: region must be "viewport", {{"box": [x, y, w, h]}} or {{"mask": path}}')
            if "box" in region and (len(region["box"]) != 4 or min(region["box"][2:]) <= 0):
                raise ManifestError(f"{cid}: box must be [x, y, w, h] with positive size")
            if "mask" in region and not str(region["mask"]).startswith("local-analysis/"):
                raise ManifestError(f"{cid}: mask must be a path under local-analysis/")
        threshold = check.get("max_mean_abs")
        if not isinstance(threshold, (int, float)) or threshold < 0:
            raise ManifestError(f"{cid}: max_mean_abs must be a non-negative number")


def viewer_args(manifest: dict, check: dict, out_png: str) -> list[str]:
    args = list(manifest.get("presets", {}).get(check.get("preset"), []))
    args += [str(a) for a in check.get("args", [])]
    return args + ["--qa-capture", out_png]


def render_path(tag: str, check: dict) -> str:
    return f"{RENDER_DIR}/{tag}/{check['id']}.png"


def render(exe: Path, root: Path, args: list[str], out_png: Path) -> str | None:
    """Run the viewer hidden; return an error string, or None when the PNG was written."""
    out_png.parent.mkdir(parents=True, exist_ok=True)
    log = out_png.with_suffix(".log")
    flags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    with log.open("w", encoding="utf-8", errors="replace") as handle:
        try:
            proc = subprocess.run([str(exe), *args], cwd=root, stdout=handle, stderr=subprocess.STDOUT,
                                  timeout=RENDER_TIMEOUT_S, creationflags=flags)
        except subprocess.TimeoutExpired:
            return f"viewer timed out after {RENDER_TIMEOUT_S} s"
    if not out_png.exists():
        return f"viewer exit {proc.returncode}, no PNG (see {log.name})"
    return None


def region_mask(region, shape, root: Path):
    import numpy as np

    height, width = shape
    if region == "viewport":
        return np.ones((height, width), bool)
    if "box" in region:
        x, y, w, h = region["box"]
        mask = np.zeros((height, width), bool)
        mask[max(y, 0):y + h, max(x, 0):x + w] = True
        return mask
    from PIL import Image

    image = np.asarray(Image.open(root / region["mask"]).convert("L"))
    if image.shape != (height, width):
        raise ManifestError(f"mask is {image.shape[1]}x{image.shape[0]}, images are {width}x{height}")
    return image > 0


def mean_abs(render_rgb, reference_rgb, mask) -> float:
    import numpy as np

    if render_rgb.shape != reference_rgb.shape:
        raise ManifestError(f"render is {render_rgb.shape[1]}x{render_rgb.shape[0]}, "
                            f"reference is {reference_rgb.shape[1]}x{reference_rgb.shape[0]}")
    if not mask.any():
        raise ManifestError("region is empty")
    diff = np.abs(render_rgb.astype(float) - reference_rgb.astype(float))
    return float(diff[mask].mean())


def load_rgb(path: Path):
    import numpy as np
    from PIL import Image

    return np.asarray(Image.open(path).convert("RGB"))


def run_check(manifest: dict, check: dict, root: Path, tag: str, exe: Path | None) -> dict:
    result = {"id": check["id"], "max": check["max_mean_abs"]}
    reference = root / check["reference"]
    region = check.get("region", "viewport")
    missing = [p for p in [reference] + ([root / region["mask"]] if isinstance(region, dict) and "mask" in region else [])
               if not p.exists()]
    if missing:
        return {**result, "status": "skip", "why": f"missing {missing[0].relative_to(root).as_posix()}"}
    out_rel = render_path(tag, check)
    out_png = root / out_rel
    if exe is not None and not out_png.exists():
        error = render(exe, root, viewer_args(manifest, check, out_rel), out_png)
        if error:
            return {**result, "status": "error", "why": error}
    if not out_png.exists():
        return {**result, "status": "skip", "why": f"no render at {out_rel}"}
    try:
        ref = load_rgb(reference)
        value = mean_abs(load_rgb(out_png), ref, region_mask(region, ref.shape[:2], root))
    except ManifestError as error:
        return {**result, "status": "error", "why": str(error)}
    status = "pass" if value <= check["max_mean_abs"] else "fail"
    return {**result, "status": status, "mean_abs": round(value, 2)}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="repo root (default: cwd)")
    parser.add_argument("--manifest", type=Path, default=None)
    parser.add_argument("--exe", type=Path, help="viewer exe used to render missing stills")
    parser.add_argument("--tag", required=True, help="render folder name, e.g. the PR or commit")
    parser.add_argument("--no-render", action="store_true", help="only compare renders that exist")
    parser.add_argument("--only", default="", help="run checks whose id starts with this prefix")
    parser.add_argument("--json", action="store_true", help="print one JSON object per check")
    opts = parser.parse_args(argv)
    root = opts.root.resolve()
    try:
        manifest = load_manifest(opts.manifest or root / DEFAULT_MANIFEST)
    except (OSError, json.JSONDecodeError, ManifestError) as error:
        print(f"manifest error: {error}", file=sys.stderr)
        return 2
    if not opts.no_render and opts.exe is None:
        print("pass --exe to render, or --no-render to compare existing stills", file=sys.stderr)
        return 2
    exe = None if opts.no_render else opts.exe.resolve()
    counts = {"pass": 0, "fail": 0, "skip": 0, "error": 0}
    for check in manifest.get("checks", []):
        if not check["id"].startswith(opts.only):
            continue
        result = run_check(manifest, check, root, opts.tag, exe)
        counts[result["status"]] += 1
        if opts.json:
            print(json.dumps(result))
        else:
            detail = f'{result["mean_abs"]:.2f} <= {result["max"]}' if "mean_abs" in result else result["why"]
            if result["status"] == "fail":
                detail = f'{result["mean_abs"]:.2f} > {result["max"]}'
            print(f'{result["status"]:5s} {result["id"]}: {detail}')
    print("summary " + " ".join(f"{k} {v}" for k, v in counts.items()))
    return 1 if counts["fail"] or counts["error"] else 0


if __name__ == "__main__":
    sys.exit(main())
