#!/bin/bash
# s43 (septembre 2026) : s35 de zéro contre s35. La note de s35 telle quelle
# (sacre pressé, Prince, Roi, peuple et cour par tête, rien d'autre), une
# population de zéro, et sur les chaises libres de chaque table le meilleur
# de s35 figé (--against, --hall 0) : un conquérant en face dès la première
# génération, au lieu de clones qui apprennent ensemble. Sous la règle des
# terres, sous laquelle s37 à s41 (de zéro, entre elles) ont plafonné.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --against s35-war.json --hall 0 \
  --generations 1200 --tables 32 --rank 40 --out s43-war.json > s43.log 2>&1
echo "SCHOOL 43 DONE" >> s43.log
