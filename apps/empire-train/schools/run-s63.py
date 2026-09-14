#!/usr/bin/env python3
"""s63 — the rules' version 3 (14 September 2026): the journal is gone from
the sight (s62c: it cost every reading three to six points), the genome
halves (15 781 weights at 32 neurons), and the brains to deliver read the
Chronique and their recall (`--told --recall`, the one reading that beat the
control at 600 generations).

Stage A, the pool: 8 trials from nothing, among themselves, bred at 0.3,
told + recall, 100 generations; their final schools, parents dropped, are
frozen as `pools/B` (the pool of reference under version 3).

Stage B, the temperaments: 16 trials from nothing against pool B, same
recipe, 400 generations (the plateau of s62c), a snapshot every 50. Every
trial's final school is measured against the pool (500 tables) into
`s63/measures.csv`; the seven to deliver are chosen by hand from these
(`--show` tells their temperament).

Stage C: `promote.py` tries the sixteen against pool B and writes `pools/C`
with the one champion that earns its chair.
"""

import csv
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TRAINER = ROOT / "target/release/empire-train"
SCHOOLS = Path(__file__).resolve().parent
HERE = SCHOOLS / "s63"
POOL = SCHOOLS / "pools" / "B"
MEASURE = 500
BASE = ["--stage", "war", "--hall", "0", "--longest", "100", "--rank", "40", "--tables", "32",
        "--hidden", "32", "--breed", "0.3", "--told", "--recall", "--gpu"]
BILAN = re.compile(
    r"one of --from .* prince ([\d.]+)% · king ([\d.]+)% · crowned ([\d.]+)%"
    r"(?: \(years: p10 (\d+) · median (\d+) · p90 (\d+)\))?.* fell ([\d.]+)% \(mother ([\d.]+)%\)"
    r" · starved (\d+)/seat · nobles ([-+\d.]+)/seat · read ([\d.]+)/seat · fitness ([-\d.]+)"
)
RIVALS = re.compile(r"five of --against .* crowned ([\d.]+)%")
CROWNED = re.compile(r"^try +(\d+) · gen +(\d+) .*crowned +([\d.]+)%")


def train(name, trials, generations, keep, against):
    log = HERE / f"{name}.log"
    if log.exists() and f"gen {generations - 1:4} " in log.read_text():
        return log
    with log.open("w") as f:
        subprocess.run(
            [str(TRAINER), *BASE, *against, "--trials", str(trials), "--generations", str(generations),
             "--keep", str(keep), "--out", str(HERE / f"{name}-{{n}}-war.json")],
            stdout=f, stderr=subprocess.STDOUT, check=True,
        )
    return log


def against(pool):
    return [x for p in sorted(pool.glob("pool-*-war.json")) for x in ("--against", str(p))]


def freeze_pool(name, trials):
    POOL.mkdir(parents=True, exist_ok=True)
    for n in range(1, trials + 1):
        school = json.loads((HERE / f"{name}-{n}-war.json").read_text())
        school.pop("parents", None)
        (POOL / f"pool-{n}-war.json").write_text(json.dumps(school))
    (POOL / "pool.json").write_text(json.dumps({
        "from": f"s63 stage A: {trials} trials from nothing among themselves, bred at 0.3, told + recall, "
                "100 generations, rules 3 (2026-09-14)",
        "members": [f"s63/{name}-{n}-war.json" for n in range(1, trials + 1)],
    }, indent=2) + "\n")


def measure(path):
    out = subprocess.run(
        [str(TRAINER), "--stage", "war", *against(POOL), "--longest", "100", "--rank", "40", "--hidden", "32",
         "--from", str(path), "--measure", str(MEASURE), "--gpu"],
        capture_output=True, text=True, check=True,
    )
    text = out.stdout + out.stderr
    m = BILAN.search(text)
    if not m:
        raise RuntimeError(f"no bilan for {path}:\n{text}")
    r = RIVALS.search(text)
    return list(m.groups()) + [r.group(1) if r else ""]


def main():
    HERE.mkdir(exist_ok=True)
    log = train("poolB", 8, 100, 25, [])
    last = [m for line in log.read_text().splitlines() for m in [CROWNED.match(line)] if m and m.group(2) == "99"]
    print(f"stage A: {sum(1 for m in last if float(m.group(3)) > 0)} of 8 trials crown among themselves", flush=True)
    freeze_pool("poolB", 8)
    print("pool B frozen", flush=True)
    train("t", 16, 400, 50, against(POOL))
    print("stage B trained", flush=True)
    fields = ["trial", "generation", "prince", "king", "crowned", "p10", "median", "p90", "fell", "mother",
              "starved", "nobles", "read", "fitness", "rivals_crowned"]
    rows = []
    for n in range(1, 17):
        for generation, path in [(200, HERE / f"t-{n}-war.g200.json"), (300, HERE / f"t-{n}-war.g300.json"),
                                 (399, HERE / f"t-{n}-war.json")]:
            if path.exists():
                rows.append([n, generation, *measure(path)])
    with (HERE / "measures.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(fields)
        w.writerows(rows)
    print(f"{len(rows)} measures written", flush=True)
    subprocess.run(
        ["python3", str(SCHOOLS / "promote.py"), str(POOL), str(SCHOOLS / "pools" / "C"),
         *[str(HERE / f"t-{n}-war.json") for n in range(1, 17)]],
        check=True,
    )
    print("campaign done", flush=True)


if __name__ == "__main__":
    main()
