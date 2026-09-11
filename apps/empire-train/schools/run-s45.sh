#!/bin/bash
# s45 (septembre 2026) : de zéro contre un mélange sans pacifique. La note de
# s35, une population de zéro, et sur chaque chaise libre un rival tiré au
# hasard parmi les meilleurs figés de s35 (garnisons, marche à l'an 5) et de
# s43 (vend sa terre, deux fronts à l'an 3) — --against les deux, --hall 0.
# s44 a montré qu'un tiers de chaises s28 (pacifique) rend le plateau : la
# pression doit être à chaque table ; deux conquérants de styles différents
# la gardent sans donner un spécialiste d'un seul adversaire.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war \
  --against s35-war.json --against s43-war.json --hall 0 \
  --generations 1200 --tables 6 --rank 40 --out s45-war.json > s45.log 2>&1
echo "SCHOOL 45 DONE" >> s45.log
