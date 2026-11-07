"""Compare acceptance with standalone Slang and validate positive HLSL with DXC."""

import argparse
import json
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
        if "bindings" in case:
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
        results.append({"file": case["file"], "passed": passed, "rust": rust, "oracle": oracle, "validators": validators, "binding_checks": binding_checks})
        print(f"{'PASS' if passed else 'FAIL'} {case['file']}")
    report = {
        "kind": "acceptance, selected resource bindings, and HLSL validity, not execution equivalence",
        "passed": sum(r["passed"] for r in results),
        "total": len(results),
        "dxc_valid_artifacts": sum(v["accepted"] and v["expected_accept"] for r in results for v in r["validators"]),
        "dxc_invalid_artifacts_rejected": sum(not v["accepted"] and not v["expected_accept"] for r in results for v in r["validators"]),
        "resource_binding_cases": sum(bool(r["binding_checks"]) for r in results),
        "cases": results,
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"{report['passed']}/{report['total']} passed")
    raise SystemExit(0 if report["passed"] == report["total"] else 1)


if __name__ == "__main__":
    main()
