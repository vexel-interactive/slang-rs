"""Compare acceptance with standalone Slang and validate positive HLSL with DXC."""

import argparse
import json
import re
import subprocess
from pathlib import Path


def run(command):
    result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60)
    return {"accepted": result.returncode == 0, "returncode": result.returncode, "stdout": result.stdout, "stderr": result.stderr}


def compare_results(case, rust, oracle, validators):
    passed = rust["accepted"] == case["accept"]
    passed = passed and oracle["accepted"] == case.get("oracle_accept", case["accept"])
    if "rust_code" in case:
        passed = passed and case["rust_code"] in rust["stderr"]
    if "rust_warning" in case:
        passed = passed and case["rust_warning"] in rust["stderr"]
    for compiler, result in [("rust", rust), ("oracle", oracle)]:
        passed = passed and all(text in result["stderr"] for text in case.get(f"{compiler}_stderr_contains", []))
    return passed and all(v["accepted"] == v["expected_accept"] for v in validators)


def normalize_bindings(reflection, compiler):
    """Compare direct global resource ranges without claiming uniform byte layout."""
    result = {}
    classes = {"constantBuffer": "b", "shaderResource": "t", "unorderedAccess": "u", "samplerState": "s"}
    for global_info in reflection["globals" if compiler == "rust" else "parameters"]:
        binding = global_info.get("binding")
        if binding is None:
            continue
        if compiler == "rust":
            normalized = {key: binding[key] for key in ["class", "index", "space", "count"]}
        else:
            register_class = classes.get(binding.get("kind"))
            if register_class is None:
                continue
            normalized = {"class": register_class, "index": binding["index"], "space": binding.get("space", 0), "count": binding.get("count", 1)}
        name = global_info["name"]
        if name in result:
            raise ValueError(f"ambiguous resource name in reflection: {name}")
        result[name] = normalized
    return result


def compare_bindings(expected, reflection, compiler):
    actual = normalize_bindings(reflection, compiler)
    return all(actual.get(name) == binding for name, binding in expected.items())


def normalize_uniform_artifact_binding(artifact, compiler):
    """Inspect the named shared cbuffer in selected, unambiguous HLSL fixtures."""
    name = r"_slang_globals" if compiler == "rust" else r"globalParams_\d+"
    matches = re.findall(rf"\bcbuffer\s+{name}\s*:\s*register\(\s*b(\d+)\s*(?:,\s*space(\d+)\s*)?\)", artifact)
    if len(matches) > 1:
        raise ValueError("ambiguous global uniform buffer in HLSL")
    if not matches:
        return None
    index, space = matches[0]
    return {"class": "b", "index": int(index), "space": int(space or 0), "count": 1}


def normalize_global_uniform_binding(reflection, compiler, artifact=None):
    """Compare native reflection with the oracle's emitted shared buffer register."""
    if compiler == "rust":
        binding = reflection.get("global_uniform_binding")
        return None if binding is None else {key: binding[key] for key in ["class", "index", "space", "count"]}
    scope = reflection["globalScope"]
    if scope["kind"] != "constantBuffer":
        return None
    # Slang 2026.21 reports a relative zero here even if the emitted buffer is b1.
    # Read the target artifact rather than treating that zero as an absolute slot.
    if artifact is None:
        raise ValueError("oracle global uniform comparison requires HLSL")
    binding = normalize_uniform_artifact_binding(artifact, compiler)
    if binding is None:
        raise ValueError("oracle global uniform buffer is missing from HLSL")
    return binding


