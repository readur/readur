"""The file-length ratchet's spec, written as assertions.

Run: python3 -m unittest discover -s scripts/ci -p 'test_*.py'
     (or `mise run ratchets-test`)

Two halves, deliberately split. Everything that exercises `evaluate` runs against
hand-built `Finding` values, because that is where the ratchet's POLICY lives.
Everything that exercises the walker runs against a synthesised tmpdir tree, NOT
against the real checkout: a fixture for an exclusion must assert on the
scanner's output, not on the filesystem, or reverting the exclusion leaves the
test green. (For example, "the walker skips .worktrees/" is vacuous against a
fresh CI checkout, which has nothing there.)

Ported from AtvikSecurity tyrfing's `scripts/ci/test_lint_file_length.py`,
adapted to readur's exclusion set.
"""

from __future__ import annotations

import contextlib
import importlib.util
import io
import subprocess
import tempfile
import unittest
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[2]
_SPEC = importlib.util.spec_from_file_location(
    "lint_file_length", _ROOT / "scripts" / "lint-file-length.py"
)
lfl = importlib.util.module_from_spec(_SPEC)
assert _SPEC.loader is not None
_SPEC.loader.exec_module(lfl)


def finding(path: str, lines: int) -> "lfl.Finding":
    return lfl.Finding(path, lines)


def write(root: Path, rel: str, lines: int) -> Path:
    p = root / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text("\n".join("x" for _ in range(lines)), encoding="utf-8")
    return p


