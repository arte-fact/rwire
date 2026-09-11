#!/bin/bash
# s36 (septembre 2026), par curiosité : la couronne allégée — l'Empereur aux
# chiffres de l'ancien Roi (2 600 serfs, 25 nobles, 14 marchés, 6 palais…), le
# Roi à ceux de l'ancien Prince, un Prince plus léger. Règles et note de s35
# par ailleurs. Cible : sacre sous quarante ans.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s36-war.json > s36.log 2>&1
echo "SCHOOL 36 DONE" >> s36.log
