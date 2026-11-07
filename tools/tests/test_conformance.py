"""Keep documented oracle differences separate from unexpected regressions."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from conformance import compare_results


def result(accepted, stderr=""):
    return {"accepted": accepted, "stderr": stderr}


class ConformanceComparison(unittest.TestCase):
    def test_declared_diagnostic_fragments_are_required_for_each_compiler(self):
        case = {
            "accept": False,
            "rust_stderr_contains": ["virtual.slang:70:", "70 | return missing;"],
            "oracle_stderr_contains": ["virtual.slang(70)", "undefined identifier"],
        }
        rust = result(False, "virtual.slang:70:1\n70 | return missing;")
        oracle = result(False, "virtual.slang(70): undefined identifier")
        self.assertTrue(compare_results(case, rust, oracle, []))
        self.assertFalse(compare_results(case, result(False, "virtual.slang:70:1"), oracle, []))
        self.assertFalse(compare_results(case, rust, result(False, "virtual.slang(71): undefined identifier"), []))

    def test_profile_upgrade_warnings_are_required_when_declared(self):
        case = {"accept": True, "rust_warning": "C003"}
        self.assertTrue(compare_results(case, result(True, "warning[C003]"), result(True), []))
        self.assertFalse(compare_results(case, result(True), result(True), []))

    def test_ordinary_cases_require_both_compilers_to_match(self):
        for expected in [False, True]:
            for rust in [False, True]:
                for oracle in [False, True]:
                    with self.subTest(expected=expected, rust=rust, oracle=oracle):
                        self.assertEqual(
                            compare_results({"accept": expected}, result(rust), result(oracle), []),
                            rust == expected and oracle == expected,
                        )

    def test_explicit_oracle_difference_requires_expected_validation_failure(self):
        case = {"accept": False, "oracle_accept": True, "rust_code": "T023"}
        oracle_validation = {"accepted": False, "expected_accept": False}
        self.assertTrue(compare_results(case, result(False, "T023"), result(True), [oracle_validation]))
        self.assertFalse(compare_results(case, result(True), result(True), [oracle_validation]))
        self.assertFalse(compare_results(case, result(False, "T023"), result(False), []))
        self.assertFalse(compare_results(case, result(False, "T023"), result(True), [{"accepted": True, "expected_accept": False}]))

    def test_positive_validation_and_native_diagnostic_codes_are_required(self):
        self.assertFalse(compare_results({"accept": True}, result(True), result(True), [{"accepted": False, "expected_accept": True}]))
        self.assertFalse(compare_results({"accept": False, "rust_code": "T023"}, result(False, "T014"), result(False), []))


if __name__ == "__main__":
    unittest.main()