def _git_available() -> bool:
    if not (_ROOT / ".git").exists():
        return False
    try:
        subprocess.run(
            ["git", "-C", str(_ROOT), "rev-parse", "--git-dir"],
            capture_output=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return False
    return True


class TestPolicy(unittest.TestCase):
    """`evaluate` -- the whole policy, with no filesystem in sight."""

    def test_a_clean_tree_with_no_baseline_is_ok(self):
        report = lfl.evaluate([finding("a/lib.rs", 12), finding("b/app.tsx", 999)], {})
        self.assertTrue(report.ok())
        self.assertEqual(report.offenders, [])
        self.assertEqual(report.stale, [])
        self.assertEqual(report.unknown, [])

    def test_the_cap_is_inclusive(self):
        at_cap = lfl.evaluate([finding("a/at.rs", lfl.MAX_LINES)], {})
        self.assertEqual(at_cap.offenders, [], "exactly MAX_LINES is at the cap, not over")

        over = lfl.evaluate([finding("a/over.rs", lfl.MAX_LINES + 1)], {})
        self.assertEqual(len(over.offenders), 1)
        self.assertEqual(over.offenders[0].path, "a/over.rs")
        self.assertFalse(over.ok())

    def test_a_baselined_file_over_the_cap_is_not_an_offender(self):
        report = lfl.evaluate([finding("a/huge.rs", 1500)], {"a/huge.rs": 1500})
        self.assertEqual(report.offenders, [])
        self.assertTrue(report.ok())

    def test_a_baselined_file_that_grew_past_its_number_is_a_regression(self):
        """The reason the baseline stores a COUNT and not just a path."""
        report = lfl.evaluate([finding("a/huge.rs", 1501)], {"a/huge.rs": 1500})
        self.assertEqual(report.offenders, [], "it is baselined, so not a NEW offender")
        self.assertEqual(len(report.regressions), 1)
        self.assertEqual(report.regressions[0][0].path, "a/huge.rs")
        self.assertEqual(report.regressions[0][1], 1500)
        self.assertFalse(report.ok())

    def test_a_baselined_file_that_shrank_but_is_still_over_has_slack(self):
        """Slack is invisible otherwise: not an offender, not a regression, not
        stale -- so it accumulates silently as a free move the file can grow
        back into."""
        report = lfl.evaluate([finding("a/huge.rs", 1200)], {"a/huge.rs": 1500})
        self.assertEqual(report.offenders, [])
        self.assertEqual(report.regressions, [])
        self.assertEqual(report.stale, [], "still over the cap, so not stale")
        self.assertEqual(len(report.slack), 1)
        self.assertEqual(report.slack[0][0].path, "a/huge.rs")
        self.assertEqual(report.slack[0][1], 1500, "the frozen number is reported")
        self.assertFalse(report.ok(), "slack is a failure, not a warning")

    def test_a_baselined_file_exactly_at_its_number_has_no_slack(self):
        report = lfl.evaluate([finding("a/huge.rs", 1500)], {"a/huge.rs": 1500})
        self.assertEqual(report.slack, [])
        self.assertTrue(report.ok())

    def test_slack_is_distinguished_from_stale(self):
        report = lfl.evaluate(
            [finding("a/still_over.rs", 1200), finding("a/now_fine.rs", 200)],
            {"a/still_over.rs": 1500, "a/now_fine.rs": 1500},
        )
        self.assertEqual([f.path for f, _ in report.slack], ["a/still_over.rs"])
        self.assertEqual(report.stale, ["a/now_fine.rs"])

    def test_slack_is_reported_worst_first(self):
        report = lfl.evaluate(
            [finding("a/one.rs", 1499), finding("a/ten.rs", 1490)],
            {"a/one.rs": 1500, "a/ten.rs": 1500},
        )
        self.assertEqual([f.path for f, _ in report.slack], ["a/ten.rs", "a/one.rs"])

    def test_a_baseline_entry_whose_file_is_now_under_the_cap_is_stale(self):
        report = lfl.evaluate([finding("a/was_huge.rs", 200)], {"a/was_huge.rs": 1500})
        self.assertEqual(report.stale, ["a/was_huge.rs"])
        self.assertEqual(report.offenders, [])
        self.assertFalse(report.ok(), "stale is a failure, not a warning")

    def test_a_baseline_entry_with_no_matching_file_is_unknown(self):
        report = lfl.evaluate([finding("a/lib.rs", 40)], {"a/renamed_away.rs": 1500})
        self.assertEqual(report.unknown, ["a/renamed_away.rs"])
        self.assertEqual(report.stale, [], "absent is not the same as under the cap")
        self.assertFalse(report.ok())

    def test_stale_and_unknown_are_distinguished(self):
        report = lfl.evaluate(
            [finding("a/fixed.rs", 10)],
            {"a/fixed.rs": 1500, "a/gone.rs": 1500},
        )
        self.assertEqual(report.stale, ["a/fixed.rs"])
        self.assertEqual(report.unknown, ["a/gone.rs"])

    def test_offenders_are_sorted_worst_first(self):
        report = lfl.evaluate([finding("a/small.rs", 1001), finding("a/big.rs", 3000)], {})
        self.assertEqual([f.path for f in report.offenders], ["a/big.rs", "a/small.rs"])


class TestWalker(unittest.TestCase):
    """`scan` -- against a synthesised tree, so every exclusion test can fail."""

    def test_governed_extensions_are_scanned_and_others_are_not(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, "src/a.rs", 5)
            write(root, "frontend/src/b.tsx", 5)
            write(root, "frontend/src/c.css", 5)
            write(root, "src/d.toml", 5)
            write(root, "docs/e.md", 5)
            write(root, "migrations/f.sql", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"src/a.rs", "frontend/src/b.tsx", "frontend/src/c.css"})

    def test_line_counting_follows_splitlines_semantics(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "a.rs").write_text("a\nb\nc", encoding="utf-8")
            (root / "b.rs").write_text("a\nb\nc\n", encoding="utf-8")
            (root / "c.rs").write_text("", encoding="utf-8")
            (root / "d.rs").write_text("\n", encoding="utf-8")
            got = {f.path: f.lines for f in lfl.scan(root)}
        self.assertEqual(got["a.rs"], 3, "no trailing newline still counts the last line")
        self.assertEqual(got["b.rs"], 3, "a trailing newline adds no phantom line")
        self.assertEqual(got["c.rs"], 0, "an empty file is zero lines")
        self.assertEqual(got["d.rs"], 1, "a lone newline is one empty line")

    def test_cargo_target_is_skipped(self):
        """Kills `target` in SKIP_DIRS / `_is_build_target`."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "Cargo.toml").write_text("[package]\n", encoding="utf-8")
            write(root, "target/debug/build/out/generated.rs", 5)
            write(root, "src/keep.rs", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"src/keep.rs"})

    def test_a_source_dir_named_coverage_is_scanned(self):
        """Kills any addition of `coverage` to SKIP_DIRS by basename."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, "src/services/coverage/audit.rs", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertIn("src/services/coverage/audit.rs", got)

    def test_skip_dirs_are_skipped_at_any_depth(self):
        """Kills the SKIP_DIRS membership test in `_walk`."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, "node_modules/pkg/index.js", 5)
            write(root, "frontend/node_modules/deep/index.js", 5)
            write(root, "frontend/dist/assets/index-abc123.js", 5)
            write(root, "frontend/src/keep.js", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"frontend/src/keep.js"})

    def test_worktrees_is_skipped(self):
        """Kills `.worktrees` in SKIP_DIRS (no `.git` marker in this fixture)."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, ".worktrees/feat-x/src/huge.rs", 5)
            write(root, "src/keep.rs", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"src/keep.rs"})

    def test_dot_claude_is_skipped(self):
        """Kills `.claude` in SKIP_DIRS (Claude Code worktrees live there)."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, ".claude/worktrees/feat-x/src/huge.rs", 5)
            write(root, ".claude/tools/helper.ts", 5)
            write(root, "src/keep.rs", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"src/keep.rs"})

    def test_a_nested_checkout_is_skipped_wherever_it_lives(self):
        """Kills `_is_nested_checkout(entry)` in `_walk`.

        The directory name is deliberately NOT in SKIP_DIRS, so only the `.git`
        marker (a FILE, as `git worktree add` writes it) can skip it.
        """
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            wt = root / "wt" / "feat-x"
            write(root, "wt/feat-x/src/huge.rs", 5)
            (wt / ".git").write_text("gitdir: /elsewhere\n", encoding="utf-8")
            write(root, "src/keep.rs", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"src/keep.rs"})

    def test_a_nested_clone_with_a_git_directory_is_also_skipped(self):
        """`exists()`, not `is_file()`: a nested clone's `.git` is a directory."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, "vendor/other-repo/src/huge.rs", 5)
            (root / "vendor" / "other-repo" / ".git").mkdir()
            write(root, "src/keep.rs", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"src/keep.rs"})

    def test_a_directory_that_only_looks_like_a_worktree_is_scanned(self):
        """The nested-checkout skip is anchored on the marker, not the name."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, "tools/worktrees/notes/helper.ts", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertIn("tools/worktrees/notes/helper.ts", got)

    def test_docs_is_governed(self):
        """SKIP_ROOTS is empty: docs/ carries real JS/CSS for the mkdocs site."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, "docs/javascripts/extra.js", 5)
            write(root, "docs/stylesheets/extra.css", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"docs/javascripts/extra.js", "docs/stylesheets/extra.css"})

    def test_a_path_containing_a_space_is_reported_verbatim(self):
        """Kills the `as_posix()` normalisation."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, "frontend/src/Some Dir/styles.css", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"frontend/src/Some Dir/styles.css"})

    def test_symlinks_are_not_followed(self):
        """A linked file must not be counted twice under two paths."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            real = write(root, "src/real.rs", 5)
            link = root / "src" / "link.rs"
            try:
                link.symlink_to(real)
            except (OSError, NotImplementedError):
                self.skipTest("symlinks unavailable on this platform")
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"src/real.rs"})

    def test_extension_match_is_case_insensitive(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, "src/Shouty.RS", 5)
            write(root, "frontend/src/Styles.CSS", 5)
            write(root, "src/normal.rs", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(
            got, {"src/Shouty.RS", "frontend/src/Styles.CSS", "src/normal.rs"}
        )

    def test_a_non_utf8_file_is_skipped_rather_than_crashing(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "bad.rs").write_bytes(b"\xff\xfe\x00binary")
            write(root, "good.rs", 5)
            got = {f.path for f in lfl.scan(root)}
        self.assertEqual(got, {"good.rs"})


class TestRealTree(unittest.TestCase):
    """The checks that must run against the actual checkout."""

    def test_the_walker_finds_the_real_source_tree(self):
        findings = lfl.scan(_ROOT)
        self.assertGreaterEqual(
            len(findings),
            lfl.MIN_SCANNED,
            "the walker found almost nothing -- wrong root, or SKIP_DIRS is too broad",
        )
        paths = [f.path for f in findings]
        self.assertTrue(any(p.startswith("src/") for p in paths), "nothing under src/")
        self.assertTrue(
            any(p.startswith("frontend/src/") for p in paths), "nothing under frontend/src/"
        )
        self.assertTrue(
            all("\\" not in p for p in paths),
            "paths must be Unix-style so committed baseline entries match everywhere",
        )

    def test_no_tracked_source_file_is_dropped_without_an_attributable_reason(self):
        """The over-skip guard, derived from an INDEPENDENT source of truth."""
        if not _git_available():
            self.skipTest("git unavailable")
        unattributed = lfl._cross_check_against_git(_ROOT, lfl.scan(_ROOT))
        self.assertEqual(
            unattributed,
            [],
            "tracked source files were dropped by the walker with no attributable "
            f"reason: {unattributed[:10]}",
        )

    def test_skip_dirs_hide_no_tracked_source(self):
        """Closes the gap the cross-check cannot: a SKIP_DIRS entry that swallows
        tracked source is self-excusing there, so assert here that every
        basename skip hides only untracked trees (build output, deps, worktrees).
        This is what keeps the basename `target` skip honest."""
        if not _git_available():
            self.skipTest("git unavailable")
        tracked = lfl._tracked_files(_ROOT) or []
        hidden = [
            rel
            for rel in tracked
            if lfl._ext_of(rel) in lfl.SOURCE_EXTS
            and any(p in lfl.SKIP_DIRS for p in rel.split("/"))
        ]
        self.assertEqual(hidden, [], f"SKIP_DIRS hides tracked source: {hidden[:10]}")

    def test_the_lint_obeys_its_own_cap(self):
        self.assertIsNone(lfl._self_check(_ROOT))

    def test_every_baseline_entry_names_a_file_that_exists(self):
        """Lives here, NOT in TestMain, whose setUp empties BASELINE."""
        report = lfl.evaluate(lfl.scan(_ROOT), lfl.BASELINE)
        self.assertTrue(lfl.BASELINE, "the committed baseline must not be empty here")
        self.assertEqual(report.unknown, [], "committed baseline has dangling entries")
        self.assertEqual(report.stale, [], "committed baseline has stale entries")


class TestEmitBaseline(unittest.TestCase):
    """`--emit-baseline`, and specifically its raise-laundering warning."""

    def setUp(self):
        self._baseline = lfl.BASELINE
        lfl.BASELINE = {}

    def tearDown(self):
        lfl.BASELINE = self._baseline

    def test_over_cap_files_are_emitted_worst_first(self):
        out = lfl._emit_baseline(
            [
                lfl.Finding("a/small.rs", lfl.MAX_LINES + 1),
                lfl.Finding("a/big.rs", lfl.MAX_LINES + 500),
                lfl.Finding("a/fine.rs", 10),
            ]
        )
        self.assertNotIn("a/fine.rs", out, "under-cap files do not belong in a baseline")
        self.assertLess(out.index("a/big.rs"), out.index("a/small.rs"), "worst first")

    def test_a_grown_file_is_flagged_as_a_raise(self):
        lfl.BASELINE = {"a/huge.rs": lfl.MAX_LINES + 10}
        out = lfl._emit_baseline([lfl.Finding("a/huge.rs", lfl.MAX_LINES + 99)])
        self.assertIn("WARNING", out)
        self.assertIn("a/huge.rs", out)
        self.assertIn(f"{lfl.MAX_LINES + 10} -> {lfl.MAX_LINES + 99}", out)

    def test_a_shrunk_file_is_not_flagged(self):
        lfl.BASELINE = {"a/huge.rs": lfl.MAX_LINES + 99}
        out = lfl._emit_baseline([lfl.Finding("a/huge.rs", lfl.MAX_LINES + 10)])
        self.assertNotIn("WARNING", out, "shrinking is the whole point of the ratchet")

    def test_a_brand_new_offender_is_not_flagged_as_a_raise(self):
        out = lfl._emit_baseline([lfl.Finding("a/new.rs", lfl.MAX_LINES + 1)])
        self.assertNotIn("WARNING", out, "it has no prior number to have raised")


class TestExclusionConstantsAreFrozen(unittest.TestCase):
    """The over-skip guard that `_cross_check_against_git` structurally cannot be.

    Adding a genuinely-correct exclusion means editing this test in the same
    commit, which is the point: it makes the reviewer see it.
    """

    def test_skip_dirs_is_exactly_the_frozen_set(self):
        self.assertEqual(
            set(lfl.SKIP_DIRS),
            {"target", "node_modules", "dist", ".git", ".claude", ".worktrees"},
            "SKIP_DIRS changed. Widening it silently exempts source.",
        )

    def test_skip_roots_is_exactly_the_frozen_set(self):
        self.assertEqual(set(lfl.SKIP_ROOTS), set(), "SKIP_ROOTS changed.")

    def test_generated_is_exactly_the_frozen_set(self):
        self.assertEqual(
            set(lfl.GENERATED),
            set(),
            "GENERATED changed. It means 'no human chose this file's length', "
            "not 'we decided to live with it' -- a hand-written file belongs in "
            "BASELINE instead.",
        )

    def test_coverage_is_not_skipped(self):
        self.assertNotIn("coverage", set(lfl.SKIP_DIRS))


class TestSelfCheck(unittest.TestCase):
    """`_self_check`'s two failure branches, which the real-tree test cannot reach."""

    def _fake_root(self, tmp: Path, lines: int) -> Path:
        (tmp / "scripts").mkdir(parents=True, exist_ok=True)
        (tmp / "scripts" / "lint-file-length.py").write_text(
            "\n".join("x" for _ in range(lines)), encoding="utf-8"
        )
        return tmp

    def test_an_over_cap_lint_file_fails_its_own_check(self):
        with tempfile.TemporaryDirectory() as d:
            root = self._fake_root(Path(d), lfl.MAX_LINES + 1)
            msg = lfl._self_check(root)
        self.assertIsNotNone(msg, "the ratchet must obey the ratchet")
        self.assertIn(str(lfl.MAX_LINES + 1), msg)

    def test_a_lint_file_exactly_at_the_cap_passes(self):
        with tempfile.TemporaryDirectory() as d:
            root = self._fake_root(Path(d), lfl.MAX_LINES)
            self.assertIsNone(lfl._self_check(root))

    def test_an_unreadable_lint_file_fails_closed(self):
        with tempfile.TemporaryDirectory() as d:
            msg = lfl._self_check(Path(d))  # no scripts/lint-file-length.py
        self.assertIsNotNone(msg)
        self.assertIn("refusing", msg.lower())


class TestDuplicateBaselineKeys(unittest.TestCase):
    """`_duplicate_baseline_keys` -- the one check that reads the SOURCE."""

    def _fake_root(self, tmp: Path, body: str) -> Path:
        (tmp / "scripts").mkdir(parents=True, exist_ok=True)
        (tmp / "scripts" / "lint-file-length.py").write_text(body, encoding="utf-8")
        return tmp

    @staticmethod
    def _baseline(*entries: str) -> str:
        return "BASELINE: dict[str, int] = {\n" + "".join(entries) + "}\n"

    def test_a_repeated_path_is_reported(self):
        with tempfile.TemporaryDirectory() as d:
            root = self._fake_root(
                Path(d),
                self._baseline('    "a/b.rs": 1400,\n', '    "a/b.rs": 1200,\n'),
            )
            msg = lfl._duplicate_baseline_keys(root)
        self.assertIsNotNone(msg)
        self.assertIn("a/b.rs", msg)
        self.assertIn("1200", msg, "the message must name the value Python keeps")

    def test_the_benign_ordering_is_reported_too(self):
        with tempfile.TemporaryDirectory() as d:
            root = self._fake_root(
                Path(d),
                self._baseline('    "a/b.rs": 1200,\n', '    "a/b.rs": 1400,\n'),
            )
            self.assertIsNotNone(lfl._duplicate_baseline_keys(root))

    def test_a_baseline_with_no_repeats_passes(self):
        with tempfile.TemporaryDirectory() as d:
            root = self._fake_root(
                Path(d),
                self._baseline('    "a/b.rs": 1400,\n', '    "c/d.rs": 1200,\n'),
            )
            self.assertIsNone(lfl._duplicate_baseline_keys(root))

    def test_entries_outside_the_dict_are_not_scanned(self):
        with tempfile.TemporaryDirectory() as d:
            root = self._fake_root(
                Path(d),
                self._baseline('    "a/b.rs": 1400,\n')
                + 'OTHER = {\n    "a/b.rs": 1,\n    "a/b.rs": 2,\n}\n',
            )
            self.assertIsNone(lfl._duplicate_baseline_keys(root))

    def test_an_unreadable_lint_file_fails_closed(self):
        with tempfile.TemporaryDirectory() as d:
            msg = lfl._duplicate_baseline_keys(Path(d))
        self.assertIsNotNone(msg)
        self.assertIn("refusing", msg.lower())

    def test_the_real_baseline_has_no_duplicates(self):
        self.assertIsNone(lfl._duplicate_baseline_keys(_ROOT))


class TestMain(unittest.TestCase):
    """`main()` is the actual CI entry point."""

    def setUp(self):
        # The module-level BASELINE describes the REAL repo; against a synthetic
        # tree every entry would read as `unknown`. MIN_SCANNED is lowered so the
        # fixtures stay small.
        self._baseline = lfl.BASELINE
        self._min_scanned = lfl.MIN_SCANNED
        lfl.BASELINE = {}
        lfl.MIN_SCANNED = 3

    def tearDown(self):
        lfl.BASELINE = self._baseline
        lfl.MIN_SCANNED = self._min_scanned

    def _tree(self, root: Path, n: int = 5) -> None:
        for i in range(n):
            write(root, f"src/f{i}.rs", 5)
        # `_self_check` fails CLOSED on an unreadable lint file, so a synthetic
        # root needs one. `.py` is not governed, so the scan count is unchanged.
        (root / "scripts").mkdir(parents=True, exist_ok=True)
        (root / "scripts" / "lint-file-length.py").write_text(
            "# stand-in for the real lint\n", encoding="utf-8"
        )

    def _main(self, d: str) -> int:
        with contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(
            io.StringIO()
        ):
            return lfl.main(["--repo", d])

    def test_a_duplicated_baseline_key_fails_through_main(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self._tree(root)
            (root / "scripts" / "lint-file-length.py").write_text(
                "BASELINE: dict[str, int] = {\n"
                '    "src/dupe.rs": 1400,\n'
                '    "src/dupe.rs": 1200,\n'
                "}\n",
                encoding="utf-8",
            )
            with contextlib.redirect_stderr(io.StringIO()) as err:
                rc = lfl.main(["--repo", d])
        self.assertEqual(rc, 1, "a duplicated BASELINE key must fail the gate")
        self.assertIn("src/dupe.rs", err.getvalue())

    def test_a_vacuous_scan_fails(self):
        """The tree carries a valid lint stand-in, so MIN_SCANNED is the only
        branch left that can fail this."""
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root, "src/tiny.rs", 3)  # 1 file, floor is 3
            (root / "scripts").mkdir(parents=True, exist_ok=True)
            (root / "scripts" / "lint-file-length.py").write_text(
                "# stand-in\n", encoding="utf-8"
            )
            rc = self._main(d)
        self.assertEqual(rc, 1, "a scan of 1 file must not be reported as clean")

    def test_a_new_offender_fails(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self._tree(root)
            write(root, "src/huge.rs", lfl.MAX_LINES + 1)
            rc = self._main(d)
        self.assertEqual(rc, 1)

    def test_a_baselined_offender_passes(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self._tree(root)
            write(root, "src/huge.rs", lfl.MAX_LINES + 1)
            lfl.BASELINE = {"src/huge.rs": lfl.MAX_LINES + 1}
            rc = self._main(d)
        self.assertEqual(rc, 0)

    def test_a_baselined_offender_that_grew_fails(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self._tree(root)
            write(root, "src/huge.rs", lfl.MAX_LINES + 50)
            lfl.BASELINE = {"src/huge.rs": lfl.MAX_LINES + 1}
            rc = self._main(d)
        self.assertEqual(rc, 1, "a baseline is a ceiling, not a licence")

    def test_a_clean_tree_passes(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self._tree(root)
            rc = self._main(d)
        self.assertEqual(rc, 0)


if __name__ == "__main__":
    unittest.main()
