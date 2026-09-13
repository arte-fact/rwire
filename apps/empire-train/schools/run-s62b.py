#!/usr/bin/env python3
"""s62b — s62's readings taken to 300 generations (13 September 2026):
each reading named on the command line (all eight by default) goes on
from its 8 schools of s62 for 200 more generations against pool A, a
snapshot every 10, then every new snapshot is measured against the pool
into `s62/measures-300-<readings>.csv`."""

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
KINDS = {name: flags for name, flags in s62.kinds()}
if len(sys.argv) > 1:
    KINDS = {name: KINDS[name] for name in sys.argv[1:]}
AGAINST = [x for n in range(1, 9) for x in ("--against", str(HERE / f"poolA-{n}-war.json"))]


def main():
    for name, flags in KINDS.items():
        log = HERE / f"{name}-300.log"
        if log.exists() and "gen  299 " in log.read_text():
            continue
        with log.open("w") as f:
            subprocess.run(
                [str(s58.TRAINER), *s62.BASE, *AGAINST, "--generations", "200", "--keep", "10", *flags,
                 "--from", str(HERE / f"{name}-{{n}}-war.json"), "--out", str(HERE / f"{name}-{{n}}-war.json")],
                stdout=f, stderr=subprocess.STDOUT, check=True,
            )
    rows = []
    for name in KINDS:
        for trial in range(1, 9):
            stem = HERE / f"{name}-{trial}-war"
            snapshots = [(g, Path(f"{stem}.g{g}.json")) for g in range(110, 300, 10)]
            snapshots.append((299, Path(f"{stem}.json")))
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
    tag = "-".join(KINDS) if len(sys.argv) > 1 else "all"
    with (HERE / f"measures-300-{tag}.csv").open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"{len(rows)} measures written", flush=True)


if __name__ == "__main__":
    main()
