# Le Brain — point d'arrêt (septembre 2026)

État : **livré** (`brains/schooled.f32`, école s28 — de zéro, sous les murs, l'hospice, les
béliers et le renseignement immédiat de `BATIMENTS.md` / `RENSEIGNEMENT.md`). Ce document fige ce que les
ordinateurs d'Empire savent faire, comment ils l'ont appris, ce qui a été essayé et ce qui
reste ouvert — pour pouvoir reprendre l'entraînement plus tard sans rien redécouvrir.
Code : `src/brain.rs` (le joueur), `src/arena.rs` (la table et la note),
`apps/empire-train` (l'école).

## 1. Ce qu'est un ordinateur

Chaque siège tenu par l'ordinateur est un `Brain` : **deux réseaux denses** à une couche
cachée de 32 neurones (`tanh` dedans, sigmoïdes en sortie), lus une fois par an chacun.

| Réseau | Entrées | Sorties | Rôle |
|---|---|---|---|
| `intendance` | `SIGHT` = 169 | `A_OUT` = 18 | rations, taxes, marché du grain, vente de terre, achats |
| `exterieur` | `SIGHT` + 18 | `B_OUT` = 19 | éclaireur, agents, puis expéditions (5 voisins + barbares) et béliers |

Génome : `Brain::GENOME` = 12 677 poids (les deux réseaux bout à bout, floats
little-endian dans `schooled.f32`). Il n'y a **aucune règle codée à la main** dans le jeu
de l'ordinateur en dehors du décodage ci-dessous : tout ce qu'il fait sort des réseaux.

### La vue (`sight`, 169 entrées)

Les parts sont lues telles quelles, les comptes en **échelle log** (`count(n, scale)` :
`scale` lit 0,7, dix fois plus 2,4, cent fois 4,6 — un royaume dix fois plus grand qu'à
l'école reste dans la plage).

- **Son royaume (43)** : année, météo, arpents, serfs, nobles, marchands, soldats,
  efficacité, ration des soldats, trésor, stocks, récolte, rats, marchés, moulins,
  fonderies, chantiers, palais, les trois taxes, titre, arpents par serf, stocks sur
  besoins, part cultivée, soldats sur nobles×20, grain à vendre, prix, l'étal en cours,
  les 7 premiers critères du titre suivant (part atteinte), terres barbares restantes,
  puis ce qui est venu avec les murs : les critères fortifications et hospice, ses
  fortifications et son hospice (dixièmes), ses béliers en stock.
- **La Chronique (12)** : naissances, immigrés, nobles et marchands partis, morts de
  maladie / malnutrition / famine, soldats perdus, delta de population, bilan du trésor,
  solde des soldats, peste.
- **Cinq voisins × 19** (`rivals(id)` = les cinq suivants à la table) : vivant, titre,
  surface **d'après le rapport d'éclaireur** (rien sans rapport — la surface « entendue »
  a disparu, ce serait tricher), grain à l'étal et prix, m'a attaqué / je l'ai attaqué /
  je l'ai battu, ses guerres avec les autres, arpents qu'il m'a pris, puis le rapport
  d'éclaireur (âge, garnison, efficacité, fortifications) et la lecture d'agent (âge,
  trésor, stocks, avancée vers son titre).
- **Sa dernière réponse d'Extérieur (19)** : ce qu'il a ordonné l'an passé.

Tout ce que la vue contient est **ce qu'un seigneur voit ou entend** — pas de triche.

### Le décodage (`decode_intendance`, `decode_missions`, `decode_expeditions`)

- Deadband 0,05 : une sortie sous ce seuil est « rien ».
- Intendance : rations (0–2× la ration pleine des serfs, 0–1,5× celle des soldats), les
  trois taxes, étal (part des stocks, prix), achat (part du trésor, sur l'étal le moins
  cher), vente de terre (part du maximum), puis les **neuf achats** (marchés, moulins,
  fonderies, chantiers, palais, soldats, fortifications, hospice, béliers) servis du plus
  voulu au moins voulu sur un trésor courant, le palais, les murs et l'hospice plafonnés
  à 10 dixièmes, l'armée à 20 par noble. Les rations sont
  ensuite bornées par les stocks (`bound_council`, règle des curseurs du web).
- Extérieur, **en deux lectures** du même réseau, parce que l'éclaireur répond sur-le-champ :
  `missions` d'abord (éclaireur = argmax sur 6, le sixième est « personne » ; agents =
  sorties ≥ 0,5), le rapport entre au dossier, puis `expeditions` sur la vue rafraîchie :
  une envie par cible (5 voisins + barbares), triées ; au plus `nobles/4 + 1`
  expéditions ; hommes = envie × soldats ; la première expédition contre un royaume
  emmène envie × béliers en stock (arrondi). Pas de guerre avant `FIRST_WAR_YEAR` (an 3).
- `Stage` (`Survive < Emperor < Market < Guard < War`) ferme les sorties non encore
  enseignées ; en jeu tout est ouvert (`Stage::War`).

### La mémoire (`Memory`)

Ce qu'un seigneur retient d'un an sur l'autre : la Chronique (`demo`, `eco`, `plague`),
les rumeurs entendues (`heard` : qui a marché sur qui, qui a battu qui, arpents perdus),
les dossiers d'éclaireur et d'agent, la réponse d'Intendance de l'année et les derniers
ordres. Le web la reconstruit chaque an (`Room::remember`), le terminal la garde.

## 2. L'école (`apps/empire-train`)

**Neuro-évolution par la méthode d'entropie croisée** (CEM) : pas de gradient.

- Population 1 000 génomes tirés autour d'une moyenne ± sigma (première génération à
  10 000, une loterie pour un départ qui vive) ; élite 100 ; la moyenne et le sigma
  glissent sur l'élite (`sigma ← 0,3·sigma + 0,7·écart-type`, plancher 5 % de l'échelle
  initiale — **c'est ce plancher qui rend les biais semés collants**).
