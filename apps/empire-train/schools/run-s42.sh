#!/bin/bash
# s42 (septembre 2026) : l'école du grenier. La note de s35 (sacre pressé,
# Prince, Roi, peuple et cour par tête, rien d'autre) avec la naissance ×5
# (0,01) et le noble ×5 (±1,5) — l'un et l'autre viennent du grain donné :
# une ration pleine fait naître, une ration large attire les colons et,
# parmi eux, les nobles. Reprise de s35 (--from) et jouée contre les
# meilleurs de s35, s34 et s33 (--against, --hall 0) : des rivaux
# conquérants qui le restent pendant que la population cherche, face à eux,
# une autre façon de tenir. Sous la règle des terres. Les générations sont
# numérotées à la suite de s35 (1 200 → 2 399).
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --from s35-war.json \
  --against s35-war.json --against s34-war.json --against s33-war.json --hall 0 \
  --generations 1200 --tables 32 --rank 40 --out s42-war.json > s42.log 2>&1
echo "SCHOOL 42 DONE" >> s42.log
