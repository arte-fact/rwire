#!/bin/bash
# s41 (septembre 2026) : le contrôle. La note de s35 telle quelle (sacre
# pressé 1000 − 4·an, Prince 150 − an, Roi 300 − 2·an, peuple et cour par
# tête, rien d'autre), de zéro, sous les règles d'aujourd'hui — murs 1 000,
# bélier 1 500 tous les 8 échanges, et la règle des terres (un royaume qui
# tombe autrement que par conquête rend ses arpents aux barbares), que s35
# n'a pas connue à l'école. s37 à s40, toutes de zéro sous cette règle, ont
# plafonné sans titres : celle-ci dit si c'est la règle ou leurs notes.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s41-war.json > s41.log 2>&1
echo "SCHOOL 41 DONE" >> s41.log
