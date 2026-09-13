#!/usr/bin/env python3
"""s58 — the capacity control of phase 1 (13 September 2026).

The brain of today (one year of sight) with a hidden layer of 4, 8, 16,
32, 64, 128 or 256 neurons, everything else the s54 recipe: from
scratch, against the seven delivered `.f32` (the game's computers),
`--hall 0`, `--longest 100`, `--rank 40`, 8 trials per width in one
process, 300 generations, a snapshot every 25. Then every snapshot is
measured on the GPU, one seat against five of the prod pool over 500
tables, into `s58/measures.csv` for the report. A width whose log is
complete is not played again, so the run can be resumed (the first run
played 16, 8 and 4 on the CPU, `s58/narrow.sh`, while the GPU played the
wide ones).
"""

import csv
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TRAINER = ROOT / "target/release/empire-train"
BRAINS = ROOT / "libs/empire-lib/brains"
HERE = Path(__file__).resolve().parent / "s58"
PROD = ["soldat", "batisseuse", "garnison", "boutiquiere", "fonceuse", "conquerante", "prudente"]
AGAINST = [x for p in PROD for x in ("--against", str(BRAINS / f"{p}.f32"))]
WIDTHS = [4, 8, 16, 32, 64, 128, 256]
TRIALS = 8
GENERATIONS = 300
KEEP = 25
MEASURE = 500
RECIPE = ["--stage", "war", *AGAINST, "--hall", "0", "--longest", "100", "--rank", "40", "--tables", "6"]
BILAN = re.compile(
    r"one of --from .* prince ([\d.]+)% · king ([\d.]+)% · crowned ([\d.]+)%"
    r"(?: \(years: p10 (\d+) · median (\d+) · p90 (\d+)\))?.* fell ([\d.]+)% \(mother ([\d.]+)%\)"
    r" · starved (\d+)/seat · nobles ([-+\d.]+)/seat · read ([\d.]+)/seat · fitness ([-\d.]+)"
)
RIVALS = re.compile(r"five of --against .* crowned ([\d.]+)%")


def train(hidden):
    log = HERE / f"h{hidden}.log"
    if log.exists() and "gen  299 " in log.read_text():
        return
    with log.open("w") as f:
        subprocess.run(
            [str(TRAINER), *RECIPE, "--generations", str(GENERATIONS), "--trials", str(TRIALS),
             "--hidden", str(hidden), "--keep", str(KEEP), "--gpu",
             "--out", str(HERE / f"h{hidden}-{{n}}-war.json")],
            stdout=f, stderr=subprocess.STDOUT, check=True,
        )


def measure(hidden, trial, generation, path):
    out = subprocess.run(
        [str(TRAINER), *RECIPE, "--hidden", str(hidden), "--from", str(path), "--measure", str(MEASURE), "--gpu"],
        capture_output=True, text=True, check=True,
    )
    text = out.stdout + out.stderr
    m = BILAN.search(text)
    r = RIVALS.search(text)
    if not m:
        raise RuntimeError(f"no bilan for {path}:\n{text}")
    prince, king, crowned, p10, median, p90, fell, mother, starved, nobles, read, fitness = m.groups()
    return dict(
        hidden=hidden, trial=trial, generation=generation, prince=prince, king=king, crowned=crowned,
        p10=p10 or "", median=median or "", p90=p90 or "", fell=fell, mother=mother, starved=starved,
        nobles=nobles, read=read, fitness=fitness, rivals_crowned=r.group(1) if r else "",
    )


def main():
    HERE.mkdir(exist_ok=True)
    for hidden in WIDTHS:
        train(hidden)
    rows = []
    for hidden in WIDTHS:
        for trial in range(1, TRIALS + 1):
            stem = HERE / f"h{hidden}-{trial}-war"
            snapshots = [(g, Path(f"{stem}.g{g}.json")) for g in range(KEEP, GENERATIONS, KEEP)]
            snapshots.append((GENERATIONS - 1, Path(f"{stem}.json")))
            for generation, path in snapshots:
                if path.exists():
                    rows.append(measure(hidden, trial, generation, path))
    with (HERE / "measures.csv").open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"{len(rows)} measures written")


if __name__ == "__main__":
    main()
