#!/bin/bash
# s10 (septembre 2026) : école d'un trait au palier war, registre du peuple
# compris — la base de s26. Reproduit avec le trainer du dépôt.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo run --release -p empire-train -- --stage war --generations 1100 --tables 6 --hall 8 --rank 40 --out s10-war.json > s10.log 2>&1
echo "SCHOOL 10 DONE" >> s10.log
