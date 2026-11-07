"""Validate native HLSL fixtures with the entry and profile from reflection."""

import argparse
import json
from pathlib import Path

from conformance import run


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
        compilation = run(command)
        validation = None
        entry = None
        if compilation["accepted"]:
            reflection = json.loads(reflection_path.read_text(encoding="utf-8"))
            entry = next(e for e in reflection["entry_points"] if e["name"] == case["entry"])
            validation = run([
                str(args.dxc.resolve()), "-T", entry["target_profile"],
                "-E", entry["target_name"], "-Fo", str(output.with_suffix(".dxil")),
                str(output),
            ])
        passed = compilation["accepted"] and validation is not None and validation["accepted"]
        results.append({
            "file": case["file"], "passed": passed, "entry": entry,
            "compilation": compilation, "validation": validation,
        })
        print(f"{'PASS' if passed else 'FAIL'} {case['file']}")
    report = {
        "kind": "native HLSL validity, not upstream parity or GPU execution",
        "passed": sum(r["passed"] for r in results),
        "total": len(results),
        "cases": results,
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"{report['passed']}/{report['total']} passed")
    raise SystemExit(0 if report["passed"] == report["total"] else 1)


if __name__ == "__main__":
    main()
