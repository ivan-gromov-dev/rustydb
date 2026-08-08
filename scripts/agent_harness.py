#!/usr/bin/env python3
"""Deterministic local verification harness for humans and coding agents."""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent

PROFILES = {
    "fast": [
        ["cargo", "fmt", "--", "--check"],
        ["cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"],
        ["cargo", "test", "--all-features"],
    ],
    "full": [
        ["cargo", "fmt", "--", "--check"],
        ["cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"],
        ["cargo", "test", "--all-features"],
        ["git", "diff", "--check"],
    ],
    "coverage": [
        ["cargo", "llvm-cov", "--workspace", "--all-features", "--json", "--output-path", "coverage.json"],
        [sys.executable, "scripts/check_module_coverage.py", "coverage.json", "--threshold", "70"],
    ],
}


def main() -> int:
    profile = sys.argv[1] if len(sys.argv) == 2 else "full"
    if profile not in PROFILES:
        print(f"usage: {Path(sys.argv[0]).name} [{', '.join(PROFILES)}]", file=sys.stderr)
        return 2

    for command in PROFILES[profile]:
        if shutil.which(command[0]) is None:
            print(f"missing required executable: {command[0]}", file=sys.stderr)
            return 127
        print(f"\n==> {subprocess.list2cmdline(command)}", flush=True)
        result = subprocess.run(command, cwd=ROOT, check=False)
        if result.returncode != 0:
            return result.returncode

    print(f"\n{profile} verification passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

