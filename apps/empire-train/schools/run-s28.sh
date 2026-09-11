#!/bin/bash
# s28 (septembre 2026) : école à partir de zéro sous les nouvelles règles —
# murs, hospice, béliers, renseignement immédiat — 3 500 générations au
# palier war d'un trait, l'âge de s27 (s10 + s26 + s27), pour comparer.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 3500 --tables 6 --hall 8 --rank 40 --out s28-war.json > s28.log 2>&1
echo "SCHOOL 28 DONE" >> s28.log
