#!/usr/bin/env python3
"""Fetch pinned public paper inputs and verify the derived workload bytes."""

from __future__ import annotations

import hashlib
from pathlib import Path
from urllib.request import urlopen


FIXTURES = {
    "chip8": {
        "url": "https://raw.githubusercontent.com/JohnEarnest/chip8Archive/761e3ffc63f43e6a849e80714122feff2afca208/roms/octojam2title.ch8",
        "name": "octojam2title.ch8",
        "sha256": "010f9ab220f55cc59645a6d05157b33a7099bac01037816ef06b74c2f8339b8a",
    },
    "pico8": {
        "url": "https://raw.githubusercontent.com/thxrsxm/fire-intro-effect/428e67017a8951d4ec32a7adadc0907aa9a36b6e/fireintro.p8",
        "name": "fireintro.p8",
        "sha256": "491888897d07e5699a5e411c9cb881181f313e6cad7b0dcfec2fd333ea0b833a",
    },
    "tic80": {
        "url": "https://raw.githubusercontent.com/lincerely/gecko/f958fd19218510468a36f93b77449ca26a2d13cf/game.tic",
        "name": "game.tic",
        "sha256": "f095a8ad534482a3aefbe0aa84c9b8bc47b74a89dd4202acbd5fc8b517995994",
    },
}
PICO8_TITLE_SOURCE = 'print("\\^w\\^tfire intro",25,30,7)'
PICO8_TITLE_PORTABLE = 'print("fire intro",25,30,7)'


def main() -> None:
    fixture_dir = Path(__file__).parent / "fixtures"
    fixture_dir.mkdir(exist_ok=True)
    for machine, fixture in FIXTURES.items():
        with urlopen(fixture["url"], timeout=60) as response:
            content = response.read()
        if machine == "pico8":
            text = content.decode("utf-8")
            if text.count(PICO8_TITLE_SOURCE) != 1:
                raise ValueError("pinned PICO-8 title source did not match expected adaptation")
            content = text.replace(PICO8_TITLE_SOURCE, PICO8_TITLE_PORTABLE).encode("utf-8")
        digest = hashlib.sha256(content).hexdigest()
        if digest != fixture["sha256"]:
            raise ValueError(f"{machine} fixture digest mismatch: {digest}")
        path = fixture_dir / fixture["name"]
        path.write_bytes(content)
        print(f"{machine}: {digest}  {path}")


if __name__ == "__main__":
    main()
