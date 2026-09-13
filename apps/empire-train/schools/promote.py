#!/usr/bin/env python3
"""One champion at a time: a campaign's schools try the pool of reference.

    promote.py <pool dir> <next pool dir> <school.json>...

Every candidate is measured against the pool (one seat against five of
it, `MEASURE` tables) and, to catch a specialist, against the other
candidates; a candidate's mark is the lesser of its two crown rates.
The best candidate replaces the weakest member of the pool — the one
that crowns least against its fellows — if it crowns more than that
member does; otherwise the pool stands. The next pool is written as a
fresh directory of eight schools with a `pool.json` telling where each
came from and what was measured. Pools are never edited in place.
"""

import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TRAINER = ROOT / "target/release/empire-train"
MEASURE = 500
BILAN = re.compile(r"one of --from .* crowned ([\d.]+)%(?: \(years: p10 \d+ · median (\d+))?")


def crowns(school, against):
    """A school's crown rate (and median crown year) against five of `against`."""
    out = subprocess.run(
        [str(TRAINER), "--stage", "war", *[x for a in against for x in ("--against", str(a))],
         "--longest", "100", "--rank", "40", "--from", str(school), "--measure", str(MEASURE), "--gpu"],
        capture_output=True, text=True, check=True,
    )
    m = BILAN.search(out.stdout + out.stderr)
    if not m:
        raise RuntimeError(f"no bilan for {school}:\n{out.stdout}{out.stderr}")
    return float(m.group(1)), int(m.group(2)) if m.group(2) else None


def main():
    pool_dir, next_dir, *candidates = map(Path, sys.argv[1:])
    pool = sorted(p for p in pool_dir.glob("*.json") if p.name != "pool.json")
    assert len(pool) == 8, f"{pool_dir} holds {len(pool)} schools, not 8"
    assert not next_dir.exists(), f"{next_dir} exists: pools are never edited"
    report = {"from": str(pool_dir), "members": [], "candidates": []}
    # The pool's members against their fellows: the weakest goes.
    weakest, weakest_rate = None, None
    for member in pool:
        rate, year = crowns(member, [m for m in pool if m != member])
        report["members"].append({"school": str(member), "against_fellows": rate, "year": year})
        print(f"member {member.name}: {rate:.1f} % against its fellows", flush=True)
        if weakest_rate is None or rate < weakest_rate:
            weakest, weakest_rate = member, rate
    # The candidates against the pool and against each other.
    best, best_mark = None, None
    for c in candidates:
        against_pool, year = crowns(c, pool)
        others = [o for o in candidates if o != c]
        against_peers = crowns(c, others)[0] if len(others) >= 5 else against_pool
        mark = min(against_pool, against_peers)
        report["candidates"].append({"school": str(c), "against_pool": against_pool, "against_peers": against_peers,
                                     "mark": mark, "year": year})
        print(f"candidate {c.name}: {against_pool:.1f} % against the pool, {against_peers:.1f} % against its peers, "
              f"year {year}", flush=True)
        if best_mark is None or mark > best_mark:
            best, best_mark = c, mark
    next_dir.mkdir(parents=True)
    promoted = best_mark > weakest_rate
    for i, member in enumerate(pool, 1):
        source = best if promoted and member == weakest else member
        shutil.copy(source, next_dir / f"pool-{i}-war.json")
    report["promoted"] = str(best) if promoted else None
    report["replaced"] = str(weakest) if promoted else None
    report["weakest_rate"] = weakest_rate
    report["best_mark"] = best_mark
    (next_dir / "pool.json").write_text(json.dumps(report, indent=2))
    if promoted:
        print(f"{best.name} ({best_mark:.1f} %) replaces {weakest.name} ({weakest_rate:.1f} %): {next_dir}")
    else:
        print(f"no candidate beats the weakest member ({weakest_rate:.1f} %): {next_dir} is the pool unchanged")


if __name__ == "__main__":
    main()
