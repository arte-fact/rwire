# Le Brain — point d'arrêt (septembre 2026)

État : **livré** (`brains/soldat.f32`, `batisseuse.f32`, `garnison.f32`, `boutiquiere.f32` —
s46a et s48b–d, quatre sœurs de zéro contre s35, sous les murs, l'hospice, les béliers et le
renseignement immédiat de `BATIMENTS.md` / `RENSEIGNEMENT.md` ; `Brain::schools()`, une par
tempérament). Ce document
fige ce que les ordinateurs d'Empire savent faire, comment ils l'ont appris, ce qui a été
essayé et ce qui reste ouvert — pour pouvoir reprendre l'entraînement plus tard sans rien
redécouvrir.
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
little-endian dans `brains/<tempérament>.f32`). Il n'y a **aucune règle codée à la main** dans le jeu
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
  floats LE, ce sont les `brains/<tempérament>.f32`.
- ≈ 1,1 s la génération sur 10 threads (`RAYON_NUM_THREADS=10`) ; 1 200 générations ≈ 22 min.
- `--measure N --show` : joue N tables de clones (ou contre `--against`) et imprime le
  bilan ; `--show` déroule les années du siège 0.

## 3. La note (`arena.rs`)

`Outcome::fitness(longest)` :

| Terme | Valeur |
|---|---|
| Sacre | `1200 − 6·an` depuis s47 (900 à l'an 50, 300 à l'an 150 : un sacre tardif reste au-dessus de toute route, cinquante ans plus tôt valent 300) ; s35–s46 : `1000 − 4·an` |
| Sinon : survie + route | `100 · années / longest + 200 · progress` |
| Prince, Roi (une fois, au premier an) | `150 − an`, `300 − 2·an` |
| Peuple (chaque an) | naissance +0,002, colon +0,01, mort de faim −0,05, noble ±0,3 (s42 les a payés ×5 sans rien déplacer) |
| Rien d'autre | ni la bataille perdue ni la chute (s37, s38 : plateau pacifique), ni l'usage des outils (s39, s40 : fermes à points) |
| Guerre | rien : ni la bataille perdue ni la chute ne coûtent au-delà des années perdues (s37 et s38 les faisaient payer, et personne ne cherchait plus le sacre) |

`progress` = moyenne des parts atteintes des 9 exigences impériales (trésor non compté :
un palais épargné n'est pas un palais). Le siège sacré **continue à jouer en paix**, la
partie s'arrête quand chaque siège est sacré ou tombé (150 ans au plus).

`Table::scores` retranche au palier war **`rank_cost × sièges finis devant`** (`--rank 40`).
C'est ce coût de rang, avec le `1200 − 6·an` du sacre, qui fait l'agressivité du brain
livré : couper un voisin paie deux fois (il finit derrière, et il ne me devance plus).
L'agressivité **n'est pas une personnalité**, c'est la fitness.

## 4. Les brains livrés — ce qu'ils valent

### Les quatre sœurs (livrées le 12 septembre 2026)

`Brain::schools()` : **le soldat** (s46a), **la bâtisseuse** (s48b), **la garnison** (s48c),
**la boutiquière** (s48d) — quatre génomes de la même recette (§5, s46) aux tempéraments
stables ; trois d'entre eux repris par s48 (quarante manches entre sœurs et s35 aux tables
de 100 ans), qui sacrent plus souvent et plus tôt que leurs versions s46 ; le soldat s48
ayant reculé (15 → 10,5 % contre s35), c'est le s46 qui est livré. En ligne (`apps/empire-web`), chaque table tire une école par siège d'ordinateur :
les quatre une fois, deux autres au hasard, le tout mélangé (`Schooling`) ; rien ne dit
aux joueurs qui est qui, c'est à deviner sur le style. Au terminal
(`apps/empire`), le siège n° i joue l'école i mod 4. Ce qu'elles valent : § 5, lignes s46
(match à six) et 8e enseignement. s28, le brain livré avant elles, reste en
`schools/s28-war.json`.

