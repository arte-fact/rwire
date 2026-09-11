#!/bin/bash
# s39 (septembre 2026) : l'intendant sans le gendarme. Règles de s37/s38
# (murs 1 000, bélier 1 500 tous les 8 échanges, terres des morts aux
# barbares) ; la note juge la tenue des livres chaque année (grenier vide
# ou débordant, trésor à sec ou dormant, −0,5 le défaut et l'an), ne fait
# plus rien payer à la défaite ni à la chute (s37 et s38 : plateau
# pacifique), et récompense un peu l'usage des outils : +0,5 l'an où le
# siège a acheté ou vendu au marché, +0,2 le rapport lu (éclaireur, agent).
# Morts de faim, nobles partis, titres et sacre pressé comme avant. De zéro.
set -e
cd "$(dirname "$0")"
export RAYON_NUM_THREADS=10
cargo build --release -p empire-train
../../../target/release/empire-train --stage war --generations 1200 --tables 6 --hall 8 --rank 40 --out s39-war.json > s39.log 2>&1
echo "SCHOOL 39 DONE" >> s39.log
