import subprocess
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[3]


def cargo_test(test_name: str, test_target: str) -> None:
    completed = subprocess.run(
        ["cargo", "test", "--test", test_target, test_name, "--", "--exact"],
        cwd=REPO,
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
    )
    if completed.returncode != 0:
        raise AssertionError(completed.stdout + completed.stderr)


class QuerySemanticsAcceptance(unittest.TestCase):
    def test_build_and_package_check_share_readiness_contract(self) -> None:
        cargo_test("build_and_package_check_share_readiness_contract", "query_semantics")

    def test_plans_do_not_replace_latest_execution(self) -> None:
        cargo_test("plans_do_not_replace_latest_execution", "package_lifecycle")

    def test_status_reads_only_real_executions(self) -> None:
        cargo_test("status_reads_only_real_executions", "package_lifecycle")

    def test_non_execution_queries_follow_the_shared_vocabulary(self) -> None:
        cargo_test("non_execution_queries_follow_the_shared_vocabulary", "cli_taxonomy")

    def test_legacy_query_invocations_have_explicit_compatibility(self) -> None:
        cargo_test("legacy_query_invocations_have_explicit_compatibility", "cli_taxonomy")


if __name__ == "__main__":
    unittest.main()