- Chaque génome joue `--tables` (6) parties de 6 sièges ; sa note est la moyenne.
- **Tables mixtes** : 1 à 5 sièges de la population, les autres pris dans le
  **Hall of Fame** (`--hall 8` : les meilleurs de générations passées, étalés sur la
  course, pour ne pas dériver loin de ce qui a déjà gagné) et dans les écoles
  `--against` (leurs `.best.json`, chacune lue à son propre palier).
- **Cursus** par paliers (`--stage`) : `survive` (rations, taxes, achats, terre) →
  `emperor` (raids barbares) → `market` (grain) → `guard` (garnison, renseignement) →
  `war`. Les écoles s8+ ont été faites **d'un trait au palier war** (1 100 générations),
  le cursus n'apportait rien.
- `--from` reprend une distribution ; un génome d'une forme plus étroite est **grandi**
  (`Brain::grown`, d'une `Shape` — `own`, `rival`, `a_out`, `b_out` — à `Shape::NOW`) :
  ses poids restent où ils étaient, les entrées nouvelles naissent aveugles à leur sigma
  initial, les sorties nouvelles à zéro (une sigmoïde à 0,5 : « moitié », que l'école
  déplace). Le fichier d'école porte ses `widths` ; sans le champ, c'est la forme d'avant
  les murs (s26 : 38 / 19 / 15 / 18).
- Sortie : `<out>.json` (moyenne, sigma, Hall, meilleur, `widths`) pour reprendre,
  `<out>.best.json` le meilleur génome ; `--deliver <chemin>` écrit le meilleur en
  floats LE, c'est `brains/schooled.f32`.
- ≈ 1,1 s la génération sur 10 threads (`RAYON_NUM_THREADS=10`) ; 1 200 générations ≈ 22 min.
- `--measure N --show` : joue N tables de clones (ou contre `--against`) et imprime le
  bilan ; `--show` déroule les années du siège 0.

## 3. La note (`arena.rs`)

`Outcome::fitness(longest)` :

| Terme | Valeur |
|---|---|
| Sacre | `1000 − an` |
| Sinon : survie + route | `100 · années / longest + 200 · progress` |
| Prince, Roi (une fois, au premier an) | `150 − an/2`, `300 − an` |
| Peuple (chaque an) | naissance +0,002, colon +0,01, mort de faim −0,05, noble ±0,3 |

`progress` = moyenne des parts atteintes des 9 exigences impériales (trésor non compté :
un palais épargné n'est pas un palais). Le siège sacré **continue à jouer en paix**, la
partie s'arrête quand chaque siège est sacré ou tombé (150 ans au plus).

`Table::scores` retranche au palier war **`rank_cost × sièges finis devant`** (`--rank 40`).
C'est ce coût de rang, avec le `1000 − an` du sacre, qui fait l'agressivité du brain
livré : couper un voisin paie deux fois (il finit derrière, et il ne me devance plus).
L'agressivité **n'est pas une personnalité**, c'est la fitness.

## 4. Le brain livré (s28) — ce qu'il vaut

Deux écoles du même âge (3 500 générations) sous les nouvelles règles : **s27** = s26 grandi
(`Brain::grown`) + 1 200 générations ; **s28** = de zéro, 3 500 générations d'un trait.

| `--measure 200` | s27 | **s28** |
|---|---|---|
| Clones : sacrés | 12 % (an 50 en médiane) | **24 %** (an 67, p90 98) |
| Clones : tombés | 88 % | **76 %** |
| Affamés par siège | 630 | **239** |
| Fitness clones | 217 | **355** |
| Seul contre cinq de l'autre | 98 contre 343 | 104 contre 213 |
| Lectures d'éclaireur par siège | 0,2 | 0,0 |

- **Aucune ne domine l'autre** : le siège minoritaire perd dans les deux sens (96,5 % de
  chutes) — le coût de rang punit celui qui joue autrement que sa table. En jeu les cinq
  ordinateurs sont le même brain, donc ça ne compte pas.
- **s28 joue long** : palais à 10 vers l'an 30, murs et hospice à 9–10, un bélier acheté
  de loin en loin ; mais très peu de serfs pour sa surface et des **stocks de grain énormes**
  jamais vendus (300 000 à 800 000 boisseaux) — la fitness ne compte pas le grain, rien ne
  l'en dissuade. C'est le défaut visible en jeu (étal vide).
- **s27** garde le style de s26 : tables de 25 ans, tout le monde tombe, murs et hospice
  10/10 sur les longues parties, jamais de bélier.
- Ni l'une ni l'autre n'envoie d'éclaireur : le renseignement ne paie toujours pas
  (§5, enseignement 1). Un s26 grandi sans rescolarisation ne valait rien sous les
  nouvelles règles (1 % de sacres, 99 % de chutes).

Pour mémoire, s26 sous ses règles : 13 % de sacres (an 43), Prince 18 %, Roi 15 %, 87 % de
chutes, tables de 24 ans. Contre un humain le brain reste très fort et très agressif.

## 5. Inventaire des écoles

Les fichiers sont dans le scratchpad de session (`schools/`, `run-s*.sh`, `s*.log`) ; les
deux points de reprise utiles sont archivés dans `apps/empire-train/schools/`
(`s10-war.json`, `s26-war.json`, `s27-war.json`, `s28-war.json` et leurs `.best.json`, 1,6 Mo
chacun, avec leurs `run-s*.sh`).

| École | Ce qu'elle a changé | Résultat |
|---|---|---|
| s1–s6 | cursus survive → market → war, **anciennes règles** | `s1-market.best.json` : le seul brain qui **liste du grain** (fitness 242, 137 ans de partie, 16 % de chutes, jamais sacré) ; les rungs war se sont effondrés |
| l-war-* | expérimentations du trainer (hall, cap, vary, hunger, reserve, floor, barb…) | obsolètes, règles depuis changées |
| s7 | reprise avec `--hall` sur toute la machine | dépassé par s8 |
| s8 « titres » | Prince/Roi payés, trésor non compté, exigences à poids égal | → devenu la fitness standard |
| s9 « trêve » | pas de guerre avant l'an 3 | → devenu une règle du jeu ; 49 ans de partie mais 1,7 % de sacres |
| s10 « peuple » | registre du peuple (naissances, colons, faim, nobles) | 1 100 gén., best ≈ 1 127, base de tout ce qui suit |
| s11–s13 | prime par rapport lu (0,5 / 3 pts), sorties réveillées (`s10-wake`) | l'éclaireur reste ignoré |
| s14–s15 | éclaireur à moitié / quart prix | idem |
| s16–s19 | école des lettres (éclaireur gratuit) puis payant à 150 / 75 / 37 | lit, puis cesse d'acheter |
| s20–s23 | lettres (`--letters scouts` / `all`) depuis s10, puis palier payant | biais semés collants : continue d'acheter, mais **perd** face au contrôle |
| s24–s25 | biais libérés (0 ± 1) au palier payant | abandonne éclaireurs et agents en ~40 générations |
| s26 | contrôle : s10 + 1 200 générations sans lettres | livré jusqu'aux murs ; à âge égal : scouts −16/−27 pts, scouts+agents −90/−81 |
| s27 | s26 grandi (43 / 19 / 18 / 19) + 1 200 générations sous les murs, l'hospice, les béliers et le renseignement immédiat | 12 % de sacres, murs et hospice 10/10, pas de béliers ; 142 contre 67 face à s26 grandi |
| **s28** | de zéro, 3 500 générations sous les mêmes règles | **livré** ; 24 % de sacres (an 67), 76 % de chutes, thésaurise le grain ; ne domine pas s27 en tête-à-tête |

Enseignements (détail dans la mémoire `empire-intel-findings`) :

1. **Le renseignement ne paie pas** aux prix et règles actuels ; ne pas re-proposer prime
   de lecture ni baisse de prix.
2. **Comparer des écoles à âge égal** (même nombre de générations) — sinon on mesure la
   durée d'entraînement, pas la variante.
3. **Les biais semés collent** (le plancher de sigma) : pour laisser un brain choisir,
   remettre le biais à 0 avec sigma 1 (`wake`).
4. Le cursus par paliers n'apporte rien : une école d'un trait au palier war vaut mieux.
   Et quand les règles changent, **repartir de zéro** (s28) a mieux valu que grandir
   l'ancien brain (s27) : à âge égal, deux fois plus de sacres et des parties plus longues.
5. Les « profils » (chasseur de titres, vendeur, diplomate) **n'ont jamais existé** comme
   brains entraînés : `ia-profils.md` du scratchpad décrivait des tables de gènes pour
   l'ancien `Mind` (arbre de décision, supprimé).

## 6. Reprendre

```bash
# reprendre s28 (1 200 générations de plus, mêmes réglages)
RAYON_NUM_THREADS=10 cargo run --release -p empire-train -- \
  --stage war --from apps/empire-train/schools/s28-war.json \
  --generations 1200 --tables 6 --hall 8 --rank 40 --out s29-war.json

# mesurer une école contre une autre
cargo run --release -p empire-train -- --stage war --from s29-war.json \
  --against apps/empire-train/schools/s28-war.best.json --measure 200 --show

# livrer : le meilleur génome devient brains/schooled.f32 (floats LE)
cargo run --release -p empire-train -- --stage war --from s29-war.best.json \
  --deliver libs/empire-lib/brains/schooled.f32
```

Si la vue ou les sorties s'élargissent (`OWN`, `RIVAL`, `A_OUT`, `B_OUT`), `load` grandit
les anciens génomes automatiquement d'après les `widths` du fichier d'école ; ajouter la
forme d'hier comme constante nommée dans `Widths` si un fichier sans `widths` doit encore
se lire.

## 7. Prochaine étape IA (parquée)

Des tempéraments, pas des règles : un `Temper` de poids de fitness sur la `Table`
(marchand : le grain vendu paie ; bâtisseur : la route et les titres pèsent plus, le rang
moins ; conquérant : le rang et les arpents pris ; diplomate : demanderait de nouvelles
entrées — qui m'a attaqué, qui m'a vendu du grain — déjà en partie dans `heard`), chaque
école partant de s26 et **co-entraînée** contre les autres (`--against`), mesurée sur des
tables mixtes avec pour cible des parties de 60–100 ans et moins de la moitié des sièges
tombés. En jeu, un tempérament par siège d'ordinateur. Mais d'abord : les nouveaux
bâtiments (`BATIMENTS.md`), puis une rescolarisation avec la vue et les sorties élargies.
