#!/usr/bin/env python3
"""s49 (septembre 2026) : de zéro contre le set de prod et s35, pour un second set de sœurs.

Une population de zéro, 300 générations, la note d'aujourd'hui (sacre `1200 − 6·an`,
arena.rs) ; sur les chaises libres, un rival tiré au hasard par chaise parmi les
meilleurs figés du set de prod (s46a, s48b, s48c, s48d — les quatre tempéraments
livrés) et s35 (`--against` ×5, `--hall 0`). Sous la règle des terres, la lignée des
titres gagne sa course contre la survie une fois sur dix environ (s46, contre s35
seul : 4 valides sur 36 essais) : on relance des essais, six à la fois à deux
threads, jusqu'à en avoir quatre (s49a … s49d). Un essai est perdu dès que la gén.
80 se passe sans sacre ; il est valide s'il finit avec au moins 5 % de sacres.
Journal des essais dans s49.log.
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
PROD = ("s46a", "s48b", "s48c", "s48d", "s35")
AGAINST = [x for p in PROD for x in ("--against", f"{p}-war.json")]
CROWNED = re.compile(r"^gen +(\d+) .* crowned +([\d.]+)%")


def crowned_at(log, gen):
    """The crown rate printed for `gen`, or None while the log is not there yet."""
    with open(log) as f:
        for line in f:
            m = CROWNED.match(line)
            if m and int(m.group(1)) == gen:
                return float(m.group(2))
    return None


def start(n):
    log = f"s49-try{n}.log"
    out = f"s49-try{n}-war.json"
    proc = subprocess.Popen(
        [
            TRAINER, "--stage", "war", *AGAINST, "--hall", "0",
            "--generations", str(GENERATIONS), "--tables", "6", "--rank", "40", "--out", out,
        ],
        stdout=open(log, "w"),
        stderr=subprocess.STDOUT,
        env={**os.environ, "RAYON_NUM_THREADS": "2"},
    )
    return {"n": n, "proc": proc, "log": log, "out": out, "judged": False}


def note(msg):
    with open("s49.log", "a") as f:
        f.write(msg + "\n")


def discard(t, why):
    t["proc"].kill()
    t["proc"].wait()
    note(f"try {t['n']}: {why}")
    for f in (t["log"], t["out"], t["out"].replace(".json", ".best.json")):
        if os.path.exists(f):
            os.remove(f)


def keep(t, letter):
    for src, dst in (
        (t["log"], f"s49{letter}.log"),
        (t["out"], f"s49{letter}-war.json"),
        (t["out"].replace(".json", ".best.json"), f"s49{letter}-war.best.json"),
    ):
        shutil.move(src, dst)
    with open(f"s49{letter}.log", "a") as f:
        f.write(f"SCHOOL 49{letter} DONE\n")


def main():
    os.chdir(HERE)
    subprocess.run(["cargo", "build", "--release", "-p", "empire-train"], check=True)
    valid = [c for c in LETTERS if os.path.exists(f"s49{c}-war.json")]
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
                c = crowned_at(t["log"], VERDICT_GEN)
                if c is not None:
                    t["judged"] = True
                    if c < 1.0:
                        discard(t, f"lost the race (gen {VERDICT_GEN}: {c}% crowned)")
                        running.remove(t)
                        continue
                    note(f"try {t['n']}: in the race (gen {VERDICT_GEN}: {c}% crowned)")
            if t["proc"].poll() is None:
                continue
            running.remove(t)
            c = crowned_at(t["log"], GENERATIONS - 1)
            if c is None or c < 5.0:
                discard(t, f"finished below 5% ({c}% crowned)")
                continue
            letter = todo.pop(0)
            keep(t, letter)
            note(f"try {t['n']}: VALID → s49{letter} ({c}% crowned)")
    for t in running:
        discard(t, "not needed any more")
    note("SCHOOL 49 DONE")


if __name__ == "__main__":
    main()
