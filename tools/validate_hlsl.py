"""Validate native HLSL fixtures with the entry and profile from reflection."""

import argparse
import json
from pathlib import Path

from conformance import compare_dxc_uniform_layouts, compare_uniform_locations, dxc_type_arguments, normalize_dxc_constant_stores, run, target_buffer_layouts
from conformance import compare_dxc_resource_strides, target_resource_names


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--dxc", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=Path("tests/native/manifest.json"))
    parser.add_argument("--output", type=Path, default=Path("out/native-hlsl.json"))
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    results = []
    for index, case in enumerate(manifest["cases"]):
        source = (args.manifest.parent / case["file"]).resolve()
        output = args.output.parent / f"native-{index}.hlsl"
        reflection_path = output.with_suffix(".json")
        command = [
            str(args.rust.resolve()), str(source), "-target", "hlsl",
            "-entry", case["entry"], "-o", str(output),
            "--reflection", str(reflection_path),
        ]
        if profile := case.get("compile_profile"):
            command.extend(["-profile", profile])
        if case.get("native_16bit_types"):
            command.append("-enable-16bit-types")
        if major := case.get("matrix_major"):
            if major not in {"row", "column"}:
                raise ValueError(f"invalid matrix_major: {major}")
            command.append(f"-matrix-layout-{major}-major")
        compilation = run(command)
        validation = None
        entry = None
        layout_checks = {}
        constant_stores = None
        if compilation["accepted"]:
            reflection = json.loads(reflection_path.read_text(encoding="utf-8"))
            entry = next(e for e in reflection["entry_points"] if e["name"] == case["entry"])
            validation = run([
                str(args.dxc.resolve()), "-T", entry["target_profile"],
                *dxc_type_arguments(entry["target_profile"], reflection),
                "-E", entry["target_name"], "-Fo", str(output.with_suffix(".dxil")),
                str(output),
            ])
            if "uniform_layouts" in case:
                layout_checks["reflection"] = compare_uniform_locations(case["uniform_layouts"], reflection, "rust")
            if "dxc_resource_strides" in case:
                layout_checks["strides"] = False
                if validation["accepted"]:
                    dump = run([str(args.dxc.resolve()), "-dumpbin", str(output.with_suffix(".dxil"))])
                    expected = target_resource_names(case["dxc_resource_strides"], reflection, {"t", "u"})
                    layout_checks["strides"] = dump["accepted"] and compare_dxc_resource_strides(expected, dump["stdout"])
            if "dxc_layouts" in case and validation["accepted"]:
                dump = run([str(args.dxc.resolve()), "-dumpbin", str(output.with_suffix(".dxil"))])
                expected = target_buffer_layouts(case["dxc_layouts"], reflection)
                layout_checks["dxc"] = dump["accepted"] and compare_dxc_uniform_layouts(expected, dump["stdout"])
            if "dxc_constant_stores" in case:
                validation["constant_stores_passed"] = False
                if validation["accepted"]:
                    dump = run([str(args.dxc.resolve()), "-dumpbin", str(output.with_suffix(".dxil"))])
                    try:
                        constant_stores = normalize_dxc_constant_stores(dump["stdout"])
                        validation["constant_stores_passed"] = dump["accepted"] and constant_stores == case["dxc_constant_stores"]
                    except ValueError as error:
                        validation["constant_stores_error"] = str(error)
        passed = compilation["accepted"] and validation is not None and validation["accepted"]
        passed = passed and all(layout_checks.values())
        passed = passed and (validation is not None and validation.get("constant_stores_passed", True))
        results.append({
            "file": case["file"], "passed": passed, "entry": entry,
            "compilation": compilation, "validation": validation,
            "uniform_layout_checks": layout_checks,
            "constant_stores_actual": constant_stores,
        })
        print(f"{'PASS' if passed else 'FAIL'} {case['file']}")
    report = {
        "kind": "native HLSL validity, not upstream parity or GPU execution",
        "passed": sum(r["passed"] for r in results),
        "total": len(results),
        "uniform_layout_cases": sum(bool(r["uniform_layout_checks"]) for r in results),
        "constant_store_cases": sum(r["constant_stores_actual"] is not None for r in results),
        "cases": results,
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"{report['passed']}/{report['total']} passed")
    raise SystemExit(0 if report["passed"] == report["total"] else 1)


if __name__ == "__main__":
    main()
