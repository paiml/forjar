#!/usr/bin/env python3
"""Read forjar bench --json from stdin, emit benchmarks/RESULTS.md to stdout."""

import json
import sys
from datetime import datetime, timezone


def fmt_us(us: float) -> str:
    if us >= 1_000_000:
        return f"{us / 1_000_000:.2f}s"
    if us >= 1000:
        return f"{us / 1000:.1f}ms"
    return f"{us:.1f}\u00b5s"


def main():
    data = json.load(sys.stdin)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    print("# Benchmark Results\n")
    print(f"Last updated: {now}\n")
    print("<!-- BENCH-TABLE-START -->")
    print("| Operation | Target | Last Run | p50 | p95 | Status |")
    print("|-----------|--------|----------|-----|-----|--------|")
    for r in data:
        avg = fmt_us(r["avg_us"])
        p50 = fmt_us(r["p50_us"])
        p95 = fmt_us(r["p95_us"])
        print(f"| {r['name']} | {r['target']} | {avg} | {p50} | {p95} | {r['status']} |")
    print("<!-- BENCH-TABLE-END -->")


if __name__ == "__main__":
    main()
