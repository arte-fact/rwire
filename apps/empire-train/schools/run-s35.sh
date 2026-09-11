#!/bin/bash
# s35 (septembre 2026) : règles de s34 (murs 1 000, bélier 8 coups) avec le
# bélier à 1 500 au lieu de 2 000, et une note qui presse : sacre `1000 − 4·an`
# (au lieu de `1000 − an`), Prince `150 − an`, Roi `300 − 2·an`. s34 sacrait
# 24,6 % des sièges mais à l'an 136 ; la cible est un sacre avant l'an 50.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s35-war.json > s35.log 2>&1
echo "SCHOOL 35 DONE" >> s35.log
