#!/usr/bin/env python3
"""s53 (septembre 2026) : de zéro contre les quatre s48, jugée sur trois chiffres.

La recette de s52 (essais de zéro, `--hall 0`, les quatre sœurs de s48 tirées au
hasard par chaise, gardés à ≥ 1 % de sacres à la gén. VERDICT_GEN), mais le
verdict final regarde les trois chiffres à la fois : **sacres ≥ CROWNED %,
année médiane de sacre ≤ MEDIAN, chutes ≤ FELL %**. s49a et s52a/b sacraient
tôt (an 52) en tombant 80 % du temps : des kamikazes, inutilisables en jeu ;
s49b–d tombaient moins mais sacraient à l'an 134. On veut les trois.
On garde les VALID premières écoles valides (s53a…), le journal est dans s53.log.

Deuxième lancement (après ~70 essais à 150 ans) : tables à `--longest 100`, comme
s48 — les quatre finalistes du premier (28 / 17 / 17 / 23 % de sacres, médiane an
35–56) n'ont été recalés que par les chutes (72–83 %), qui à 150 ans comptent
aussi celles d'après l'an 100 ; et les finalistes recalés sont désormais gardés
(`s53-finalist{n}-war.json`) au lieu d'être effacés, pour pouvoir les mesurer.
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
CROWNED = 5.0
MEDIAN = 100
FELL = 50.0
PROD = ("s48a", "s48b", "s48c", "s48d")
AGAINST = [x for p in PROD for x in ("--against", f"{p}-war.json")]
READING = re.compile(r"^gen +(\d+) .* fell +([\d.]+)% .* crowned +([\d.]+)%(?: \(median year (\d+)\))?")


def reading_at(log, gen):
    """(crowned %, median crown year, fell %) printed for `gen`, or None while
    the log is not there yet. No crown yet reads as a median of 999."""
    with open(log) as f:
        for line in f:
            m = READING.match(line)
            if m and int(m.group(1)) == gen:
                return float(m.group(3)), int(m.group(4) or 999), float(m.group(2))
    return None


def told(r):
    return f"{r[0]}% crowned, median year {r[1]}, fell {r[2]}%"


def start(n):
    log = f"s53-try{n}.log"
    out = f"s53-try{n}-war.json"
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
    with open("s53.log", "a") as f:
        f.write(msg + "\n")


def discard(t, why):
    t["proc"].kill()
    t["proc"].wait()
    note(f"try {t['n']}: {why}")
    for f in (t["log"], t["out"], t["out"].replace(".json", ".best.json")):
        if os.path.exists(f):
            os.remove(f)


def keep(t, name):
    """Files the trial away as `s53{name}` (a letter for a valid school, `-finalist{n}` for
    one that entered the race but finished off the mark)."""
    for src, dst in (
        (t["log"], f"s53{name}.log"),
        (t["out"], f"s53{name}-war.json"),
        (t["out"].replace(".json", ".best.json"), f"s53{name}-war.best.json"),
    ):
        shutil.move(src, dst)
    with open(f"s53{name}.log", "a") as f:
        f.write(f"SCHOOL 53{name} DONE\n")


def main():
    os.chdir(HERE)
    subprocess.run(["cargo", "build", "--release", "-p", "empire-train"], check=True)
    valid = [c for c in LETTERS if os.path.exists(f"s53{c}-war.json")]
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
            if r[0] < CROWNED or r[1] > MEDIAN or r[2] > FELL:
                keep(t, f"-finalist{t['n']}")
                note(f"try {t['n']}: finished off the mark ({told(r)}), kept as s53-finalist{t['n']}")
                continue
            letter = todo.pop(0)
            keep(t, letter)
            note(f"try {t['n']}: VALID → s53{letter} ({told(r)})")
    for t in running:
        discard(t, "not needed any more")
    note("SCHOOL 53 DONE")


if __name__ == "__main__":
    main()
