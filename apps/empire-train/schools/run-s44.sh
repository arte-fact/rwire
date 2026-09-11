#!/bin/bash
# s44 (septembre 2026) : de zéro contre un mélange. La note de s35, une
# population de zéro, et sur chaque chaise libre un rival tiré au hasard
# parmi les meilleurs figés de s35 (le conquérant), s28 (le cerveau livré,
# qui thésaurise et garde ses hommes) et s43 (le spécialiste de s35) —
# --against les trois, --hall 0. s43 a montré qu'un seul rival figé fait
# renaître les titres mais donne un spécialiste (8 % contre s28) ; ici la
# table change à chaque partie.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war \
  --against s35-war.json --against s28-war.json --against s43-war.json --hall 0 \
  --generations 1200 --tables 6 --rank 40 --out s44-war.json > s44.log 2>&1
echo "SCHOOL 44 DONE" >> s44.log
