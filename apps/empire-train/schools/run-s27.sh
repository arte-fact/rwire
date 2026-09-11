#!/bin/bash
# s27 (septembre 2026) : s26 grandi aux nouvelles règles — murs, hospice,
# béliers, renseignement immédiat lu en deux temps — et scolarisé 1 200
# générations de plus au palier war, sans lettres.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --from s26-war.json --out s27-war.json > s27.log 2>&1
echo "SCHOOL 27 DONE" >> s27.log
