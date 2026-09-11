#!/bin/bash
# s34 (septembre 2026) : bélier à 8 coups (comme s32, où le mur ne valait rien)
# mais à 2 000 l'unité au lieu de 500 — un bélier abat jusqu'à cinq dixièmes
# (5 000 or de murs) par bataille, il doit coûter quelque chose.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s34-war.json > s34.log 2>&1
echo "SCHOOL 34 DONE" >> s34.log
