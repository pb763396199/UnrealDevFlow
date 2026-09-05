"""Acceptance wrapper for the native-run construction plan.

The wrapper intentionally fails when a case has no evidence instead of using
unittest.skip: a green wrapper with zero matched native tests is not an
acceptance result.
"""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = ROOT / "target" / "run-evidence"


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=ROOT,
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
        check=False,
    )


class NativeRunAcceptance(unittest.TestCase):
    def test_profile_rules_are_exercised(self) -> None:
        result = run("cargo", "test", "--locked", "--bin", "udf", "run_profile::tests")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("19 passed", result.stdout)

    def test_cli_read_only_cases_are_exercised(self) -> None:
        result = run(
            "cargo",
            "test",
            "--locked",
            "--test",
            "run_profile",
            "--test",
            "run_commands",
            "--test",
            "run_existing_editor",
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("test result: ok", result.stdout)

    def test_lifecycle_cases_preserve_failure_data(self) -> None:
        result = run("cargo", "test", "--locked", "--test", "run_lifecycle")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("3 passed", result.stdout)

    def test_gauntlet_rules_have_real_native_evidence(self) -> None:
        logs = sorted(EVIDENCE.glob("p1-gauntlet-rules-*/runuat.stdout.log"))
        self.assertTrue(logs, "缺少 UAT TestGauntlet 证据")
        text = logs[-1].read_text(encoding="utf-8", errors="replace")
        self.assertIn("Test Udf.EditorExitRulesTests Passed", text)
        self.assertIn("Udf.EditorExitRulesTests result=Passed", text)

    def test_shell_suite_has_three_real_shell_records(self) -> None:
        summaries = sorted(EVIDENCE.glob("shells-game-*/summary.json"))
        if not summaries:
            summaries = sorted(EVIDENCE.glob("shells-*/summary.json"))
        self.assertTrue(summaries, "缺少 Shells 套件证据")
        summary = json.loads(summaries[-1].read_text(encoding="utf-8"))
        self.assertEqual(summary["shells"], [
            "configure-perflab-game",
            "shell-powershell-plan",
            "shell-cmd-plan",
            "shell-git-bash-plan",
            "shell-powershell-start",
            "shell-cmd-start",
            "shell-git-bash-start",
        ])
        for label in ("shell-powershell-plan", "shell-cmd-plan", "shell-git-bash-plan"):
            path = summaries[-1].parent / f"{label}.stdout.json"
            self.assertTrue(path.is_file(), path)
            payload = json.loads(path.read_text(encoding="utf-8"))
            argv = payload["data"]["nativeArgv"]
            self.assertIn("/Game/Maps/UGA_local/aes6_sh_sz_q1", argv)
            self.assertFalse(any("Git/Game" in value for value in argv))
        for label in ("shell-powershell-start", "shell-cmd-start", "shell-git-bash-start"):
            marker = summaries[-1].parent / f"{label}.map-loaded.txt"
            self.assertTrue(marker.is_file(), marker)
            self.assertIn("Map=/Game/Maps/UGA_local/aes6_sh_sz_q1", marker.read_text(encoding="utf-8"))

    def test_host_execution_has_strict_exit_result(self) -> None:
        records = sorted(EVIDENCE.glob("cli-config-*/executions/run/*/record.json"))
        self.assertTrue(records, "缺少 UDF start 的 execution 证据")
        payloads = [json.loads(path.read_text(encoding="utf-8")) for path in records]
        strict = [item for item in payloads if item.get("name") == "editor-exit"]
        self.assertTrue(strict, "缺少 editor-exit execution")
        self.assertTrue(any(item.get("state") == "passed" for item in strict))
        passed = next(item for item in strict if item.get("state") == "passed")
        self.assertEqual(passed.get("toolExitCode"), 0)
        self.assertEqual(passed.get("ueExitCode"), 0)
        self.assertEqual(passed.get("exitResult"), "passed")
        self.assertTrue(any(artifact["kind"] == "nativeReport" for artifact in passed["artifacts"]))


if __name__ == "__main__":
    unittest.main()
