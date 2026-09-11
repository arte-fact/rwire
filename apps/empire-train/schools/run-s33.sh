#!/bin/bash
# s33 (septembre 2026) : le bélier frappe tous les 12 échanges — entre s31 (16 :
# murs absolus, jeu gelé, 8 % de couronnés vers l'an 140) et s32 (8 : murs
# inutiles, agression pure, 20 % de couronnés vers l'an 60, 80 % de chutes).
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s33-war.json > s33.log 2>&1
echo "SCHOOL 33 DONE" >> s33.log
