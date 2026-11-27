"""Render README figures from a saved benchmark report with Matplotlib."""

import argparse
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    parser.add_argument("figure", type=Path)
    args = parser.parse_args()
    report = json.loads(args.report.read_text(encoding="utf-8"))
    cases = report["cases"]
    labels = list(cases)
    positions = range(len(labels))
    median = [case["median_ms"] for case in cases.values()]
    p95 = [case["p95_ms"] for case in cases.values()]
    sizes = [case["binary_bytes"] / 2**20 for case in cases.values()]

    fig, (latency, size) = plt.subplots(
        1, 2, figsize=(11, 3.6), sharey=True, width_ratios=[1.22, 1], layout="constrained"
    )
    fig.patch.set_facecolor("#f3f6fb")
    fig.suptitle("HELLO WORLD  /  PERFORMANCE BASELINE", x=0.02, ha="left", weight="bold", color="#16263c")
    for axis in (latency, size):
        axis.set_facecolor("#ffffff")
        axis.spines[:].set_visible(False)
        axis.set_axisbelow(True)
        axis.grid(axis="x", color="#dce5f0")
        axis.tick_params(length=0, pad=7, colors="#263c54")

    latency.set_title("01   Full process latency", loc="left", weight="bold")
    latency.hlines(positions, median, p95, color="#7297be", linewidth=3)
    latency.scatter(median, positions, color="#13568b", label="p50", zorder=3)
    latency.scatter(p95, positions, color="#ab451b", marker="D", label="p95", zorder=3)
    latency.set_xscale("log")
    latency.xaxis.set_major_formatter(FuncFormatter(lambda value, _: f"{value:g}"))
    latency.set_yticks(list(positions), labels)
    latency.invert_yaxis()
    latency.set_xlabel("Milliseconds, log scale")
    latency.legend(loc="upper right", frameon=False)

    size.set_title("02   Executable size", loc="left", weight="bold")
    bars = size.barh(positions, sizes, color=["#7894af", "#13568b", "#ab451b"], height=0.45)
    size.bar_label(bars, labels=[f"{value:.2f}" for value in sizes], padding=4, color="#263c54")
    size.set_xlim(0, max(sizes) * 1.2)
    size.set_xlabel("MiB, smaller is better")

    fig.savefig(args.figure, dpi=150, facecolor=fig.get_facecolor())
    plt.close(fig)


if __name__ == "__main__":
    main()
