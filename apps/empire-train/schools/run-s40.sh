#!/bin/bash
# s40 (septembre 2026) : la note de s35 (sacre pressé, Prince et Roi au
# premier an, peuple et cour par tête, rien sur la défaite ni la chute),
# sous les règles de s37+ (murs 1 000, bélier 1 500 tous les 8 échanges,
# terres des morts aux barbares), et un léger coup de pouce aux outils de
# guerre, +0,2 l'an chacun : murs pleins (au prorata des dixièmes), béliers
# menés au siège d'un royaume, rapport lu (éclaireur ou agent). Par an et
# jamais par pièce — s39 payait chaque rapport et fermait des lettres. De
# zéro, à comparer à s35 à âge égal.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s40-war.json > s40.log 2>&1
echo "SCHOOL 40 DONE" >> s40.log
