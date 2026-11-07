"""Keep documented oracle differences separate from unexpected regressions."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from conformance import compare_bindings, compare_results, normalize_bindings


def result(accepted, stderr=""):
    return {"accepted": accepted, "stderr": stderr}


class ConformanceComparison(unittest.TestCase):
    def test_resource_bindings_normalize_oracle_defaults_and_ignore_uniform_offsets(self):
        oracle = {"parameters": [
            {"name": "image", "binding": {"kind": "shaderResource", "index": 2}},
            {"name": "sampler", "binding": {"kind": "samplerState", "index": 1}},
            {"name": "value", "binding": {"kind": "uniform", "offset": 0, "size": 4}},
        ]}
        self.assertEqual(normalize_bindings(oracle, "oracle"), {
            "image": {"class": "t", "index": 2, "space": 0, "count": 1},
            "sampler": {"class": "s", "index": 1, "space": 0, "count": 1},
        })

    def test_resource_range_comparison_requires_each_binding_field(self):
        expected = {"images": {"class": "t", "index": 2, "space": 3, "count": 6}}
        for compiler, reflection in [
            ("rust", {"globals": [{"name": "images", "binding": {**expected["images"], "explicit": False}}]}),
            ("oracle", {"parameters": [{"name": "images", "binding": {"kind": "shaderResource", "index": 2, "space": 3, "count": 6}}]}),
        ]:
            self.assertTrue(compare_bindings(expected, reflection, compiler))
            for key, value in [("class", "u"), ("index", 1), ("space", 0), ("count", 1)]:
                self.assertFalse(compare_bindings({"images": {**expected["images"], key: value}}, reflection, compiler))
            self.assertFalse(compare_bindings({"missing": expected["images"]}, reflection, compiler))

    def test_missing_binding_data_cannot_count_as_a_match(self):
        expected = {"image": {"class": "t", "index": 0, "space": 0, "count": 1}}
        self.assertFalse(compare_bindings(expected, {"globals": [{"name": "image"}]}, "rust"))
        with self.assertRaises(KeyError):
            normalize_bindings({"globals": [{"name": "image", "binding": {"class": "t"}}]}, "rust")

    def test_duplicate_resource_names_are_ambiguous(self):
        binding = {"class": "t", "index": 0, "space": 0, "count": 1}
        with self.assertRaises(ValueError):
            normalize_bindings({"globals": [{"name": "image", "binding": binding}] * 2}, "rust")

    def test_accepted_sources_can_require_an_invalid_oracle_artifact(self):
        case = {"accept": True, "oracle_validation_accept": False}
        native = {"accepted": True, "expected_accept": True}
        oracle = {"accepted": False, "expected_accept": False}
        self.assertTrue(compare_results(case, result(True), result(True), [native, oracle]))
        self.assertFalse(compare_results(case, result(True), result(True), [native, {"accepted": True, "expected_accept": False}]))
        self.assertFalse(compare_results(case, result(True), result(True), [{"accepted": False, "expected_accept": True}, oracle]))

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