def normalize_uniform_locations(paths, reflection, compiler):
    """Resolve selected fields and first array elements without expanding arrays."""
    globals_info = reflection["globals" if compiler == "rust" else "parameters"]
    result = {}
    for path in paths:
        parts = path.split(".")
        root = parts[0].split("[")[0]
        matches = [g for g in globals_info if g["name"] == root]
        if len(matches) != 1:
            raise ValueError(f"missing or ambiguous uniform global: {root}")
        global_info = matches[0]
        if compiler == "rust":
            location = global_info["uniform_layout"]
            offset = location["offset"]
            ty = reflection["uniform_types"][location["type_name"]]
        else:
            binding = global_info["binding"]
            ty = global_info["type"]
            if binding["kind"] == "constantBuffer":
                while ty["kind"] == "array":
                    ty = ty["elementType"]
                ty = ty["elementType"]
                offset = 0
            else:
                if binding["kind"] != "uniform":
                    raise ValueError(f"global is not uniform data: {root}")
                offset = binding["offset"]
        for index, part in enumerate(parts):
            field_name = part.split("[")[0]
            if index:
                fields = [f for f in ty["fields"] if f["name"] == field_name]
                if len(fields) != 1:
                    raise ValueError(f"missing or ambiguous uniform field: {path}")
                field = fields[0]
                offset += field["offset"] if compiler == "rust" else field["binding"]["offset"]
                ty = reflection["uniform_types"][field["type_name"]] if compiler == "rust" else field["type"]
            suffix = part[len(field_name):]
            if suffix != "[]" * (len(suffix) // 2):
                raise ValueError(f"uniform paths support only first-element [] selectors: {path}")
            for _ in range(len(suffix) // 2):
                ty = reflection["uniform_types"][ty["element_type"]] if compiler == "rust" else ty["elementType"]
        size = ty["size"] if compiler == "rust" else next(s["value"] for s in ty["sizes"] if s["kind"] == "uniform")
        result[path] = {"offset": offset, "size": size}
    return result


def normalize_dxc_uniform_layouts(dump):
    """Read selected legacy DXC buffer definitions with unambiguous field names."""
    result = {}
    for name, body in re.findall(r"^; cbuffer (\S+)\n; \{\n(.*?)^; \}\n", dump, re.MULTILINE | re.DOTALL):
        offsets = {}
        for field, offset in re.findall(r"\b(\w+)(?:\[[^\]]+\])*\s*;\s*; Offset:\s*(\d+)", body):
            offset = int(offset)
            if field in offsets and offsets[field] != offset:
                raise ValueError(f"ambiguous DXC uniform field: {name}.{field}")
            offsets[field] = offset
        sizes = re.findall(r"\bSize:\s*(\d+)", body)
        if len(sizes) != 1 or name in result:
            raise ValueError(f"missing or ambiguous DXC uniform buffer size: {name}")
        result[name] = {"size": int(sizes[0]), "offsets": offsets}
    return result


def compare_uniform_locations(expected, reflection, compiler):
    actual = normalize_uniform_locations(expected, reflection, compiler)
    return all(all(actual[path].get(key) == value for key, value in location.items())
               for path, location in expected.items())


def compare_dxc_uniform_layouts(expected, dump):
    actual = normalize_dxc_uniform_layouts(dump)
    return all(name in actual and actual[name]["size"] == layout["size"]
               and all(actual[name]["offsets"].get(field) == offset for field, offset in layout["offsets"].items())
               for name, layout in expected.items())


def target_buffer_layouts(expected, reflection):
    """Translate source buffer names through reflection without assuming hidden identifiers."""
    result = {}
    for name, layout in expected.items():
        buffers = [g for g in reflection["globals"] if g["name"] == name
                   and g.get("binding", {}).get("class") == "b"]
        if len(buffers) > 1:
            raise ValueError(f"ambiguous constant-buffer name: {name}")
        target = buffers[0]["target_name"] if buffers else name
        if target in result:
            raise ValueError(f"ambiguous target buffer name: {target}")
        result[target] = layout
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--oracle", type=Path, required=True)
    parser.add_argument("--dxc", type=Path)
    parser.add_argument("--manifest", type=Path, default=Path("tests/conformance/manifest.json"))
    parser.add_argument("--output", type=Path, default=Path("out/conformance.json"))
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    results = []
    for index, case in enumerate(manifest["cases"]):
        source = (args.manifest.parent / case["file"]).resolve()
        rust_output = args.output.parent / f"case-{index}-rust.hlsl"
        oracle_output = args.output.parent / f"case-{index}-oracle.hlsl"
        common = [str(source), "-target", "hlsl", "-entry", case["entry"]]
        profile = case.get("compile_profile", case.get("profile"))
        if profile:
            common.extend(["-profile", profile])
        rust_command = [str(args.rust.resolve()), *common, "-o", str(rust_output)]
        oracle_command = [str(args.oracle.resolve()), *common, "-o", str(oracle_output)]
        if any(key in case for key in ["bindings", "global_uniform_binding", "uniform_layouts"]):
            rust_command.extend(["--reflection", str(rust_output.with_suffix(".json"))])
            oracle_command.extend(["-reflection-json", str(oracle_output.with_suffix(".json"))])
        rust = run(rust_command)
        oracle = run(oracle_command)
        validators = []
        if args.dxc:
            for compiler, result, output in [("rust", rust, rust_output), ("oracle", oracle, oracle_output)]:
                if not result["accepted"]:
                    continue
                validation = run([str(args.dxc.resolve()), "-T", case.get("profile", "cs_6_0"), "-E", case["entry"], "-Fo", str(output.with_suffix(".dxil")), str(output)])
                validation["compiler"] = compiler
                validation["expected_accept"] = case.get(f"{compiler}_validation_accept", True)
                validators.append(validation)
        passed = compare_results(case, rust, oracle, validators)
        binding_checks = []
        uniform_binding_checks = []
        layout_checks = []
        if "bindings" in case:
            for compiler, compilation, output in [("rust", rust, rust_output), ("oracle", oracle, oracle_output)]:
                check = {"compiler": compiler, "passed": False, "expected": case["bindings"]}
                if compilation["accepted"]:
                    try:
                        reflection = json.loads(output.with_suffix(".json").read_text(encoding="utf-8"))
                        check["actual"] = normalize_bindings(reflection, compiler)
                        check["passed"] = compare_bindings(case["bindings"], reflection, compiler)
                    except (OSError, ValueError, KeyError, TypeError) as error:
                        check["error"] = str(error)
                binding_checks.append(check)
            passed = passed and all(check["passed"] for check in binding_checks)
        if "global_uniform_binding" in case:
            for compiler, compilation, output in [("rust", rust, rust_output), ("oracle", oracle, oracle_output)]:
                check = {"compiler": compiler, "passed": False, "expected": case["global_uniform_binding"]}
                if compilation["accepted"]:
                    try:
                        reflection = json.loads(output.with_suffix(".json").read_text(encoding="utf-8"))
                        artifact = output.read_text(encoding="utf-8")
                        check["actual"] = normalize_global_uniform_binding(reflection, compiler, artifact)
                        check["artifact"] = normalize_uniform_artifact_binding(artifact, compiler)
                        check["passed"] = check["actual"] == check["expected"] == check["artifact"]
                    except (OSError, ValueError, KeyError, TypeError) as error:
                        check["error"] = str(error)
                uniform_binding_checks.append(check)
            passed = passed and all(check["passed"] for check in uniform_binding_checks)
        if "uniform_layouts" in case:
            for compiler, compilation, output in [("rust", rust, rust_output), ("oracle", oracle, oracle_output)]:
                check = {"compiler": compiler, "passed": False, "expected": case["uniform_layouts"]}
                if compilation["accepted"]:
                    try:
                        reflection = json.loads(output.with_suffix(".json").read_text(encoding="utf-8"))
                        check["actual"] = normalize_uniform_locations(case["uniform_layouts"], reflection, compiler)
                        check["passed"] = compare_uniform_locations(case["uniform_layouts"], reflection, compiler)
                    except (OSError, ValueError, KeyError, TypeError, StopIteration) as error:
                        check["error"] = str(error)
                layout_checks.append(check)
            passed = passed and all(check["passed"] for check in layout_checks)
        results.append({"file": case["file"], "passed": passed, "rust": rust, "oracle": oracle, "validators": validators, "binding_checks": binding_checks, "uniform_binding_checks": uniform_binding_checks, "uniform_layout_checks": layout_checks})
        print(f"{'PASS' if passed else 'FAIL'} {case['file']}")
    report = {
        "kind": "acceptance, selected resource bindings, and HLSL validity, not execution equivalence",
        "passed": sum(r["passed"] for r in results),
        "total": len(results),
        "dxc_valid_artifacts": sum(v["accepted"] and v["expected_accept"] for r in results for v in r["validators"]),
        "dxc_invalid_artifacts_rejected": sum(not v["accepted"] and not v["expected_accept"] for r in results for v in r["validators"]),
        "resource_binding_cases": sum(bool(r["binding_checks"]) for r in results),
        "global_uniform_binding_cases": sum(bool(r["uniform_binding_checks"]) for r in results),
        "uniform_layout_cases": sum(bool(r["uniform_layout_checks"]) for r in results),
        "cases": results,
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"{report['passed']}/{report['total']} passed")
    raise SystemExit(0 if report["passed"] == report["total"] else 1)


if __name__ == "__main__":
    main()
