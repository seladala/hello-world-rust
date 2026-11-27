"""Measure complete Hello World invocations on Linux and emit a JSON report."""

import argparse
import json
import math
import os
from pathlib import Path
import platform
import random
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
EXPECTED = b"hello world\n"


def invoke(binary, env):
    start = time.perf_counter_ns()
    result = subprocess.run([binary], cwd=ROOT, env=env, capture_output=True, check=True)
    duration_ms = (time.perf_counter_ns() - start) / 1_000_000
    if result.stdout != EXPECTED:
        raise RuntimeError(f"{binary} returned unexpected stdout: {result.stdout!r}")
    return duration_ms


def measure(cases, env, runs, warmups):
    samples = {name: [] for name in cases}
    order = list(cases)
    rng = random.Random(67)
    for index in range(warmups + runs):
        rng.shuffle(order)
        for name in order:
            duration = invoke(cases[name], env)
            if index >= warmups:
                samples[name].append(duration)
    return {
        name: {
            "median_ms": round(statistics.median(values), 4),
            "p95_ms": round(sorted(values)[math.ceil(len(values) * 0.95) - 1], 4),
            "binary_bytes": cases[name].stat().st_size,
            "samples_ms": [round(value, 4) for value in values],
        }
        for name, values in samples.items()
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=50)
    parser.add_argument("--warmups", type=int, default=5)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--summary", type=Path)
    args = parser.parse_args()
    if args.runs < 1 or args.warmups < 0:
        parser.error("--runs must be positive and --warmups must be nonnegative")
    if platform.system() != "Linux":
        parser.error("this benchmark report is configured for Linux")

    def command(*argv):
        return subprocess.check_output(argv, cwd=ROOT, text=True).strip()

    with tempfile.TemporaryDirectory(prefix="hello-world-benchmark-") as tmp:
        source = Path(tmp) / "minimal.rs"
        source.write_text('fn main() { println!("hello world"); }\n', encoding="utf-8")
        baseline = Path(tmp) / "minimal"
        subprocess.run(["rustc", "-C", "opt-level=3", source, "-o", baseline], cwd=ROOT, check=True)
        metadata = command("cargo", "metadata", "--locked", "--no-deps", "--format-version", "1")
        target = Path(json.loads(metadata)["target_directory"])
        cases = {
            "Minimal Rust (-O)": baseline,
            "Hello World (release)": target / "release" / "hello-world",
            "Hello World (debug)": target / "debug" / "hello-world",
        }
        env = dict(os.environ, HELLO_WORLD_ROOT=str(ROOT))
        metrics = measure(cases, env, args.runs, args.warmups)

    cpuinfo = Path("/proc/cpuinfo").read_text(encoding="utf-8")
    cpu = next((line.split(":", 1)[1].strip() for line in cpuinfo.splitlines() if line.startswith("model name")), "unknown")
    report = {
        "source_commit": command("git", "rev-parse", "HEAD"),
        "rustc": command("rustc", "--version"),
        "platform": platform.platform(),
        "cpu": cpu,
        "runs_per_case": args.runs,
        "warmups_per_case": args.warmups,
        "cases": metrics,
    }
    output = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.write_text(output, encoding="utf-8")
    else:
        print(output, end="")

    if args.summary:
        rows = [
            "## End-to-end greeting benchmark",
            "",
            f"{args.runs} samples per case after {args.warmups} warmups. Smaller is faster.",
            "",
            "| Variant | Median (ms) | p95 (ms) | Binary (MiB) |",
            "| --- | ---: | ---: | ---: |",
        ]
        for name, values in metrics.items():
            rows.append(
                f"| {name} | {values['median_ms']:.4f} | {values['p95_ms']:.4f} | "
                f"{values['binary_bytes'] / 2**20:.2f} |"
            )
        with args.summary.open("a", encoding="utf-8") as handle:
            handle.write("\n".join(rows) + "\n")


if __name__ == "__main__":
    main()
