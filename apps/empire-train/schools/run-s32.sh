#!/bin/bash
# s32 (septembre 2026) : mêmes règles que s31 (murs 1 000 le dixième, béliers 500)
# mais le bélier frappe tous les 8 échanges au lieu de 16 — s31 avait gelé le jeu
# en sièges stériles (murs à 10/10 partout dès l'an 10, 8 % de couronnés vers
# l'an 140). École courte, à partir de zéro : la tendance se lit dès 500 gens.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s32-war.json > s32.log 2>&1
echo "SCHOOL 32 DONE" >> s32.log
