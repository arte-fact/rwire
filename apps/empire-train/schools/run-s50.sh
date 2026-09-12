#!/usr/bin/env bash
# s50 (septembre 2026) : la pressée. Reprise de s49a (sacre médian an 52, tables
# de 28 ans) contre les quatre s48 tirées au hasard par chaise (`--hall 0`),
# tables de 100 ans, et les murs payés : `--walls 30` coûte 30 points par
# dixième de mur gardé en moyenne sur la partie (murs pleins = −300, le prix
# d'un sacre). Idée de l'utilisateur : une IA qui joue vite et frappe tôt
# plutôt que de se murer — le set de prod se fait rouler par un humain qui
# envoie 300 hommes l'an 15 devant 20 gardes murés.
set -e
cd "$(dirname "$0")"
T=../../../target/release/empire-train
AGAINST=()
for s in s48a s48b s48c s48d; do AGAINST+=(--against "$s-war.json"); done
exec "$T" --stage war --from s49a-war.json "${AGAINST[@]}" --hall 0 --longest 100 \
  --walls 30 --generations 300 --tables 6 --rank 40 --out s50-war.json
