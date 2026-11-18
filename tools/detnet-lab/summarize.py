#!/usr/bin/env python3
"""Aggregate results/summary.csv over repetitions: median of each metric per
(profile, load), printed as a markdown table. Latencies in ms."""
import csv
import statistics
import sys
from collections import defaultdict

path = sys.argv[1] if len(sys.argv) > 1 else "results/summary.csv"
groups = defaultdict(list)
order = []
with open(path) as f:
    for row in csv.DictReader(f):
        key = (row["profile"], int(row["load"]))
        if key not in groups:
            order.append(key)
        groups[key].append(row)

cols = ["p50_us", "p99_us", "p999_us", "max_us"]
print("| Profile | Load % | Runs | Loss % | p50 | p99 | p99.9 | max (worst run) |")
print("| --- | --- | --- | --- | --- | --- | --- | --- |")
for key in sorted(order, key=lambda k: (["stock", "prio", "lane"].index(k[0])
                                        if k[0] in ("stock", "prio", "lane") else 9, k[1])):
    rows = groups[key]
    med = {c: statistics.median(float(r[c]) for r in rows) / 1000 for c in cols}
    worst = max(float(r["max_us"]) for r in rows) / 1000
    loss = statistics.median(float(r["loss_pct"]) for r in rows)
    fmt = lambda v: f"{v:.2f}" if v < 10 else f"{v:.0f}"
    print(f"| {key[0]} | {key[1]} | {len(rows)} | {loss:.2f} | {fmt(med['p50_us'])} | "
          f"{fmt(med['p99_us'])} | {fmt(med['p999_us'])} | {fmt(worst)} |")
