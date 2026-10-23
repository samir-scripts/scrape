"""Deterministic tests for the offline O*NET role-title seed normalizer."""

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("prepare-role-title-seed.py")
SPEC = importlib.util.spec_from_file_location("role_title_seed", SCRIPT)
SEED = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SEED)


class RoleTitleSeedTests(unittest.TestCase):
    def build(self, occupations, reported, alternates):
        return SEED.build_seed(occupations.encode(), reported.encode(), alternates.encode()).decode()

    def test_trims_deduplicates_by_casefold_and_keeps_highest_priority(self):
        result = self.build(
            "O*NET-SOC Code,Title,Description\n15-1252.00, Engineer ,fixture\n15-1253.00,Data Engineer,fixture\n",
            "O*NET-SOC Code,Title,Reported Job Title,Shown in My Next Move\n15-1253.00,Data Engineer, engineer ,N\n15-1252.00,Engineer,Platform Engineer,Y\n",
            "O*NET-SOC Code,Title,Job Title,Short Title,Source(s)\n15-1252.00,Software Developers,Engineer,,08\n15-1253.00,Data Engineer,Intern,,08\n",
        )
        self.assertEqual(result.splitlines(), [
            "soc_code\ttitle\tpriority_tier",
            "15-1253\tData Engineer\t1",
            "15-1252\tEngineer\t1",
            "15-1252\tPlatform Engineer\t2",
            "15-1253\tIntern\t3",
        ])

    def test_filters_all_other_titles_and_sorts_tiers_before_titles(self):
        result = self.build(
            "O*NET-SOC Code,Title,Description\n15-1252.00,Software Developers,fixture\n15-1253.00,All Other Developers,fixture\n",
            "O*NET-SOC Code,Title,Reported Job Title,Shown in My Next Move\n15-1252.00,Software Developers,Zeta Role,N\n",
            "O*NET-SOC Code,Title,Job Title,Short Title,Source(s)\n15-1252.00,Software Developers,Alpha Role,,08\n",
        )
        self.assertEqual(result.splitlines()[1:], [
            "15-1252\tSoftware Developers\t1",
            "15-1252\tZeta Role\t2",
            "15-1252\tAlpha Role\t3",
        ])
        self.assertNotIn("All Other", result)

    def test_rejects_invalid_soc_code_instead_of_truncating(self):
        with self.assertRaisesRegex(ValueError, "Invalid O\\*NET-SOC code"):
            self.build(
                "O*NET-SOC Code,Title,Description\n15-1252,Software Developers,fixture\n",
                "O*NET-SOC Code,Title,Reported Job Title,Shown in My Next Move\n",
                "O*NET-SOC Code,Title,Job Title,Short Title,Source(s)\n",
            )

    def test_casefold_sort_and_same_tier_soc_tie_break_are_deterministic(self):
        occupations = "O*NET-SOC Code,Title\n15-1253.00,zeta\n15-1252.00,alpha\n15-1253.00,STRASSE\n15-1252.00,Straße\n"
        result = self.build(occupations, "O*NET-SOC Code,Reported Job Title\n", "O*NET-SOC Code,Job Title\n")
        self.assertEqual(result.splitlines()[1:], [
            "15-1252\talpha\t1", "15-1252\tStraße\t1", "15-1253\tzeta\t1",
        ])
        self.assertEqual(result, self.build(occupations, "O*NET-SOC Code,Reported Job Title\n", "O*NET-SOC Code,Job Title\n"))

    def test_rejects_missing_headers_empty_titles_and_malformed_rows(self):
        for invalid in (
            "Wrong,Title\n15-1252.00,Engineer\n",
            "O*NET-SOC Code,Title\n15-1252.00,\n",
            "O*NET-SOC Code,Title\n15-1252.00,Engineer,extra\n",
            "O*NET-SOC Code,Title\n15-1252.00\n",
        ):
            with self.subTest(source=invalid), self.assertRaises(ValueError):
                self.build(invalid, "O*NET-SOC Code,Reported Job Title\n", "O*NET-SOC Code,Job Title\n")


if __name__ == "__main__":
    unittest.main()
