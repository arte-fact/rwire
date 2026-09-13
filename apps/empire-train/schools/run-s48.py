#!/usr/bin/env python3
"""s48 (septembre 2026) : les quatre sœurs de s46 s'entraînent les unes contre les
autres, aux tables courtes, avec s35 sur une chaise.

Quatre écoles reprises de s46a–d, chacune contre les trois autres figées et s35
(`--against` ×4, `--hall 0` : à chaque table, une à cinq chaises de sa population,
les autres tirées au hasard parmi les quatre rivaux ou d'un seul), par manches de
GENERATIONS générations : à chaque manche, chaque école repart de son propre
fichier et lit les sœurs telles que la manche d'avant les a laissées. Quatre
entraîneurs à la fois, trois threads chacun. La note est celle de s35 avec le
sacre à `1200 − 6·an` (arena.rs), et les tables s'arrêtent à `--longest 100` :
survivre ne rapporte plus rien après l'an 100, il faut sacrer avant. (s47, la même
chose à 150 ans sans s35, a été arrêtée à la manche 5 : deux sœurs sur quatre
glissaient vers le plateau pacifique.) Journal des manches dans s48.log, sortie de
chaque école dans s48{lettre}.log.
"""

import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
TRAINER = os.path.join(HERE, "../../../target/release/empire-train")
LETTERS = "abcd"
ROUNDS = 40
GENERATIONS = 25
READING = re.compile(r"^gen +(\d+) .* crowned +([\d.]+)%(?: \(median year (\d+)\))?")


def note(msg):
    with open("s48.log", "a") as f:
        f.write(msg + "\n")


def last_reading(letter):
    """The last generation's crown rate and median crown year, from the school's log."""
    with open(f"s48{letter}.log") as f:
        readings = [READING.match(line) for line in f]
    m = [r for r in readings if r][-1]
    return int(m.group(1)), float(m.group(2)), m.group(3) or "-"


def main():
    os.chdir(HERE)
    subprocess.run(["cargo", "build", "--release", "-p", "empire-train"], check=True)
    for c in LETTERS:
        if not os.path.exists(f"s48{c}-war.json"):
            note(f"s48{c}: starts from s46{c}")
    for r in range(ROUNDS):
        procs = []
        for c in LETTERS:
            own = f"s48{c}-war.json"
            source = own if os.path.exists(own) else f"s46{c}-war.json"
            sisters = [f"s48{d}-war.json" if os.path.exists(f"s48{d}-war.json") else f"s46{d}-war.json" for d in LETTERS if d != c]
            against = [x for path in (*sisters, "s35-war.json") for x in ("--against", path)]
            cmd = [
                TRAINER, "--stage", "war", "--from", source, *against, "--hall", "0", "--longest", "100",
                "--generations", str(GENERATIONS), "--tables", "32", "--rank", "40", "--out", own,
            ]
            procs.append(subprocess.Popen(
                cmd,
                stdout=open(f"s48{c}.log", "a"),
                stderr=subprocess.STDOUT,
                env={**os.environ, "RAYON_NUM_THREADS": "3"},
            ))
        for p in procs:
            p.wait()
        if any(p.returncode != 0 for p in procs):
            note(f"round {r + 1}: a trainer failed, stopping")
            return
        note(f"round {r + 1}: " + " · ".join(
            f"{c} gen {g} crowned {crowned}% (median {year})" for c in LETTERS for g, crowned, year in [last_reading(c)]
        ))
    note("SCHOOL 47 DONE")


if __name__ == "__main__":
    main()
