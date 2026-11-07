"""Record upstream source/test paths and digests; this is not a coverage report."""

import argparse
import hashlib
import json
from pathlib import Path

AREAS = ("source/slang", "source/core", "source/compiler-core", "include", "tests")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", type=Path, default=Path("docs/upstream-inventory.json"))
    args = parser.parse_args()
    if not args.source.is_dir():
        parser.error("upstream checkout does not exist")
    summary = {}
    records = []
    for area in AREAS:
        files = sorted(path for path in (args.source / area).rglob("*") if path.is_file())
        code_lines = 0
        for path in files:
            data = path.read_bytes()
            lines = len(data.splitlines())
            if path.suffix in (".c", ".cpp", ".h", ".slang"):
                code_lines += lines
            records.append({
                "path": path.relative_to(args.source).as_posix(),
                "area": area,
                "sha256": hashlib.sha256(data).hexdigest(),
                "lines": lines,
                "parity_status": "unverified",
            })
        summary[area] = {"files": len(files), "code_lines": code_lines}
    result = {"revision": args.revision, "meaning": "inventory, not test coverage", "summary": summary, "files": records}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
