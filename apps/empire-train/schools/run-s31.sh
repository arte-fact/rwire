#!/bin/bash
# s31 (septembre 2026) : la défense rééquilibrée — fortification à 1 000 le
# dixième (au lieu de 5 000), bélier à 500 (au lieu de 2 500) qui ne frappe
# plus que tous les 16 échanges (au lieu de 4). À partir de zéro, 3 500
# générations au palier war d'un trait, l'âge de s28, pour comparer.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
# Resumes where it stands: gen 298 at the pause for the speed pass
# (dot on lanes, allocation-free titles, quiet marches — ~2.5x).
../../../target/release/empire-train --stage war --from s31-war.json --generations 3202 --tables 6 --hall 8 --rank 40 --out s31-war.json >> s31.log 2>&1
echo "SCHOOL 31 DONE" >> s31.log
