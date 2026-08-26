import subprocess
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]


def run(command: list[str]) -> None:
    completed = subprocess.run(
        command,
        cwd=REPO,
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
    )
    if completed.returncode != 0:
        raise AssertionError(completed.stdout + completed.stderr)


def cargo_test(test_name: str) -> None:
    run(["cargo", "test", "--test", "merge_yes", test_name, "--", "--exact"])


class RebaseSilentDataLossAcceptance(unittest.TestCase):
    def test_rebase_refuses_dirty_target_before_upstream_fast_forward(self) -> None:
        cargo_test("rebase_refuses_dirty_target_before_upstream_fast_forward")

    def test_rebase_merge_preserves_existing_target_commits(self) -> None:
        cargo_test("rebase_merge_preserves_existing_target_commits")

    def test_an_untracked_file_does_not_block_squash_or_rebase(self) -> None:
        cargo_test("an_untracked_file_does_not_block_squash_or_rebase")


if __name__ == "__main__":
    unittest.main()
