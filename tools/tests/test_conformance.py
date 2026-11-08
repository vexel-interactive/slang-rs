"""Keep documented oracle differences separate from unexpected regressions."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from conformance import compare_bindings, compare_results, normalize_bindings, normalize_global_uniform_binding, normalize_uniform_artifact_binding, normalize_uniform_locations, compare_uniform_locations, compare_dxc_uniform_layouts, normalize_dxc_uniform_layouts
from conformance import target_buffer_layouts
from conformance import normalize_dxc_constant_stores
from conformance import expected_constant_stores
from conformance import expected_uniform_locations


def result(accepted, stderr=""):
    return {"accepted": accepted, "stderr": stderr}


class ConformanceComparison(unittest.TestCase):
    def test_documented_constant_differences_require_both_exact_goldens(self):
        case = {"dxc_constant_stores": {"0": 1}, "oracle_dxc_constant_stores": {"0": 0}, "reason": "Pinned SCCP compares integer bits as signed for boolean results."}
        self.assertEqual(expected_constant_stores(case, "rust"), {"0": 1})
        self.assertEqual(expected_constant_stores(case, "oracle"), {"0": 0})
        self.assertEqual(expected_constant_stores({"dxc_constant_stores": {"0": 1}}, "oracle"), {"0": 1})
        for invalid in [{"oracle_dxc_constant_stores": {"0": 0}}, {"dxc_constant_stores": {"0": 1}, "oracle_dxc_constant_stores": {"0": 0}}]:
            with self.assertRaises(ValueError):
                expected_constant_stores(invalid, "oracle")

    def test_constant_stores_require_exact_values_and_valid_hlsl(self):
        dump = "call void @dx.op.bufferStore.i32(i32 69, %dx.types.Handle %1, i32 2, i32 0, i32 -1, i32 undef, i32 undef, i32 undef, i8 1)"
        self.assertEqual(normalize_dxc_constant_stores(dump), {"2": 0xffffffff})
        self.assertEqual(normalize_dxc_constant_stores(""), {})
        validator = {"accepted": True, "expected_accept": True, "constant_stores_passed": False}
        self.assertFalse(compare_results({"accept": True}, result(True), result(True), [validator]))
        self.assertTrue(compare_results({"accept": True}, result(True), result(True), [{**validator, "constant_stores_passed": True}]))

    def test_dynamic_repeated_vector_and_multi_resource_stores_are_rejected(self):
        store = "call void @dx.op.bufferStore.i32(i32 69, %dx.types.Handle %1, i32 2, i32 0, i32 42, i32 undef, i32 undef, i32 undef, i8 1)"
        for dump in [store.replace("i32 42", "i32 %value"), store.replace("i8 1", "i8 3"),
                     store + "\n" + store,
                     store + "\n" + store.replace("%1", "%2").replace("i32 2,", "i32 3,")]:
            with self.assertRaises(ValueError):
                normalize_dxc_constant_stores(dump)

    def test_constant_double_splitting_preserves_bits_and_negative_zero(self):
        for number, bits in [("0x3FF0000000000001", 0x3ff0000000000001), ("-0.000000e+00", 0x8000000000000000), ("4.940656e-324", 1)]:
            dump = f"%2 = call %dx.types.splitdouble @dx.op.splitDouble.f64(i32 102, double {number})\n"
            dump += "%3 = extractvalue %dx.types.splitdouble %2, 0\n%4 = extractvalue %dx.types.splitdouble %2, 1\n"
            for index, value in enumerate(['%3', '%4']):
                dump += f"call void @dx.op.bufferStore.i32(i32 69, %dx.types.Handle %1, i32 {index}, i32 0, i32 {value}, i32 undef, i32 undef, i32 undef, i8 1)\n"
            self.assertEqual(normalize_dxc_constant_stores(dump), {"0": bits & 0xffffffff, "1": bits >> 32})

    def test_valid_hlsl_with_wrong_physical_layout_cannot_pass(self):
        validator = {"accepted": True, "expected_accept": True, "layout_passed": False}
        self.assertFalse(compare_results({"accept": True}, result(True), result(True), [validator]))
        self.assertTrue(compare_results({"accept": True}, result(True), result(True), [{**validator,"layout_passed":True}]))

    def test_buffer_layouts_use_reflected_targets_and_reject_ambiguous_names(self):
        layout = {"size": 4, "offsets": {"value": 0}}
        reflection = {"globals": [
            {"name": "Params", "target_name": "hidden", "binding": {"class": "b"}},
            {"name": "Params", "target_name": "Params"},
        ]}
        self.assertEqual(target_buffer_layouts({"Params": layout, "_slang_globals": layout}, reflection),
                         {"hidden": layout, "_slang_globals": layout})
        with self.assertRaises(ValueError):
            target_buffer_layouts({"Params": layout}, {"globals": [reflection["globals"][0]] * 2})
        with self.assertRaises(ValueError):
            target_buffer_layouts({"Params": layout, "hidden": layout}, reflection)

    def test_uniform_locations_follow_shared_types_fields_and_first_array_elements(self):
        native = {"globals": [{"name": "data", "uniform_layout": {"offset": 16, "type_name": "P"}}], "uniform_types": {
            "P": {"size": 40, "fields": [{"name": "values", "offset": 16, "type_name": "float2[2]"}]},
            "float2[2]": {"size": 24, "element_type": "float2"}, "float2": {"size": 8},
        }}
        oracle = {"parameters": [{"name": "data", "binding": {"kind": "uniform", "offset": 16}, "type": {
            "sizes": [{"kind": "uniform", "value": 40}], "fields": [{"name": "values", "binding": {"offset": 16}, "type": {
                "sizes": [{"kind": "uniform", "value": 24}], "elementType": {"sizes": [{"kind": "uniform", "value": 8}]},
            }}],
        }}]}
        expected = {"data": {"offset": 16, "size": 40}, "data.values": {"offset": 32, "size": 24}, "data.values[]": {"offset": 32, "size": 8}}
        for compiler, reflection in [("rust", native), ("oracle", oracle)]:
            self.assertEqual(normalize_uniform_locations(expected, reflection, compiler), expected)
            self.assertTrue(compare_uniform_locations({"data.values": {"offset": 32}}, reflection, compiler))
            self.assertFalse(compare_uniform_locations({"data.values": {"offset": 16}}, reflection, compiler))
            self.assertFalse(compare_uniform_locations({"data.values": {"size": 32}}, reflection, compiler))
            with self.assertRaises(ValueError):
                normalize_uniform_locations(["data.values[1]"], reflection, compiler)

    def test_missing_uniform_layouts_cannot_count_as_matching(self):
        with self.assertRaises(KeyError):
            normalize_uniform_locations(["value"], {"globals": [{"name": "value"}]}, "rust")
        with self.assertRaises(ValueError):
            normalize_uniform_locations(["value"], {"globals": []}, "rust")

    def test_documented_reflection_differences_require_both_complete_goldens(self):
        native = {"value": {"offset": 0, "size": 4}}
        oracle = {"value": {"offset": 0, "size": 2}}
        case = {"uniform_layouts": native, "oracle_uniform_layouts": oracle, "reason": "Half physical storage differs from source reflection."}
        self.assertEqual(expected_uniform_locations(case, "rust"), native)
        self.assertEqual(expected_uniform_locations(case, "oracle"), oracle)
        reflection = {"parameters": [{"name": "value", "binding": {"kind": "uniform", "offset": 0}, "type": {"sizes": [{"kind": "uniform", "value": 2}]}}]}
        self.assertTrue(compare_uniform_locations(expected_uniform_locations(case, "oracle"), reflection, "oracle"))
        self.assertFalse(compare_uniform_locations(native, reflection, "oracle"))
        for invalid in [
            {"oracle_uniform_layouts": oracle, "reason": "Missing native."},
            {"uniform_layouts": native, "oracle_uniform_layouts": oracle},
            {**case, "oracle_uniform_layouts": {}},
            {**case, "oracle_uniform_layouts": {"value": {"offset": 0}}},
        ]:
            with self.assertRaises(ValueError):
                expected_uniform_locations(invalid, "oracle")

    def test_dxc_uniform_buffer_sizes_and_offsets_must_match(self):
        dump = "; cbuffer parameters\n; {\n; float value; ; Offset: 4\n; } parameters; ; Offset: 0 Size: 8\n; }\n"
        expected = {"parameters": {"size": 8, "offsets": {"value": 4}}}
        self.assertTrue(compare_dxc_uniform_layouts(expected, dump))
        self.assertFalse(compare_dxc_uniform_layouts({"parameters": {"size": 16, "offsets": {"value": 4}}}, dump))
        self.assertFalse(compare_dxc_uniform_layouts({"parameters": {"size": 8, "offsets": {"value": 0}}}, dump))
        self.assertFalse(compare_dxc_uniform_layouts(expected, ""))
        with self.assertRaises(ValueError):
            normalize_dxc_uniform_layouts(dump.replace("; } parameters", "; float value; ; Offset: 0\n; } parameters"))

    def test_global_uniform_buffer_bindings_normalize_defaults_and_origin(self):
        expected = {"class": "b", "index": 1, "space": 0, "count": 1}
        self.assertEqual(normalize_global_uniform_binding({"global_uniform_binding": {**expected, "explicit": False}}, "rust"), expected)
        scope = {"globalScope": {"kind": "constantBuffer", "binding": {"kind": "constantBuffer", "index": 0}}}
        self.assertEqual(normalize_global_uniform_binding(scope, "oracle", "cbuffer globalParams_0 : register(b1) {}"), expected)
        expected = {**expected, "space": 3}
        self.assertEqual(normalize_global_uniform_binding(scope, "oracle", "cbuffer globalParams_0 : register(b1, space3) {}"), expected)

    def test_absent_and_malformed_global_uniform_buffer_bindings_are_distinct(self):
        self.assertIsNone(normalize_global_uniform_binding({}, "rust"))
        self.assertIsNone(normalize_global_uniform_binding({"globalScope": {"kind": "struct"}}, "oracle"))
        for compiler, reflection in [
            ("rust", {"global_uniform_binding": {"class": "b"}}),
            ("oracle", {}),
        ]:
            with self.assertRaises(KeyError):
                normalize_global_uniform_binding(reflection, compiler)
        with self.assertRaises(ValueError):
            normalize_global_uniform_binding({"globalScope": {"kind": "constantBuffer"}}, "oracle")
        with self.assertRaises(ValueError):
            normalize_global_uniform_binding({"globalScope": {"kind": "constantBuffer"}}, "oracle", "")

    def test_shared_uniform_hlsl_comparison_requires_unambiguous_named_buffers(self):
        expected = {"class": "b", "index": 2, "space": 3, "count": 1}
        self.assertEqual(normalize_uniform_artifact_binding("cbuffer _slang_globals : register(b2, space3) {}", "rust"), expected)
        self.assertIsNone(normalize_uniform_artifact_binding("cbuffer other : register(b0) {}", "oracle"))
        with self.assertRaises(ValueError):
            normalize_uniform_artifact_binding("cbuffer globalParams_0 : register(b0) {} cbuffer globalParams_1 : register(b1) {}", "oracle")

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
