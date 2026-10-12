#!/usr/bin/env python3
"""List unresolved manuscript markers and reject them in final mode."""

from __future__ import annotations

import argparse
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SOURCE_SUFFIXES = {".tex", ".bib"}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--final",
        action="store_true",
        help="fail if any TBA: marker remains in TeX or bibliography sources",
    )
    args = parser.parse_args()

    matches: list[tuple[Path, int, str]] = []
    for path in sorted(ROOT.rglob("*")):
        if not path.is_file() or path.suffix not in SOURCE_SUFFIXES or "build" in path.parts:
            continue
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if "TBA:" in line:
                matches.append((path.relative_to(ROOT), line_number, line.strip()))

    if matches:
        for path, line_number, line in matches:
            print(f"{path}:{line_number}: {line}")
        print(f"{len(matches)} unresolved TBA marker line(s).")
        return 1 if args.final else 0

    print("No TBA markers found in TeX or bibliography sources.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
