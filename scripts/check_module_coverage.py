#!/usr/bin/env python3
"""Fail when any RustyDB logical module is below the line-coverage threshold."""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path


MODULES = (
    "app",
    "command",
    "database",
    "executor",
    "line_protocol",
    "line_session",
    "output",
    "server",
    "storage",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path, help="cargo llvm-cov JSON report")
    parser.add_argument("--threshold", type=float, default=70.0)
    return parser.parse_args()


def source_module(relative_path: Path) -> str | None:
    parts = relative_path.parts

    if relative_path.name == "tests.rs" or "tests" in parts[:-1]:
        return None

    if len(parts) == 1:
        return relative_path.stem if relative_path.stem in MODULES else None

    return parts[0] if parts[0] in MODULES else None


def main() -> int:
    args = parse_args()
    report = json.loads(args.report.read_text(encoding="utf-8"))
    manifest = Path(report["cargo_llvm_cov"]["manifest_path"]).resolve()
    source_root = manifest.parent / "src"
    totals: dict[str, list[int]] = defaultdict(lambda: [0, 0])

    for file_data in report["data"][0]["files"]:
        path = Path(file_data["filename"]).resolve()
        try:
            relative_path = path.relative_to(source_root)
        except ValueError:
            continue

        module = source_module(relative_path)
        if module is None:
            continue

        lines = file_data["summary"]["lines"]
        totals[module][0] += int(lines["covered"])
        totals[module][1] += int(lines["count"])

    failures = []
    print(f"{'Module':<12} {'Covered':>9} {'Total':>9} {'Coverage':>10}")
    print("-" * 43)

    for module in MODULES:
        covered, count = totals[module]
        percentage = covered * 100.0 / count if count else 0.0
        print(f"{module:<12} {covered:>9} {count:>9} {percentage:>9.2f}%")

        if count == 0:
            failures.append(f"{module}: no executable lines found")
        elif percentage <= args.threshold:
            failures.append(
                f"{module}: {percentage:.2f}% is not greater than {args.threshold:.2f}%"
            )

    if failures:
        print("\nCoverage gate failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print(f"\nEvery module is above {args.threshold:.2f}% line coverage.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
