#!/usr/bin/env python3
"""s62c — s62's readings raised to 600 generations (13 September 2026, the
GPU machine).

The s62 schools themselves stayed on the machine of origin (the genomes are
not in git), so the factorial is played again from nothing under the same
recipe against the frozen pool A (`pools/A`, 8 schools of zero bred among
themselves under version 2): the eight readings (every combination of
`--told`, `--recall`, `--journal`), each 8 trials bred at 0.3, 600
generations, a snapshot every 25; `none` is the control. Every snapshot is
measured against the pool over 500 tables into `s62/measures-600.csv`, the
CSV rewritten after each reading so a partial campaign can be read.

New since s62: a company sits 32 tables (`--tables 32`, the GPU's workgroup):
every genome is scored on 32 games of its company, not 6.

Readings named on the command line restrict the run (default: all eight).
"""

import csv
import importlib.util
import subprocess
import sys
from pathlib import Path

spec = importlib.util.spec_from_file_location("run_s62", Path(__file__).with_name("run-s62.py"))
s62 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s62)
s58 = s62.s58
HERE = s62.HERE
POOL = Path(__file__).resolve().parent / "pools" / "A"
GENERATIONS = 600
KEEP = 25
AGAINST = [x for n in range(1, 9) for x in ("--against", str(POOL / f"pool-{n}-war.json"))]
KINDS = {name: flags for name, flags in s62.kinds()}
if len(sys.argv) > 1:
    KINDS = {name: KINDS[name] for name in sys.argv[1:]}
FIELDS = ["kind", "hidden", "trial", "generation", "prince", "king", "crowned", "p10", "median", "p90",
          "fell", "mother", "starved", "nobles", "read", "fitness", "rivals_crowned"]


def train(name, flags):
    log = HERE / f"{name}-600.log"
    if log.exists() and f"gen  {GENERATIONS - 1} " in log.read_text():
        return
    with log.open("w") as f:
        subprocess.run(
            [str(s58.TRAINER), *s62.BASE, *AGAINST, "--generations", str(GENERATIONS), "--keep", str(KEEP), *flags,
             "--out", str(HERE / f"{name}-600-{{n}}-war.json")],
            stdout=f, stderr=subprocess.STDOUT, check=True,
        )


def measure(name):
    rows = []
    for trial in range(1, s62.TRIALS + 1):
        stem = HERE / f"{name}-600-{trial}-war"
        snapshots = [(g, Path(f"{stem}.g{g}.json")) for g in range(KEEP, GENERATIONS, KEEP)]
        snapshots.append((GENERATIONS - 1, Path(f"{stem}.json")))
        for generation, path in snapshots:
            if not path.exists():
                continue
            out = subprocess.run(
                [str(s58.TRAINER), "--stage", "war", *AGAINST, "--longest", "100", "--rank", "40", "--hidden", "32",
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
    return rows


def main():
    HERE.mkdir(exist_ok=True)
    out = HERE / "measures-600.csv"
    rows = []
    if out.exists():
        rows = [r for r in csv.DictReader(out.open()) if r["kind"] not in KINDS]
    for name, flags in KINDS.items():
        print(f"{name}: training", flush=True)
        train(name, flags)
        print(f"{name}: measuring", flush=True)
        rows = [r for r in rows if r["kind"] != name] + measure(name)
        with out.open("w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=FIELDS)
            w.writeheader()
            w.writerows(rows)
        print(f"{name}: {len(rows)} measures written", flush=True)
    print("campaign done", flush=True)


if __name__ == "__main__":
    main()
