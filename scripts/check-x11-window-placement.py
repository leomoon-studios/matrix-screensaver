#!/usr/bin/env python3
"""Require one full-size X11 screensaver window on each Xorg monitor."""

import re
import subprocess
import sys
from pathlib import Path


def monitor_boxes(path: Path) -> set[tuple[int, int, int, int]]:
    boxes = set()
    for line in path.read_text().splitlines()[1:]:
        match = re.search(r"(\d+)/\d+x(\d+)/\d+\+(-?\d+)\+(-?\d+)", line)
        if match:
            width, height, x, y = map(int, match.groups())
            boxes.add((x, y, width, height))
    return boxes


def window_box(window: str) -> tuple[int, int, int, int]:
    output = subprocess.check_output(
        ["xdotool", "getwindowgeometry", "--shell", window], text=True
    )
    values = dict(line.split("=", 1) for line in output.splitlines() if "=" in line)
    return tuple(int(values[key]) for key in ("X", "Y", "WIDTH", "HEIGHT"))


def main() -> int:
    monitors = monitor_boxes(Path(sys.argv[1]))
    windows = {
        window_box(window)
        for window in Path(sys.argv[2]).read_text().splitlines()
        if window
    }
    if not monitors or not monitors.issubset(windows):
        print(f"X11 placement mismatch: monitors={monitors}, windows={windows}", file=sys.stderr)
        return 1
    print(f"X11 windows cover {len(monitors)} monitors")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
