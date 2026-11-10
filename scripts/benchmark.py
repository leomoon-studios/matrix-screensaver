#!/usr/bin/env python3
"""Sample one Matrix run against a short idle baseline (NVIDIA hosts)."""

import argparse
import json
import os
import statistics
import subprocess
import time


def gpu_sample():
    output = subprocess.check_output(
        [
            "nvidia-smi",
            "--query-gpu=memory.used,utilization.gpu,power.draw",
            "--format=csv,noheader,nounits",
        ],
        text=True,
    ).splitlines()[0]
    return [float(value.strip()) for value in output.split(",")]


def cpu_counters():
    with open("/proc/stat", encoding="ascii") as stat:
        fields = [int(value) for value in stat.readline().split()[1:]]
    return sum(fields), fields[3] + fields[4]


def process_sample(pid):
    try:
        with open(f"/proc/{pid}/statm", encoding="ascii") as statm:
            rss = int(statm.read().split()[1]) * os.sysconf("SC_PAGE_SIZE") / 1024**2
        gpu = subprocess.check_output(
            [
                "nvidia-smi",
                "--query-compute-apps=pid,used_gpu_memory",
                "--format=csv,noheader,nounits",
            ],
            text=True,
        )
        gpu_mib = next(
            (float(parts[1].strip()) for line in gpu.splitlines()
             if len(parts := line.split(",")) == 2 and parts[0].strip() == str(pid)),
            None,
        )
        cpu = subprocess.check_output(["ps", "-p", str(pid), "-o", "%cpu="], text=True).strip()
        return [rss, gpu_mib, float(cpu)]
    except (OSError, subprocess.CalledProcessError):
        return [None, None, None]


def window_info(pid):
    try:
        clients = json.loads(subprocess.check_output(["hyprctl", "clients", "-j"], text=True))
        client = next(client for client in clients if client.get("pid") == pid)
        monitors = json.loads(subprocess.check_output(["hyprctl", "monitors", "-j"], text=True))
        monitor = next(monitor for monitor in monitors if monitor["id"] == client["monitor"])
        scale = monitor["scale"]
        return client, scale
    except (OSError, subprocess.CalledProcessError, StopIteration, KeyError, ValueError):
        return None


def window_size(pid):
    info = window_info(pid)
    if info is None:
        return None
    client, scale = info
    return {"logical": client["size"], "scale": scale,
            "estimated_physical": [round(value * scale) for value in client["size"]]}


def float_at_requested_size(pid, requested):
    info = window_info(pid)
    if info is None:
        raise RuntimeError("Could not find the Matrix window in Hyprland")
    client, scale = info
    width, height = [int(value) for value in requested.split("x")]
    address = f"address:{client['address']}"
    if not client["floating"]:
        subprocess.run(["hyprctl", "eval", f'hl.dispatch(hl.dsp.window.float("{address}"))'],
                       check=True, stdout=subprocess.DEVNULL)
    subprocess.run(["hyprctl", "eval",
                    f'hl.dispatch(hl.dsp.window.resize({{x={round(width / scale)},'
                    f'y={round(height / scale)},relative=false,window="{address}"}}))'],
                   check=True, stdout=subprocess.DEVNULL)
    time.sleep(1)


def collect(seconds, pid=None):
    samples = []
    start = time.monotonic()
    previous_cpu = cpu_counters()
    while time.monotonic() - start < seconds:
        sample = gpu_sample()
        current_cpu = cpu_counters()
        total_delta = current_cpu[0] - previous_cpu[0]
        idle_delta = current_cpu[1] - previous_cpu[1]
        sample.append(100 * (total_delta - idle_delta) / total_delta if total_delta else None)
        previous_cpu = current_cpu
        if pid is not None:
            sample += process_sample(pid)
        samples.append(sample)
        time.sleep(max(0, start + len(samples) - time.monotonic()))
    return samples


def range_of(samples, column):
    values = [row[column] for row in samples if row[column] is not None]
    return [round(min(values), 1), round(statistics.mean(values), 1), round(max(values), 1)] if values else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary")
    parser.add_argument("--seconds", type=int, default=60)
    parser.add_argument("--idle-seconds", type=int, default=10)
    parser.add_argument("--size", default="1280x720")
    parser.add_argument("--float", action="store_true", help="float on Hyprland at the requested physical size")
    args = parser.parse_args()
    idle = collect(args.idle_seconds)
    env = os.environ.copy()
    env["MATRIX_WINDOW_SIZE"] = args.size
    env["MATRIX_BENCHMARK"] = "1"
    process = subprocess.Popen([args.binary], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    try:
        time.sleep(3)
        if args.float:
            float_at_requested_size(process.pid, args.size)
        geometry = window_size(process.pid)
        running = collect(args.seconds, process.pid)
    finally:
        process.terminate()
        try:
            _, stderr = process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            _, stderr = process.communicate()
    report = {
        "binary": args.binary,
        "requested_size": args.size,
        "observed_window": geometry,
        "seconds": args.seconds,
        "idle": {name: range_of(idle, index) for index, name in enumerate(("gpu_memory_mib", "gpu_util_percent", "board_power_w", "system_cpu_percent"))},
        "running": {name: range_of(running, index) for index, name in enumerate(("gpu_memory_mib", "gpu_util_percent", "board_power_w", "system_cpu_percent", "rss_mib", "process_gpu_mib", "process_cpu_percent"))},
        "log": [line for line in stderr.splitlines() if "GPU ready:" in line or "Matrix benchmark:" in line or "could not initialize" in line],
    }
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
