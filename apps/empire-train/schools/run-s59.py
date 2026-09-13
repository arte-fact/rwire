#!/usr/bin/env python3
"""s59 — breeding against redrawing (13 September 2026).

The s58 recipe at 32 neurons (from scratch, against the seven delivered
`.f32`, `--hall 0`, `--longest 100`, `--rank 40`, 8 trials, 300
generations, a snapshot every 25), the population bred instead of
redrawn: the elite of 100 are parents, each begets nine children by
mutation at `--breed` times the starting scale of every weight, and the
next elite is chosen among parents and children — a hundred lineages
where the redraw fuses them into one. Two mutation scales, 0.1 and 0.3.
Then every snapshot measured on the GPU against the prod pool, into
`s59/measures.csv`; s58's h32 is the control. A config whose log is
complete is not played again.
"""

import csv
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import importlib.util

spec = importlib.util.spec_from_file_location("run_s58", Path(__file__).with_name("run-s58.py"))
s58 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s58)

HERE = Path(__file__).resolve().parent / "s59"
BREEDS = ["0.1", "0.3"]
HIDDEN = 32


def train(breed, engine):
    log = HERE / f"b{breed}.log"
    if log.exists() and "gen  299 " in log.read_text():
        return
    with log.open("w") as f:
        subprocess.run(
            [str(s58.TRAINER), *s58.RECIPE, "--generations", str(s58.GENERATIONS), "--trials", str(s58.TRIALS),
             "--hidden", str(HIDDEN), "--keep", str(s58.KEEP), "--breed", breed, *engine,
             "--out", str(HERE / f"b{breed}-{{n}}-war.json")],
            stdout=f, stderr=subprocess.STDOUT, check=True,
        )


def main():
    HERE.mkdir(exist_ok=True)
    engine = ["--gpu"] if "--gpu" in sys.argv else []
    for breed in BREEDS:
        train(breed, engine)
    rows = []
    for breed in BREEDS:
        for trial in range(1, s58.TRIALS + 1):
            stem = HERE / f"b{breed}-{trial}-war"
            snapshots = [(g, Path(f"{stem}.g{g}.json")) for g in range(s58.KEEP, s58.GENERATIONS, s58.KEEP)]
            snapshots.append((s58.GENERATIONS - 1, Path(f"{stem}.json")))
            for generation, path in snapshots:
                if path.exists():
                    row = s58.measure(HIDDEN, trial, generation, path)
                    row["breed"] = breed
                    rows.append(row)
    with (HERE / "measures.csv").open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"{len(rows)} measures written")


if __name__ == "__main__":
    main()
