#!/usr/bin/env python3
"""s61 — what a brain reads, all eight ways (13 September 2026).

One genome shape for all: the sight holds the Chronique and last year's
orders, the recall, and the journal of the last four years, and a school
reads the blocks its flags say (the others are zero). The s59 recipe at
32 neurons (from scratch against the seven delivered `.f32`, `--hall 0`,
`--longest 100`, `--rank 40`, 8 trials, bred at 0.3), 100 generations, a
snapshot every 10, on the GPU, for every combination of `--told`,
`--recall` and `--journal`. Every snapshot measured against the prod
pool into `s61/measures.csv`.
"""

import csv
import importlib.util
import itertools
import subprocess
from pathlib import Path

spec = importlib.util.spec_from_file_location("run_s58", Path(__file__).with_name("run-s58.py"))
s58 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s58)

HERE = Path(__file__).resolve().parent / "s61"
FLAGS = ["--told", "--recall", "--journal"]
GENERATIONS = 100
KEEP = 10


def kinds():
    for bits in itertools.product([False, True], repeat=3):
        flags = [f for f, on in zip(FLAGS, bits) if on]
        name = "+".join(f[2:] for f in flags) or "none"
        yield name, flags


def train(name, flags):
    log = HERE / f"{name}.log"
    if log.exists() and f"gen  {GENERATIONS - 1} " in log.read_text():
        return
    with log.open("w") as f:
        subprocess.run(
            [str(s58.TRAINER), *s58.RECIPE, "--generations", str(GENERATIONS), "--trials", str(s58.TRIALS),
             "--hidden", "32", "--keep", str(KEEP), "--breed", "0.3", "--gpu", *flags,
             "--out", str(HERE / f"{name}-{{n}}-war.json")],
            stdout=f, stderr=subprocess.STDOUT, check=True,
        )


def main():
    HERE.mkdir(exist_ok=True)
    for name, flags in kinds():
        train(name, flags)
    rows = []
    for name, _ in kinds():
        for trial in range(1, s58.TRIALS + 1):
            stem = HERE / f"{name}-{trial}-war"
            snapshots = [(g, Path(f"{stem}.g{g}.json")) for g in range(KEEP, GENERATIONS, KEEP)]
            snapshots.append((GENERATIONS - 1, Path(f"{stem}.json")))
            for generation, path in snapshots:
                if path.exists():
                    row = s58.measure(32, trial, generation, path)
                    row["kind"] = name
                    rows.append(row)
    with (HERE / "measures.csv").open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"{len(rows)} measures written")


if __name__ == "__main__":
    main()
