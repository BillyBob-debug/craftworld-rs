"""The public repo carries only authored source and docs.

Fails when a tracked file is a binary, image or game asset, sits under a local-only
folder, or holds decompiled code, key material, a personal path or a personal address.
"""

import re
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SELF = Path(__file__).resolve().relative_to(ROOT).as_posix()

# Mirrors the default-deny .gitignore: only these kinds of file are published.
ALLOWED_SUFFIXES = {
    ".md", ".rs", ".py", ".toml", ".ps1", ".java", ".json", ".yml", ".lock", ".cmd",
}
ALLOWED_NAMES = {".gitignore", "LICENSE", "LICENSE-MIT", "LICENSE-APACHE"}
LOCAL_ONLY_DIRS = ("local-analysis/", "game-data/", "references/", "target/")
MAX_BYTES = 2_000_000

FORBIDDEN = {
    "decompiled code": re.compile(
        r"\bundefined[1248] |\bin_r[0-9]+\b|/\* WARNING: |\(code \*\)"
        r"|\b(?:[pu]?[iu]|pu|b|c|s|f|d)Var[0-9]+\b|\blocal_[0-9a-f]{2,}\b"
    ),
    "key material": re.compile(
        r"(?i)(?:\berk\b|\briv\b|klic|\bkey\b|idps|secret|token)[^\n]{0,24}"
        r"\b(?:0x)?[0-9a-f]{32}"
        r"|\b[0-9a-f]{2}(?:[ ,:]+(?:0x)?[0-9a-f]{2}){15,}\b"
        r"|-----BEGIN [A-Z ]*PRIVATE KEY-----"
    ),
    "personal path": re.compile(r"(?i)\b[a-z]:[\\/]+users[\\/]+(?!<)[^\s\\/'\"`]+|/c/users/"),
    "personal address": re.compile(r"(?i)[a-z0-9._%+-]+@gmail\.com"),
}


def tracked_files():
    out = subprocess.run(
        ["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True
    ).stdout
    return [p for p in out.decode("utf-8").split("\0") if p]


class PublicContentTest(unittest.TestCase):
    def test_tracked_files_are_authored_text(self):
        problems = []
        for rel in tracked_files():
            path = ROOT / rel
            if not path.is_file():
                continue
            name = path.name
            if rel.startswith(LOCAL_ONLY_DIRS):
                problems.append(f"{rel}: under a local-only folder")
                continue
            if name not in ALLOWED_NAMES and path.suffix.lower() not in ALLOWED_SUFFIXES:
                problems.append(f"{rel}: file type not published (binary, image or asset?)")
                continue
            data = path.read_bytes()
            if len(data) > MAX_BYTES:
                problems.append(f"{rel}: {len(data)} bytes, over {MAX_BYTES}")
            if b"\0" in data:
                problems.append(f"{rel}: binary content")
                continue
            try:
                text = data.decode("utf-8")
            except UnicodeDecodeError:
                problems.append(f"{rel}: not UTF-8 text")
                continue
            if rel == SELF:
                continue
            for what, pattern in FORBIDDEN.items():
                m = pattern.search(text)
                if m:
                    line = text.count("\n", 0, m.start()) + 1
                    problems.append(f"{rel}:{line}: {what}")
        self.assertEqual(problems, [], "\n" + "\n".join(problems))


if __name__ == "__main__":
    unittest.main()
