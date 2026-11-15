#!/usr/bin/env python3
"""Mean ± sd over seeds of detnet-sim CSV rows, as a markdown table.

Usage: summarize.py FILE.csv --by col,col --metrics col,col [--pct col,col]
"""
import argparse
import csv
import math
import statistics
from collections import OrderedDict

ap = argparse.ArgumentParser()
ap.add_argument("file")
ap.add_argument("--by", required=True)
ap.add_argument("--metrics", required=True)
ap.add_argument("--pct", default="", help="metrics shown as percent")
a = ap.parse_args()
by, metrics, pct = a.by.split(","), a.metrics.split(","), set(filter(None, a.pct.split(",")))

groups = OrderedDict()
with open(a.file) as f:
    for row in csv.DictReader(f):
        groups.setdefault(tuple(row[c] for c in by), []).append(row)


def cell(rows, m):
    xs = [float(r[m]) for r in rows if r[m] not in ("NaN", "nan")]
    if not xs:
        return "–"
    k = 100.0 if m in pct else 1.0
    mu = statistics.mean(xs) * k
    sd = statistics.stdev(xs) * k if len(xs) > 1 else 0.0
    digits = 1 if m in pct or abs(mu) >= 10 else 2
    if math.isclose(sd, 0.0, abs_tol=10 ** -digits / 2):
        return f"{mu:.{digits}f}"
    return f"{mu:.{digits}f} ± {sd:.{digits}f}"


print("| " + " | ".join(by + metrics) + " |")
print("|" + " --- |" * (len(by) + len(metrics)))
for key, rows in groups.items():
    print("| " + " | ".join(list(key) + [cell(rows, m) for m in metrics]) + " |")
