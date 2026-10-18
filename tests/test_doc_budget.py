"""Doc budgets from CLAUDE.md: every thread reads STATUS-CURRENT.md, so it stays short."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).parents[1]
STATUS_CURRENT_MAX_LINES = 80  # CLAUDE.md asks for ~60; this is the hard stop.
DOC_MAX_LINES = 1500  # docs/**/*.md; split long docs per part (docs/level-scripts/part-NN.md).


def line_count(path):
    return len(path.read_text(encoding="utf-8").splitlines())


class DocBudgetTests(unittest.TestCase):
    def test_status_current_is_short(self):
        lines = line_count(ROOT / "STATUS-CURRENT.md")
        self.assertLessEqual(
            lines, STATUS_CURRENT_MAX_LINES,
            "STATUS-CURRENT.md is too long: move older milestone sections to the end of STATUS.md",
        )

    def test_status_current_keeps_no_before_sections(self):
        text = (ROOT / "STATUS-CURRENT.md").read_text(encoding="utf-8")
        found = re.findall(r"^#+ *Before:.*$", text, flags=re.MULTILINE)
        self.assertEqual(found, [], "move \"Before:\" sections to the end of STATUS.md")

    def test_docs_stay_under_budget(self):
        over = sorted(
            f"{path.relative_to(ROOT).as_posix()} ({lines} lines)"
            for path in (ROOT / "docs").rglob("*.md")
            if (lines := line_count(path)) > DOC_MAX_LINES
        )
        self.assertEqual(over, [], f"split docs over {DOC_MAX_LINES} lines")


if __name__ == "__main__":
    unittest.main()
