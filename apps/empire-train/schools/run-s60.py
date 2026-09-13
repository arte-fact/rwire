#!/usr/bin/env python3
"""s60 — the recall against the Chronique, bred (13 September 2026).

The s59 recipe (32 neurons, from scratch against the seven delivered
`.f32`, `--hall 0`, `--longest 100`, `--rank 40`, 8 trials, bred at
0.3) over 100 generations, a snapshot every 10, on the GPU, in two
kinds: `recall`, the default school — not told the Chronique, last
year's orders nor its old reports, 32 outputs of the Extérieur read
again the next year to remember with; and `told` — told everything, no
recall. Every snapshot measured against the prod pool into
`s60/measures.csv`.
"""

import csv
import importlib.util
import subprocess
from pathlib import Path

spec = importlib.util.spec_from_file_location("run_s58", Path(__file__).with_name("run-s58.py"))
s58 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s58)

HERE = Path(__file__).resolve().parent / "s60"
KINDS = {"recall": [], "told": ["--told"]}
GENERATIONS = 100
KEEP = 10


def train(kind, flags):
    log = HERE / f"{kind}.log"
    if log.exists() and f"gen  {GENERATIONS - 1} " in log.read_text():
        return
    with log.open("w") as f:
        subprocess.run(
            [str(s58.TRAINER), *s58.RECIPE, "--generations", str(GENERATIONS), "--trials", str(s58.TRIALS),
             "--hidden", "32", "--keep", str(KEEP), "--breed", "0.3", "--gpu", *flags,
             "--out", str(HERE / f"{kind}-{{n}}-war.json")],
            stdout=f, stderr=subprocess.STDOUT, check=True,
        )


def main():
    HERE.mkdir(exist_ok=True)
    for kind, flags in KINDS.items():
        train(kind, flags)
    rows = []
    for kind in KINDS:
        for trial in range(1, s58.TRIALS + 1):
            stem = HERE / f"{kind}-{trial}-war"
            snapshots = [(g, Path(f"{stem}.g{g}.json")) for g in range(KEEP, GENERATIONS, KEEP)]
            snapshots.append((GENERATIONS - 1, Path(f"{stem}.json")))
            for generation, path in snapshots:
                if path.exists():
                    row = s58.measure(32, trial, generation, path)
                    row["kind"] = kind
                    rows.append(row)
    with (HERE / "measures.csv").open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"{len(rows)} measures written")


if __name__ == "__main__":
    main()
