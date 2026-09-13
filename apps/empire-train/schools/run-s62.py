#!/usr/bin/env python3
"""s62 — the first campaign under the rules' version 2 (13 September 2026).

Version 2: a want buys its fourth power of what the treasury allows, a
table ends at the first Emperor, at most one ram per tenth of wall is
brought up per pace, the sight carries the journal (four years, with
what the éclaireurs read). No brain of version 1 is kept: the pool of
rivals is raised here.

Stage A, the pool: 8 trials from nothing, among themselves (no rival),
bred at 0.3, reading everything (Chronique, recall, journal), 100
generations; their bests become the pool. If fewer than four of the
eight find the crown, stage A is played again against the old delivered
`.f32` (which still play, weaker, under version 2) so that the pool has
conquerors in it.

Stage B, the factorial: the eight readings (every combination of
`--told`, `--recall`, `--journal`), each 8 trials bred at 0.3, 100
generations against the pool, a snapshot every 10; `none` is the
control. Every snapshot measured against the pool over 500 tables into
`s62/measures.csv`.
"""

import csv
import importlib.util
import itertools
import re
import subprocess
from pathlib import Path

spec = importlib.util.spec_from_file_location("run_s58", Path(__file__).with_name("run-s58.py"))
s58 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s58)

HERE = Path(__file__).resolve().parent / "s62"
TRIALS = 8
GENERATIONS = 100
KEEP = 10
BASE = ["--stage", "war", "--hall", "0", "--longest", "100", "--rank", "40", "--tables", "32",
        "--hidden", "32", "--breed", "0.3", "--trials", str(TRIALS), "--gpu"]
FLAGS = ["--told", "--recall", "--journal"]
CROWNED = re.compile(r"^try +(\d+) · gen +99 .*crowned +([\d.]+)%")


def train(name, against, flags):
    log = HERE / f"{name}.log"
    if log.exists() and "gen  99 " in log.read_text():
        return log
    with log.open("w") as f:
        subprocess.run(
            [str(s58.TRAINER), *BASE, *against, "--generations", str(GENERATIONS), "--keep", str(KEEP), *flags,
             "--out", str(HERE / f"{name}-{{n}}-war.json")],
            stdout=f, stderr=subprocess.STDOUT, check=True,
        )
    return log


def crowning(log):
    return sum(1 for line in log.read_text().splitlines() for m in [CROWNED.match(line)] if m and float(m.group(2)) > 0)


def kinds():
    for bits in itertools.product([False, True], repeat=3):
        flags = [f for f, on in zip(FLAGS, bits) if on]
        yield "+".join(f[2:] for f in flags) or "none", flags


def main():
    HERE.mkdir(exist_ok=True)
    log = train("poolA", [], FLAGS)
    pool = "poolA"
    print(f"stage A: {crowning(log)} of {TRIALS} trials crown among themselves", flush=True)
    if crowning(log) < 4:
        log = train("poolA2", s58.AGAINST, FLAGS)
        pool = "poolA2"
        print(f"stage A against the old .f32: {crowning(log)} of {TRIALS} trials crown", flush=True)
    against = [x for n in range(1, TRIALS + 1) for x in ("--against", str(HERE / f"{pool}-{n}-war.json"))]
    for name, flags in kinds():
        train(name, against, flags)
    rows = []
    for name, _ in kinds():
        for trial in range(1, TRIALS + 1):
            stem = HERE / f"{name}-{trial}-war"
            snapshots = [(g, Path(f"{stem}.g{g}.json")) for g in range(KEEP, GENERATIONS, KEEP)]
            snapshots.append((GENERATIONS - 1, Path(f"{stem}.json")))
            for generation, path in snapshots:
                if not path.exists():
                    continue
                out = subprocess.run(
                    [str(s58.TRAINER), "--stage", "war", *against, "--longest", "100", "--rank", "40", "--hidden", "32",
                     "--from", str(path), "--measure", str(s58.MEASURE), "--gpu"],
                    capture_output=True, text=True, check=True,
                )
                text = out.stdout + out.stderr
                m = s58.BILAN.search(text)
                r = s58.RIVALS.search(text)
                if not m:
                    raise RuntimeError(f"no bilan for {path}:\n{text}")
                prince, king, crowned, p10, median, p90, fell, mother, starved, nobles, read, fitness = m.groups()
                rows.append(dict(
                    kind=name, hidden=32, trial=trial, generation=generation, prince=prince, king=king, crowned=crowned,
                    p10=p10 or "", median=median or "", p90=p90 or "", fell=fell, mother=mother, starved=starved,
                    nobles=nobles, read=read, fitness=fitness, rivals_crowned=r.group(1) if r else "",
                ))
    with (HERE / "measures.csv").open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"{len(rows)} measures written", flush=True)


if __name__ == "__main__":
    main()
