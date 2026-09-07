"""Acceptance wrappers for package cache lifecycle behavior."""

from __future__ import annotations

import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=ROOT,
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
        check=False,
        timeout=180,
    )


class PackageCacheLifecycleAcceptance(unittest.TestCase):
    def assert_cargo_case(self, *args: str) -> None:
        result = run("cargo", "test", "--locked", *args, "--", "--nocapture")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_final_output_is_never_an_automatic_cleanup_target(self) -> None:
        self.assert_cargo_case("package_inventory::tests::final_output_is_never_an_automatic_cleanup_target")

    def test_clean_inventory_reports_categories_and_protects_final_outputs(self) -> None:
        self.assert_cargo_case("--test", "package_lifecycle", "clean_inventory_reports_categories_and_protects_final_outputs")

    def test_cache_key_ignores_profile_revision_name_output_and_reason(self) -> None:
        self.assert_cargo_case("package_cache::tests::cache_key_ignores_profile_revision_name_output_and_reason")

    def test_global_preflight_uses_historical_cache_size_and_exposes_cap(self) -> None:
        self.assert_cargo_case("package_storage::tests::global_preflight_uses_historical_cache_size_and_exposes_cap")

    def test_scoped_stale_cleanup_requires_yes_and_updates_each_record(self) -> None:
        self.assert_cargo_case("--test", "package_lifecycle", "scoped_stale_cleanup_requires_yes_and_updates_each_record")

    def test_successful_plugin_delivery_removes_private_stage(self) -> None:
        self.assert_cargo_case("successful_plugin_delivery_removes_private_stage")

    def test_plugin_stage_uses_read_only_junctions_and_private_generated_dirs(self) -> None:
        self.assert_cargo_case("plugin_stage_uses_read_only_junctions_and_private_generated_dirs")
