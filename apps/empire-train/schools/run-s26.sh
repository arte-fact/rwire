#!/bin/bash
# s26 (septembre 2026) : s10 scolarisé 1 200 générations de plus, sans
# lettres — le brain livré (brains/schooled.f32 = s26-war.best.json).
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo run --release -p empire-train -- --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --from s10-war.json --out s26-war.json > s26.log 2>&1
echo "SCHOOL 26 DONE" >> s26.log
