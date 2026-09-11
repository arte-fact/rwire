#!/bin/bash
# s37 (septembre 2026) : règles de s35 (murs 1 000, bélier 1 500 tous les 8
# échanges, note pressée) plus deux choses : un royaume dont le souverain
# meurt rend ses terres aux barbares (elles disparaissaient de la carte), et
# la note retire 300 points à la chute sous le couteau d'une mère affamée.
# Titres originaux (l'essai s36 aux titres allégés n'est pas retenu).
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s37-war.json > s37.log 2>&1
echo "SCHOOL 37 DONE" >> s37.log
