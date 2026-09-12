#!/usr/bin/env python3
"""s51 (septembre 2026) : les quatre sœurs de s48 apprennent à tenir contre la pressée.

Quatre écoles reprises de s48a–d, chacune contre les trois autres figées et s50
(`--against` ×4, `--hall 0` : à chaque table, une à cinq chaises de sa population,
les autres tirées au hasard parmi les quatre rivaux ou d'un seul), par manches de
GENERATIONS générations, comme s48 — mais la chaise de s35 est prise par s50, la
pressée : elle marche sur ses deux voisines dès l'an 3 par salves de 25 à 150
hommes et annexe vers l'an 20 qui ne garde personne, exactement ce qu'un joueur
humain fait au set de prod (s48a jouait avec 0 à 7 hommes de garnison). Ce que
les sœurs doivent apprendre : voir venir (l'éclaireur), garder du monde, ou
frapper les premières. Note de s48 (sacre `1200 − 6·an`, `--longest 100`).
Journal des manches dans s51.log, sortie de chaque école dans s51{lettre}.log.
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
    with open("s51.log", "a") as f:
        f.write(msg + "\n")


def last_reading(letter):
    """The last generation's crown rate and median crown year, from the school's log."""
    with open(f"s51{letter}.log") as f:
        readings = [READING.match(line) for line in f]
    m = [r for r in readings if r][-1]
    return int(m.group(1)), float(m.group(2)), m.group(3) or "-"


def main():
    os.chdir(HERE)
    subprocess.run(["cargo", "build", "--release", "-p", "empire-train"], check=True)
    for c in LETTERS:
        if not os.path.exists(f"s51{c}-war.json"):
            note(f"s51{c}: starts from s48{c}")
    for r in range(ROUNDS):
        procs = []
        for c in LETTERS:
            own = f"s51{c}-war.json"
            source = own if os.path.exists(own) else f"s48{c}-war.json"
            sisters = [f"s51{d}-war.json" if os.path.exists(f"s51{d}-war.json") else f"s48{d}-war.json" for d in LETTERS if d != c]
            against = [x for path in (*sisters, "s50-war.json") for x in ("--against", path)]
            cmd = [
                TRAINER, "--stage", "war", "--from", source, *against, "--hall", "0", "--longest", "100",
                "--generations", str(GENERATIONS), "--tables", "6", "--rank", "40", "--out", own,
            ]
            procs.append(subprocess.Popen(
                cmd,
                stdout=open(f"s51{c}.log", "a"),
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
    note("SCHOOL 51 DONE")


if __name__ == "__main__":
    main()
