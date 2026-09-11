#!/bin/bash
# s38 (septembre 2026) : l'école de l'intendant. Règles de s37 (murs 1 000,
# bélier 1 500 tous les 8 échanges, terres des morts aux barbares), et une
# note qui juge la tenue des livres chaque année : grenier vide (moins d'un
# an de rations) ou débordant (plus de cinq ans), trésor à sec (< 1 000) ou
# dormant (> 50 000), −0,5 le défaut et l'an ; chaque bataille perdue −5 ;
# la chute −100 quelle qu'en soit la cause (la mère à 300 dans s37 avait
# tué toute exploration) ; morts de faim, nobles partis, titres et sacre
# pressé comme avant.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s38-war.json > s38.log 2>&1
echo "SCHOOL 38 DONE" >> s38.log
