#!/usr/bin/env python3
"""s54 (septembre 2026) : de zéro contre tout le jeu de prod, jugée seule à table.

La recette de s53 (essais de zéro, `--hall 0`, `--longest 100`, gardés à ≥ 1 % de
sacres à la gén. VERDICT_GEN), les cinq autres chaises tirées au hasard parmi les
**sept écoles livrées** — quatre fortifieuses (s46a, s48b–d), deux rusheuses
(s53-finalist18, s53-finalist9) et une prudente (s53a) — pour faire naître un
profil qui réponde à chacun.

Le verdict final n'est plus lu aux tables d'école (où une rusheuse tombe sous
ses propres clones : finalist18 y faisait 63 % de chutes, et 7 % seule contre
cinq s48) mais sur un **bilan** : l'école seule sur une chaise, les cinq autres
tirées parmi les sept, 200 tables de 150 ans (`--measure 200`). On garde si
sacres ≥ CROWNED %, année médiane de sacre ≤ MEDIAN, chutes ≤ FELL %. Les
finalistes recalés sont gardés (`s54-finalist{n}`), les perdants effacés.
Journal dans s54.log, bilan de chaque finaliste à la fin de son s54-…log.
"""

import os
import re
import shutil
import subprocess
import time

HERE = os.path.dirname(os.path.abspath(__file__))
TRAINER = os.path.join(HERE, "../../../target/release/empire-train")
LETTERS = "abcd"
PARALLEL = 6
GENERATIONS = 300
VERDICT_GEN = 80
CROWNED = 30.0
MEDIAN = 60
FELL = 50.0
PROD = ("s46a", "s48b", "s48c", "s48d", "s53-finalist18", "s53-finalist9", "s53a")
AGAINST = [x for p in PROD for x in ("--against", f"{p}-war.json")]
READING = re.compile(r"^gen +(\d+) .* fell +([\d.]+)% .* crowned +([\d.]+)%(?: \(median year (\d+)\))?")
BILAN = re.compile(r"one of --from .* crowned ([\d.]+)%(?: \(years: p10 \d+ · median (\d+))?.* fell ([\d.]+)%")


def reading_at(log, gen):
    """(crowned %, median crown year, fell %) printed for `gen`, or None while
    the log is not there yet. No crown yet reads as a median of 999."""
    with open(log) as f:
        for line in f:
            m = READING.match(line)
            if m and int(m.group(1)) == gen:
                return float(m.group(3)), int(m.group(4) or 999), float(m.group(2))
    return None


def bilan(t):
    """The school alone at a table of five prod brains, over 200 tables of 150
    years: the same three figures, appended to the trial's log."""
    out = subprocess.run(
        [TRAINER, "--stage", "war", "--from", t["out"], *AGAINST, "--hall", "0", "--longest", "150", "--measure", "200"],
        capture_output=True,
        text=True,
        env={**os.environ, "RAYON_NUM_THREADS": "2"},
    )
    with open(t["log"], "a") as f:
        f.write("bilan, alone against five of prod:\n" + out.stdout + out.stderr)
    for line in (out.stdout + out.stderr).splitlines():
        m = BILAN.search(line)
        if m:
            return float(m.group(1)), int(m.group(2) or 999), float(m.group(3))
    return None


def told(r):
    return f"{r[0]}% crowned, median year {r[1]}, fell {r[2]}%"


def start(n):
    log = f"s54-try{n}.log"
    out = f"s54-try{n}-war.json"
    proc = subprocess.Popen(
        [
            TRAINER, "--stage", "war", *AGAINST, "--hall", "0", "--longest", "100",
            "--generations", str(GENERATIONS), "--tables", "6", "--rank", "40", "--out", out,
        ],
        stdout=open(log, "w"),
        stderr=subprocess.STDOUT,
        env={**os.environ, "RAYON_NUM_THREADS": "2"},
    )
    return {"n": n, "proc": proc, "log": log, "out": out, "judged": False}


def note(msg):
    with open("s54.log", "a") as f:
        f.write(msg + "\n")


def discard(t, why):
    t["proc"].kill()
    t["proc"].wait()
    note(f"try {t['n']}: {why}")
    for f in (t["log"], t["out"], t["out"].replace(".json", ".best.json")):
        if os.path.exists(f):
            os.remove(f)


def keep(t, name):
    """Files the trial away as `s54{name}` (a letter for a valid school, `-finalist{n}` for
    one that entered the race but finished off the mark)."""
    for src, dst in (
        (t["log"], f"s54{name}.log"),
        (t["out"], f"s54{name}-war.json"),
        (t["out"].replace(".json", ".best.json"), f"s54{name}-war.best.json"),
    ):
        shutil.move(src, dst)
    with open(f"s54{name}.log", "a") as f:
        f.write(f"SCHOOL 54{name} DONE\n")


def main():
    os.chdir(HERE)
    subprocess.run(["cargo", "build", "--release", "-p", "empire-train"], check=True)
    valid = [c for c in LETTERS if os.path.exists(f"s54{c}-war.json")]
    todo = [c for c in LETTERS if c not in valid]
    note(f"kept: {' '.join(valid) or '-'} · to find: {' '.join(todo)}")
    running = []
    n = 0
    while todo:
        while len(running) < PARALLEL:
            n += 1
            running.append(start(n))
            note(f"try {n}: started")
        time.sleep(20)
        for t in list(running):
            if not t["judged"]:
                r = reading_at(t["log"], VERDICT_GEN)
                if r is not None:
                    t["judged"] = True
                    if r[0] < 1.0:
                        discard(t, f"lost the race (gen {VERDICT_GEN}: {told(r)})")
                        running.remove(t)
                        continue
                    note(f"try {t['n']}: in the race (gen {VERDICT_GEN}: {told(r)})")
            if t["proc"].poll() is None:
                continue
            running.remove(t)
            r = reading_at(t["log"], GENERATIONS - 1)
            if r is None:
                discard(t, "finished with no reading")
                continue
            note(f"try {t['n']}: finished (school tables: {told(r)}), measuring")
            b = bilan(t)
            if b is None:
                discard(t, "no bilan")
                continue
            if b[0] < CROWNED or b[1] > MEDIAN or b[2] > FELL:
                keep(t, f"-finalist{t['n']}")
                note(f"try {t['n']}: off the mark (bilan: {told(b)}), kept as s54-finalist{t['n']}")
                continue
            letter = todo.pop(0)
            keep(t, letter)
            note(f"try {t['n']}: VALID → s54{letter} (bilan: {told(b)})")
    for t in running:
        discard(t, "not needed any more")
    note("SCHOOL 54 DONE")


if __name__ == "__main__":
    main()
