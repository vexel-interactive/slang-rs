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
    return passed and all(v["accepted"] == v["expected_accept"] for v in validators)


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
        rust = run([str(args.rust.resolve()), *common, "-o", str(rust_output)])
        oracle = run([str(args.oracle.resolve()), *common, "-o", str(oracle_output)])
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
        results.append({"file": case["file"], "passed": passed, "rust": rust, "oracle": oracle, "validators": validators})
        print(f"{'PASS' if passed else 'FAIL'} {case['file']}")
    report = {
        "kind": "acceptance and HLSL validity, not execution equivalence",
        "passed": sum(r["passed"] for r in results),
        "total": len(results),
        "dxc_valid_artifacts": sum(v["accepted"] and v["expected_accept"] for r in results for v in r["validators"]),
        "dxc_invalid_artifacts_rejected": sum(not v["accepted"] and not v["expected_accept"] for r in results for v in r["validators"]),
        "cases": results,
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"{report['passed']}/{report['total']} passed")
    raise SystemExit(0 if report["passed"] == report["total"] else 1)


if __name__ == "__main__":
    main()
