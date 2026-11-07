"""Compare acceptance with standalone Slang and validate positive HLSL with DXC."""

import argparse
import json
import subprocess
from pathlib import Path


def run(command):
    result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60)
    return {"accepted": result.returncode == 0, "returncode": result.returncode, "stdout": result.stdout, "stderr": result.stderr}


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
        passed = rust["accepted"] == oracle["accepted"] == case["accept"]
        if "rust_code" in case:
            passed = passed and case["rust_code"] in rust["stderr"]
        validators = []
        if args.dxc and case["accept"] and rust["accepted"] and oracle["accepted"]:
            for output in (rust_output, oracle_output):
                validation = run([str(args.dxc.resolve()), "-T", case.get("profile", "cs_6_0"), "-E", case["entry"], "-Fo", str(output.with_suffix(".dxil")), str(output)])
                validators.append(validation)
                passed = passed and validation["accepted"]
        results.append({"file": case["file"], "passed": passed, "rust": rust, "oracle": oracle, "validators": validators})
        print(f"{'PASS' if passed else 'FAIL'} {case['file']}")
    report = {"kind": "acceptance and HLSL validity, not execution equivalence", "passed": sum(r["passed"] for r in results), "total": len(results), "cases": results}
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"{report['passed']}/{report['total']} passed")
    raise SystemExit(0 if report["passed"] == report["total"] else 1)


if __name__ == "__main__":
    main()