### s28 (livré du 10 au 12 septembre 2026)

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
| s31 | rééquilibrage défense : murs 1 000 le dixième (×2 à 10/10), bélier 500 frappant tous les **16** échanges | tout le monde se mure, sièges gelés : 9 % de sacres à l'an 140, 137 ans de partie ; arrêtée à 1 575 |
| s32 | bélier tous les **8** échanges | bascule inverse : murs ignorés, agression pure — 21 % de sacres à l'an 62, 79 % de chutes, tables de 30 ans |
| s33 | bélier tous les **12** échanges | comme s31 (11 % à l'an 147) ; le multiplicateur de murs est **bistable**, pas de milieu |
| s34 | 8 échanges, bélier à **2 000** | 25 % de sacres mais à l'an 136, 60 % de chutes ; à 1 500 la même école bascule à 89 % de chutes |
| s35 | bélier 1 500 ; note pressée : sacre `1000 − 4·an`, Prince `150 − an`, Roi `300 − 2·an` | 23 % de sacres à **l'an 59** (136 → 59 par la seule note), 77 % de chutes ; la meilleure sous les règles complètes |
| s36 (essai) | **titres allégés** d'un cran (Prince : palier plus léger — 2 000 serfs, 5 nobles, 1 palais ; Roi = ancien Prince ; Empereur = ancien Roi) | 32 % de sacres, p10 28 / **médiane 39** / p90 55, 68 % de chutes — s35 sous la même règle 22 %, s28 21 %. Résultat intéressant, **règle non retenue** : l'échelle des titres reste l'originale ; l'hypothèse suivante est la terre manquante (s37) |
| s37 | terres des morts aux barbares, mère affamée **−300** | **plateau pacifique** : 0 % de titres en 1 200 gén., tables de 141 ans, 19 % de chutes — à la gén. 10, 91 % des sièges tombaient sous la mère, la sélection n'a vu que « ne plus affamer » et la lignée des titres n'est jamais née (s35 avait ses premiers Princes à la gén. 25) |
| s38 | l'école de l'intendant : grenier/trésor jugés chaque an, batailles perdues −5, chute −100 | **même plateau** : 0 % de titres en 1 200 gén., tables de 142 ans, 14 % de chutes, 24 batailles perdues par siège, 2 nobles à vie — punir la chute et la défaite élimine le pool des guerriers, d'où sortent les sacrés (la terre du sacre ne vient que de la conquête) |
| s39 | l'intendant sans le gendarme : grenier/trésor jugés chaque an, **rien** sur la défaite ni la chute, +0,5 l'an de marché, +0,2 **par rapport lu** ; de zéro | **arrêtée à la gén. 138** : 0 % de titres, 4 rapports lus par an et par siège (≈ 110 points sur 230), 28 % de chutes — payé à la pièce, le rapport devient une ferme à points, le brain achète des lettres au lieu de conquérir |
| s40 | la note de s35 (titres, peuple, cour, rien sur la chute) et **+0,2 l'an** par outil de guerre utilisé : murs, béliers au siège, rapport lu ; de zéro | **arrêtée à la gén. 229** : 0 % de titres, tables de 139 ans, 18 % de chutes. Le meilleur brain **simule l'usage** : murs 10/10 dès l'an 10, éclaireur chaque année, et 111 « sièges » menés avec **25 hommes et 232 béliers** (tous cassés, battu chaque fois) pour cocher l'année ; trésor à 289 000, 194 serfs à l'an 120. Même à 0,2 par an, un usage qui se simule pour rien se simule |
| s41 | **le contrôle** : la note de s35 telle quelle, de zéro, sous la règle des terres (s35 a été scolarisée avant elle) | **arrêtée à la gén. 155** : 0 % de titres, tables de 137 ans, 29 % de chutes — le même plateau que s37–s40. **Verdict : c'est la règle des terres.** Les six écoles de zéro d'avant (s31–s36) avaient des sacres à la gén. 50 (2 à 11 %) et 15–20 % à la gén. 100 ; les cinq d'après (s37–s41), zéro, quelle que soit la note. Mécanisme : quand les arpents des affamés vont aux barbares, la terre est abondante et sans risque, la conquête (qui apportait aussi serfs et nobles) n'a plus de raison de naître ; le meilleur brain grignote ses voisins à 25 hommes et 300 béliers, trésor 6,6 M, 6 serfs à l'an 150. **La règle reste à trancher** (retirer ; ou terre sans maître qu'on peut envahir) |
| s42 | **l'école du grenier** : la note de s35 avec la naissance ×5 (0,01) et le noble ×5 (±1,5) — tous deux viennent du grain donné ; **reprise de s35** (`--from`) et jouée **contre les meilleurs de s35, s34 et s33** (`--against`, `--hall 0` : des rivaux agressifs qui le restent) pour lui faire trouver, face à des conquérants, une autre façon de tenir ; sous la règle des terres | **arrêtée à la gén. 2 162, sans effet** : 25 % de sacres aux tables mixtes du début à la fin, médiane an 56, affamés et nobles inchangés ; mesurée seule contre 5 × s35 : 22 % de sacres comme l'ancêtre. Posée sur l'optimum de s35, sigma refermé, la population ne trouve pas de pente vers le grain à ×5 — la note est revenue à celle de s35 |
| s43 | **s35 de zéro contre s35** : la note de s35 telle quelle, de zéro, les autres chaises tenues par **le meilleur de s35** figé (`--against s35-war.json --hall 0`) — un conquérant en face dès la première génération ; sous la règle des terres | **la lignée des titres renaît** : Princes gén. 50, 11 % de sacres gén. 100, palier à 20–21 % dès la gén. 500 (tables 30 ans). Mesurée à la fin (300 tables) : **1 contre 5 × s35 : 23,7 % de sacres, médiane an 61** (les s35 en face 18 % ; s35 contre lui-même 22 %) ; entre clones 20,7 %, médiane an 64 ; mais **contre 5 × s28 : 8 %** (s35 contre s28 : 19 %) — un spécialiste de son rival figé. Personnalité (`--show`) : même script à chaque partie — marché et palais, vend ~600–1 900 arpents aux barbares **chaque année**, dès l'an 3 deux fronts contre deux voisins sans jamais garder de garnison, puis dès l'an 19 des raids barbares de 200–450 hommes qui reprennent la terre vendue et celle des affamés (12 000 → 66 000 arpents), moulins en rafale, sacre an 56 ; les parties perdues sont des annexions vers l'an 13–16 en attaquant encore. Béliers achetés, jamais emmenés |
| s44 | **de zéro contre un mélange** : la note de s35, une population de zéro, et sur chaque chaise libre un rival tiré au hasard parmi les meilleurs figés de **s35, s28 et s43** (`--against` ×3, `--hall 0`) — la table change à chaque partie, pour obtenir la renaissance des titres de s43 sans son spécialiste ; sous la règle des terres | **arrêtée à la gén. 153 : plateau pacifique**. Princes à la gén. 50 (1,4 %) puis plus aucun titre dès la gén. 60 ; tables 56 → 107 ans, chutes 95 → 46 %, le profil de s41. Les chaises s28 (un tiers des voisins) ne mordent pas : aux tables qu'ils dominent, c'est la survie qui paie, et l'élite s'est sélectionnée là-dessus. Un rival figé ne fait renaître les titres que si la pression est **à chaque table** |
| s45 | **de zéro contre un mélange sans pacifique** : la note de s35, une population de zéro, chaque chaise libre tirée au hasard entre les meilleurs figés de **s35 et s43** (`--against` ×2, `--hall 0`) — deux conquérants de styles différents, la pression à chaque table sans un seul adversaire à apprendre par cœur ; sous la règle des terres | **arrêtée à la gén. 100 : plateau pacifique**, la courbe de s44 (Princes 1 % gén. 50, plus rien après, tables 99 ans). Mesurée : annexée 95–100 % du temps par ses rivaux, ne survit qu'entre clones. **Contre-épreuve** : la recette exacte de s43 rejouée (autre graine, 150 gén.) donne le même plateau — Princes 0,6 % gén. 40, zéro dès la gén. 70. **s43 était un coup de dés** : sous la règle des terres, la lignée des titres naît vers la gén. 45 à 1–3 % de Princes et doit devenir Rois avant que la survie aux longues tables ne prenne l'élite ; elle a gagné cette course une fois sur huit (s37–s41, s44, s45, la contre-épreuve) et six fois sur six avant la règle |
| s46 | **la recette de s43 rejouée jusqu'à quatre écoles valides** (`run-s46.py`) : de zéro, note de s35, `--against s35-war.json --hall 0`, 300 gén., cinq essais à la fois ; un essai est jeté si la gén. 80 passe sans sacre, gardé s'il finit à ≥ 5 % | **3 gagnantes sur 35 essais** (+ s46a d'avant) : sacres finaux à leurs tables d'école 20,7 % (a), 21,7 % (b), 24,2 % (c), 17,4 % (d), médiane an 110–125. **Quatre tempéraments stables** (mêmes chiffres en match à six et entre clones) : **a le soldat** (le plus de soldats achetés, quatre fois moins de moulins, murs négligés — la seule annexée dans les deux parties), **b la bâtisseuse** (moulins, marchés, murs et grain en tête, le moins de soldats, mais 33 victoires aux béliers ; sacrée an 123 du match), **c la garnison** (172 soldats entretenus contre 66–101, paysans sacrifiés), **d la boutiquière** (marché dès l'an 8, le plus de marches, sacrée an 128 entre clones). Match à six (`--show` avec cinq `--against` = une école par chaise, tous les sièges racontés) contre s35 et s43 : les deux conquérants vendent leur terre et meurent de faim l'an 9 et l'an 11 ; les quatre sœurs jettent 50 000 hommes contre des murs à 10 pendant 85 ans, puis la famine de la Germanie (an 106) ouvre le bal : France annexée an 121, Bretagne (b) Empereur an 123, Castille (d) et Germanie (c) mortes de faim ans 130 et 134 |
| s47 | **les quatre sœurs l'une contre l'autre** (`run-s47.py`) : chacune reprise de s46, `--against` les trois autres figées, `--hall 0`, par manches de 25 gén. (chaque manche relit les sœurs), quatre entraîneurs à 3 threads ; sacre **`1200 − 6·an`** pour presser le sacre | **arrêtée à la manche 5 (gén. 424)** : entre sœurs murées, les sacres reculent à l'an 138–146 et deux sœurs sur quatre glissent vers le plateau pacifique (a : chutes 63 → 24 %, Princes 37 → 5 %, sacres 0,2 % ; d : 3,3 %) ; b et c tiennent à 18–20 % sans avancer. La survie (~210 points sûrs sur 150 ans) bat un sacre à 4 % après l'an 138 ; la note pressée ne raccourcit pas un sacre qui n'existe plus |
| s48 | **s47 aux tables courtes avec s35 sur une chaise** (`run-s48.py`) : mêmes manches, reprises de s46, `--against` les trois sœurs **et s35** (un conquérant sans murs à chaque table), **`--longest 100`** : survivre ne rapporte plus rien après l'an 100, il faut sacrer avant | **finie (40 manches, gén. 1299)** : aux tables d'école les sacres montent de 3–5 % à **a 8,0 · b 6,7 · c 9,6 · d 9,4 %** (la moitié du gain dans les dix premières manches, palier dès la 25e), médianes figées à l'an 80–93. Mesure à 150 ans, chaque sœur seule contre cinq s35 (200 tables), s46 → s48 : **a 15 → 10,5 %** (médiane 78 → 74), **b 5,5 → 11 %** (96 → 73), **c 19 → 26 %** (80 → 72), **d 4 → 7,5 %** (93 → 76) — trois sœurs sur quatre sacrent plus souvent, toutes sacrent plus tôt, le soldat recule. Match à six contre s35 et s43 : s35 annexe le soldat (France) l'an 16, s43 meurt de faim l'an 16, s35 l'an 40 ; **deux sacres l'an 89** (b et d, contre un seul l'an 123 chez s46), puis la garnison (c) annexe la boutiquière l'an 123 avant de mourir de faim l'an 127 |

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
6. **Sous la règle des terres, une école de zéro entre clones plafonne** (s37–s41) ; **un
   rival conquérant figé en face** (`--against`, s43) recrée la pression et la lignée des titres
   renaît — mais l'élève se spécialise contre ce rival (s43 bat s35 et perd contre s28). Mais
   s44 (s35/s28/s43 au hasard), s45 (s35/s43) et la recette de s43 rejouée plafonnent tous :
   s43 a gagné une course perdue sept fois sur huit sous la règle des terres. Trancher la règle.
8. **Une même recette donne des tempéraments différents** (s46a–d : soldat, bâtisseuse,
   garnison, boutiquière), stables d'une table à l'autre — le style est dans le génome, pas
   dans la chaise. Ce qui dépend de la chaise : qui laisse partir ses paysans (celui qui
   prend les coups). Entre sœurs aux murs à 10, rien ne bouge tant que personne n'a faim :
   la partie se décide au premier grenier vide, et gagne celle qui a le plus de stock.
7. **Le taux de sacre suit le taux de survie** (s32–s36 : sacrés ≈ 100 % − tombés, à
   quelques points près) : ce qui tombe ne sacre pas, et la note ne sacre vite qu'un
   siège qui a de la terre. D'où s37 : quand un royaume tombe sous le couteau d'une
   mère affamée, **ses terres reviennent aux barbares** (`EmpireGame::break_up`) au lieu
   de disparaître de la carte, et la note punit cette chute de 300 points.

## 6. Reprendre

```bash
# reprendre s35 (1 200 générations de plus, mêmes réglages) — ou `run-s41.sh` de zéro
RAYON_NUM_THREADS=10 cargo run --release -p empire-train -- \
  --stage war --from apps/empire-train/schools/s35-war.json \
  --generations 1200 --tables 6 --hall 8 --rank 40 --out s43-war.json

# une école de zéro contre un rival fixe (s43 : `run-s43.sh`) — 1 à 5 sièges de la
# population par table, les autres tenues par le meilleur de --against
RAYON_NUM_THREADS=10 cargo run --release -p empire-train -- \
  --stage war --against s35-war.json --hall 0 \
  --generations 1200 --tables 6 --rank 40 --out s43-war.json

# un match : cinq --against = une école par chaise (France = --from, puis dans l'ordre
# Britanny, Germany, Spain, Moscovy, Persia), tous les sièges racontés année par année
target/release/empire-train --stage war --from s46a-war.json --against s46b-war.json \
  --against s46c-war.json --against s46d-war.json --against s35-war.json \
  --against s43-war.json --show --tables 6 --rank 40

# les sœurs l'une contre l'autre par manches (s48 : `run-s48.py`, quatre entraîneurs à la fois)
python3 apps/empire-train/schools/run-s48.py

# une sœur seule contre cinq s35, 200 tables de 150 ans (le bilan de s48)
target/release/empire-train --stage war --from s48c-war.json --against s35-war.json \
  --measure 200 --rank 40

# mesurer une école contre une autre
cargo run --release -p empire-train -- --stage war --from s29-war.json \
  --against apps/empire-train/schools/s28-war.best.json --measure 200 --show

# livrer : le meilleur génome d'une école devient un brains/<tempérament>.f32 (floats LE)
cargo run --release -p empire-train -- --stage war --from s48c-war.json \
  --deliver libs/empire-lib/brains/garnison.f32
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
