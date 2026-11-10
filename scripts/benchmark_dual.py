#!/usr/bin/env python3
"""Benchmark two simultaneous Matrix windows on adjacent Hyprland monitors."""

import argparse
import json
import os
import subprocess
import time

import benchmark as b


def summarize(rows):
    names = ("gpu_memory_mib", "gpu_util_percent", "board_power_w", "system_cpu_percent",
             "rss_sum_mib", "process_gpu_sum_mib", "process_cpu_sum_percent")
    return {name: b.range_of(rows, index) for index, name in enumerate(names)}


def collect(seconds, pids):
    rows = []
    start = time.monotonic()
    previous_cpu = b.cpu_counters()
    while time.monotonic() - start < seconds:
        gpu = b.gpu_sample()
        current_cpu = b.cpu_counters()
        total = current_cpu[0] - previous_cpu[0]
        idle = current_cpu[1] - previous_cpu[1]
        previous_cpu = current_cpu
        gpu.append(100 * (total - idle) / total if total else None)
        processes = [b.process_sample(pid) for pid in pids]
        gpu += [sum(row[index] for row in processes) if processes else None for index in range(3)]
        rows.append(gpu)
        time.sleep(max(0, start + len(rows) - time.monotonic()))
    return rows


def focus_monitor(name):
    subprocess.run(["hyprctl", "eval", f'hl.dispatch(hl.dsp.focus({{monitor="{name}"}}))'],
                   check=True, stdout=subprocess.DEVNULL)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary")
    parser.add_argument("--seconds", type=int, default=60)
    parser.add_argument("--idle-seconds", type=int, default=10)
    parser.add_argument("--low-power", action="store_true")
    args = parser.parse_args()
    monitors = json.loads(subprocess.check_output(["hyprctl", "monitors", "-j"], text=True))
    if len(monitors) != 2:
        raise RuntimeError("Expected exactly two connected monitors")
    original_focus = next(monitor["name"] for monitor in monitors if monitor["focused"])
    baseline = collect(args.idle_seconds, [])
    env = os.environ.copy()
    env["MATRIX_BENCHMARK"] = "1"
    if args.low_power:
        env["MATRIX_LOW_POWER"] = "1"
    processes = []
    try:
        focus_monitor(monitors[0]["name"])
        first = subprocess.Popen([args.binary], env=env, stdout=subprocess.DEVNULL,
                                 stderr=subprocess.PIPE, text=True)
        processes.append(first)
        time.sleep(3)
        focus_monitor(monitors[1]["name"])
        second = subprocess.Popen([args.binary], env=env, stdout=subprocess.DEVNULL,
                                  stderr=subprocess.PIPE, text=True)
        processes.append(second)
        time.sleep(3)
        first_info = b.window_info(first.pid)
        second_info = b.window_info(second.pid)
        if first_info[0]["monitor"] == second_info[0]["monitor"]:
            raise RuntimeError("Both Matrix windows are still on the same monitor")
        windows = [dict(pid=p.pid, monitor=info[0]["monitor"], geometry=b.window_size(p.pid))
                   for p, info in zip(processes, (first_info, second_info))]
        running = collect(args.seconds, [p.pid for p in processes])
    finally:
        focus_monitor(original_focus)
        logs = []
        for process in processes:
            process.terminate()
            try:
                _, stderr = process.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                _, stderr = process.communicate()
            logs.append([line for line in stderr.splitlines() if "GPU ready:" in line or "Matrix benchmark:" in line])
    print(json.dumps({"monitors": windows, "idle": summarize(baseline),
                      "running": summarize(running), "logs": logs}, indent=2))


if __name__ == "__main__":
    main()
