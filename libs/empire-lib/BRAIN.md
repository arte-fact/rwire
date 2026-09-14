# Le Brain — point d'arrêt (14 septembre 2026, règles v3)

État : **en régénération**. Les brains livrés (`brains/*.f32`, s46a et s48b–d, s53) ont été
formés sous la version 1 des règles ; le jeu tourne maintenant sous la **version 2**
(`empire_lib::RULES = 2`), où ils jouent encore mais mal (ils achètent moins qu'ils ne
veulent). Décision du 13 septembre : on ne porte pas de rétrocompatibilité, on jette les
modèles et on les régénère ; les règles sont versionnées, un fichier d'école d'une autre
version est refusé, et le script de régénération fait foi. Ce document fige ce que les
ordinateurs savent faire, comment ils l'apprennent, ce qui a été essayé et ce qui reste
ouvert. Code : `src/brain.rs` (le joueur), `src/arena.rs` (la table et la note),
`libs/empire-gpu` (la même arène en WGSL), `apps/empire-train` (l'école).

## 1. Ce qu'est un ordinateur

Chaque siège tenu par l'ordinateur est un `Brain` : **deux réseaux denses** à une couche
cachée (32 neurones par défaut, `--hidden` pour plus ; `tanh` dedans, sigmoïdes en sortie),
lus une fois par an chacun.

| Réseau | Entrées | Sorties | Rôle |
|---|---|---|---|
| `intendance` | `SIGHT` = 201 | `A_OUT` = 18 | rations, taxes, marché du grain, vente de terre, achats |
| `exterieur` | `SIGHT` + 18 | `B_OUT` = 51 (19 ordres + 32 de recall) | éclaireur, agents, puis expéditions (5 voisins + barbares) et béliers ; les 32 dernières sorties sont relues l'an d'après |

Génome à 32 neurones : 15 781 poids (les deux réseaux bout à bout ; 33 445 sous v2, avec le journal). Il n'y a **aucune règle
codée à la main** dans le jeu de l'ordinateur en dehors du décodage ci-dessous.

### La vue (`sight`, 201 entrées) et ce qu'un brain en lit (`Reads`)

Les parts sont lues telles quelles, les comptes en **échelle log** (`count(n, scale)`).

- **Son royaume (43)** : année, météo, arpents, serfs, nobles, marchands, soldats,
  efficacité, ration, trésor, stocks, récolte, rats, bâtiments, taxes, titre, ratios, étal,
  critères du titre suivant, terres barbares, fortifications, hospice, béliers.
- **La Chronique (12)** : naissances, immigrés, départs, morts, pertes de soldats, bilan.
- **Cinq voisins × 19** : vivant, titre, surface lue par l'éclaireur, étal, rumeurs de la
  campagne (m'a attaqué, je l'ai attaqué, battu, arpents pris), rapport d'éclaireur et
  lecture d'agent, chacun avec son âge.
- **Sa dernière réponse d'Extérieur (19 ordres + 32 de recall)**.
- ~~Le journal (4 années × 69)~~ : **retiré sous v3** (14 septembre). Il donnait d'office
  l'état et les guerres de chaque royaume sur quatre ans et les rapports d'éclaireur du
  siège ; s62c a montré qu'il coûtait trois à six points de sacres à toute lecture qui le
  portait. Avec lui sont partis `Memory::note`, `Recorded`, `Sighted` et le bloc du noyau.

Deux **drapeaux de lecture** par brain (`Reads`, portés par le fichier d'école et par le
siège dans le noyau GPU) : `told` (la Chronique, les ordres de l'an passé, les vieux
rapports), `recall` (les 32 sorties relues). Ce qu'un brain ne lit pas est à
zéro sur sa vue : **une seule forme de génome sert toutes les lectures**, ce qui permet de
les comparer à recette égale (s61, s62). Les `.f32` livrés lisent la Chronique seule.

### Le décodage (`decode_intendance`, `decode_missions`, `decode_expeditions`)

- Deadband 0,05 : une sortie sous ce seuil est « rien ».
- Intendance : rations, taxes, étal, achat de grain, vente de terre, puis les **neuf achats**
  servis du plus voulu au moins voulu sur un trésor courant ; **une envie achète sa puissance
  quatrième de ce que le trésor permet** (`n = envie⁴ × plafond`, v2) — une petite envie
  achète quelques unités quel que soit le trésor, une envie de 1 vide la caisse. En v1
  l'achat était linéaire et un trésor de 60 M faisait acheter 30 000 moulins par an.
- Extérieur, en deux lectures : missions (éclaireur = argmax sur 6, agents ≥ 0,5), le rapport
  entre au dossier et au journal, puis expéditions sur la vue rafraîchie. Pas de guerre
  avant l'an 3.

### Le recall, ce qu'il fait vraiment (lu sur s63, 14 septembre)

Tracé sur 200 parties par école (`--trace`, une ligne par an : l'état du siège et les 32
cases écrites), à 100 générations (pool B) et à 400 (les tempéraments livrés) :

- **À 100 générations, des intégrateurs à fuite.** Deux à sept cases figées, une seule
  qui saute ; les autres suivent continûment une grandeur de l'état, presque toujours le
  trésor en log (corrélations 0,6–0,75), sinon les réserves, les nobles, l'année. L'état
  courant explique 28–40 % de leur variance, les deux années précédentes n'y ajoutent que
  3–6 points (rien n'y mémorise un événement), mais la case de l'an d'avant porte le tout
  à 49–55 % avec des coefficients de 0,4 à 0,9 : chaque case lisse la grandeur qu'elle
  suit, une tendance plutôt qu'une valeur.
- **À 400 générations, des bits de phase apparaissent.** Sept à treize cases sautent
  (|Δ| > 0,5) au moins une fois par partie, de deux sortes : des **verrous d'ouverture**
  qui basculent une fois pour toutes à l'an 2–5 dans presque toutes les parties (t-5 r9 et
  r30, t-12 r8 et r25 : montée à 98 %, jamais redescendues — « la première année est
  passée ») ; et des **bascules de milieu de partie** à l'an 8–15, quand le trésor passe
  5 000–15 000 et l'armée 50–150, deux à trois fois par partie, pour moitié définitives —
  elles précèdent le sacre de 8 à 14 ans dans plus de 95 % des parties sacrées : la phase
  « assaut » de s59. L'état courant n'explique plus que 16–35 % de la variance, la case
  d'avant beaucoup plus : l'automate remplace le lissage.
- **Ablation** (`--no-recall`, entrées à zéro) : les sacres tombent de 33–36 % à 0–4 %
  et les chutes montent de 45–50 % à 84–98 %. Une dépendance apprise, pas une mesure de
  la valeur de la mémoire : le brain n'a jamais vu de zéros à cet endroit de sa vue. La
  comparaison juste reste s62c, recall contre témoin à recette égale (même plafond, départ
  deux fois plus rapide, sacre deux ans plus tôt ; avec la Chronique, +6 points).

## 2. L'école (`apps/empire-train`)

**Élevage** (`--breed 0.3`, s59, la recette de référence depuis le 13 septembre) : la
population de 1 000 est faite des 100 parents (l'élite) et de 900 enfants, neuf par
parent, chaque poids muté d'une gaussienne d'écart-type `0,3 × échelle de départ` ; l'élite
suivante est prise parmi parents et enfants. Cent lignées au lieu d'une : contre le pool de
prod v1, 45 % de sacres en médiane à 300 générations là où l'**entropie croisée** (le
tirage autour d'une moyenne, `--breed 0`) plafonnait à 17 % et n'y changeait rien en 600.
La première génération (10 000) reste une loterie.

- Chaque génome joue `--tables` (32, un multiple de 32) parties de 6 sièges ; sa note est
  la moyenne. Tables mixtes : 1 à 5 sièges de la population, les autres pris dans
  `--against` (le pool de référence) et dans le Hall (`--hall`). Une compagnie tirée joue
  **32 tables d'un coup** (graines et chaises différentes) : ce sont les 32 voies d'un groupe
  de travail du GPU, qui lisent ainsi les mêmes six génomes depuis le cache ; chaque génome
  est noté sur 32 parties au lieu de 6 pour le même prix.
- **GPU** (`--gpu`, `libs/empire-gpu`) : toute l'année d'une table jouée dans un noyau WGSL,
  une table par voie, règles et dés identiques au CPU (test de parité `tests/year.rs`).
  Rentable à partir de ≈ 6 essais en un processus (`--trials N`, tous les essais dans une
  même passe) : 8 essais à 32 neurones font 2 à 4 s la génération, 20 essais 2,8 s contre
  7,1 s sur 12 threads. Au-delà de 32 neurones le noyau est borné par la lecture des poids
  (chaque table lit six génomes) ; les passes sont découpées (50 000 table-années) pour
  rester sous le délai du pilote. Avec `--gpu`, les cœurs jouent aussi : chaque génération
  est partagée entre la carte et le CPU selon le débit que chacun a montré à la
  précédente (`Pace`). Sur `threadreaper` (3090 + 32 threads) : 8 essais h32 en 1,9 s la
  génération, 32 essais en 6,7 s.
- `--hidden H` (couche cachée, les rivaux plus étroits sont élargis par des neurones muets),
  `--told --recall` (les lectures d'une école de zéro), `--keep N` (un instantané
  toutes les N générations, pour les courbes), `--trials N` avec `{n}` dans `--out` et
  `--from`.
- Sortie : `<out>.json` (moyenne, sigma, Hall, meilleur, parents, `widths`, `rules`, les
  lectures), `<out>.best.json`, `<out>.g<n>.json` ; `--deliver` écrit le meilleur en floats.
- **Mesure** : `--measure N --gpu` joue N tables d'un siège contre cinq du pool et imprime
  le bilan (sacres, année médiane, chutes, lectures, fitness) ; c'est la seule note qui fasse
  foi, la lecture d'école (tables mixtes) est indicative. `--show` déroule une partie.
- **Pool de référence** (`schools/pools/A`, figé, 8 écoles de zéro élevées entre elles sous
  v2) : chaque campagne s'entraîne contre lui et s'y mesure ; `promote.py` fait entrer **un
  champion à la fois** (le meilleur candidat contre le pool et contre ses pairs remplace le
  membre le plus faible en tournoi) dans un nouveau dossier, jamais en place.
- Les campagnes sont des scripts (`run-s58.py` … `run-s62.py`) lancés **détachés** (`nohup
  setsid`) : l'outil de session tue les tâches d'arrière-plan longues.

## 3. La note (`arena.rs`)

`Outcome::fitness(longest)` : sacre `1200 − 6·an` ; sinon `100 · années / longest + 200 ·
progress` ; Prince `150 − an`, Roi `300 − 2·an` ; le peuple chaque an (naissance +0,002,
colon +0,01, mort de faim −0,05, noble ±0,3). Rien sur la bataille perdue ni la chute.
`Table::scores` retranche `rank_cost × sièges finis devant` (`--rank 40`).

**v2 : la table s'arrête au premier Empereur** (ou quand tout le monde est tombé), comme
la partie d'un seigneur ; plus d'après-sacre hors score, des tables trois fois plus courtes,
et un seul sacre par table — le « sacres par siège » plafonne donc vers 25–30 %, il ne se
compare pas aux chiffres v1. Au siège, **au plus un bélier par dixième de mur** monte à
l'assaut par cadence, les autres attendent au camp (une boucle de 400 000 béliers faisait
sauter le GPU).

## 4. Les brains livrés — ce qu'ils valent

Les sept `.f32` sont de la **version 3**, livrés le 14 septembre depuis s63 (seize écoles
de zéro contre le pool B, told + recall, 400 générations) ; `Brain::schools()` les lit à
la forme actuelle avec `Reads::ALL`. Mesurés seuls contre cinq du pool B (500 tables) :

| Brain | École | Sacres | An | Ce qui le distingue (200 parties tracées) |
|---|---|---|---|---|
| bâtisseuse | t-4 | 36 % | 22 | 1 300 moulins, les terres les plus vastes (20 800 arpents), 185 soldats |
| garnison | t-5 | 33 % | 22 | la plus grosse armée entretenue (199), 38 000 boisseaux en grenier, jamais de bélier |
| prudente | t-12 | 31 % | 21 | murs 6/10, 6 hospices, 8 palais, un éclaireur chaque année |
| boutiquière | t-16 | 31 % | 19 | 19 000 livres de trésor, du grain à l'étal 9 ans sur 10 |
| conquérante | t-14 | 26 % | 22 | 55 béliers, 18 chantiers, 113 foires, 171 hommes en marche par an |
| soldat | t-2 | 25 % | 22 | le plus d'hommes en marche (173 par an), 640 moulins, 20 béliers |
| fonceuse | t-8 | 24 % | 19 | sacre médian an 13 aux tables d'école, murs 4/10 et palais négligés |

Pool de référence : `pools/B` (8 écoles de zéro, v3) ; `pools/C` = B où t-4 a remplacé le
membre muet (`promote.py`). Trois des seize essais (t-3, t-7, t-9) sont restés sur le
plateau pacifique (0 %), un (t-6) presque.

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
| s49–s55 | schools de la reprise GPU (recette s54 : de zéro, contre s46a/s48b–d/s53 figés, `--hall 0 --longest 100`, 300 gén., tirage) | s54a = 68 % contre prod (an 23), l'exception d'une campagne de 38 ; les autres finalistes 12–18 % |
| s56 | auto-jeu pur, 30 000 génomes, élite 1 000, tirage, GPU | 9 % contre prod à 100 gén., 19,5 % à 379 : transfère mal et lentement |
| s57 | de zéro contre prod + son propre Hall (`--hall 8`), 8 essais, 500 gén. | 6–20 % contre prod : pas mieux que s54 à âge égal |
| s58 | **largeur de la couche cachée** 4 à 256, 8 essais chacune, 300 gén., tirage | lignée trouvée 0/8 (4), 1/8 (8), 2/8 (16), 6/8 (32), 5/8 (64), 8/8 (128, 256) ; médianes 13–17 % partout ; à 256, deux essais à 40–43 % (plateau franchi par la queue haute) ; h32 prolongé à 600 gén. ne bouge pas |
| **s59** | **élevage** (`--breed 0.1` et `0.3`) contre tirage, h32, 8 essais, 300 gén. | 0,3 : 8/8, 29–61 %, **médiane 45 %** (33 % dès la gén. 100), sacre an 19–27 ; le plafond de 15 % était la méthode |
| s60 | recall (sans Chronique) contre told (Chronique, sans recall), élevage 0,3, 100 gén. | told 40 % de médiane, recall 26 % : la Chronique donnée d'office bat l'automate à construire, à 100 gén. |
| s61 | plan factoriel told × recall × journal sous v1 | interrompu deux fois par la boucle des béliers (400 000 béliers), abandonné pour v2 |
| **s62** | **règles v2** : pool A régénéré de zéro (8/8 lignée entre clones), puis les huit lectures contre le pool, 100 gén. | rien 16 % de médiane (max 48), recall 13, les trois 11, journal 9, Chronique seule 3 : à 100 gén. toute entrée en plus ralentit ; s62b prolonge « les trois » et le témoin à 300 |
| **s63** | **règles v3** (sans journal) : pool B de zéro (8 essais, told + recall, 100 gén., 7/8 sacrent), puis 16 essais de zéro contre B, 400 gén., 32 tables par génome | 12 essais sur 16 valides (20–36 % contre B, an 19–24), 3 sur le plateau pacifique ; **sept livrés** (§ 4) ; `promote.py` → pool C. Génération de 8 essais en 3 s, de 16 en 6,5 s |
| **s62c** | **les huit lectures à 600 gén.** contre le pool A, de zéro (`run-s62c.py`, machine GPU, 32 tables par génome), 8 essais, mesure tous les 25 | **Chronique + recall 50 %** de médiane (33–59, sacre an 21, 51 % dès 400) ; témoin 44 % (38–54), recall 44 (part à 32 % à 100 contre 22), Chronique 46 (12–55, tard et fragile) ; **le journal retire 3 à 6 points partout** (journal 41, recall+journal 42, Chronique+journal 38, les trois 43). Plafond atteint entre 300 et 400, rien ne bouge en 200 de plus. La Chronique lit 1–2 rapports par partie contre 9 |

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
9. **Le plafond à 15 % était la méthode, pas la capacité ni le temps** (13 septembre) :
   l'entropie croisée fond l'élite en une gaussienne et jette la diversité ; l'élevage la
   garde (s59 : 45 % de médiane contre 17 %). La largeur ne bouge pas la médiane (s58) mais
   rend la lignée sûre dès 128 neurones et ouvre la queue haute à 256.
10. **Comparer dans la même campagne, avec un témoin** ; les pools de référence sont figés
    et numérotés, on y fait entrer un champion à la fois. Les chiffres v1 et v2 ne se
    comparent pas (un sacre par table en v2).
11. **L'élevage envoie des éclaireurs** (4 à 30 lectures par siège et par partie) là où le
    tirage n'en envoyait jamais : le geste à deux temps (payer, puis conditionner) est ce
    qu'une lignée découvre et qu'une moyenne perd.
12. **Plus d'entrées, départ plus lent** (s62 à 100 gén.) : une école qui ne lit pas un bloc
    cherche dans un espace plus petit ; la mémoire se juge sur 300 générations.

## 6. Reprendre

```bash
# régénérer le pool sous les règles courantes, puis le plan des lectures (s62)
python3 apps/empire-train/schools/run-s62.py

# une campagne d'élevage de 8 essais contre le pool A, sur GPU (≈ 5 min pour 100 gén.)
P=apps/empire-train/schools/pools/A
target/release/empire-train --stage war $(for n in 1 2 3 4 5 6 7 8; do echo --against $P/pool-$n-war.json; done) \
  --hall 0 --longest 100 --rank 40 --tables 32 --trials 8 --breed 0.3 --told --recall \
  --generations 300 --keep 25 --gpu --out "s63-{n}-war.json"

# mesurer une école contre le pool (la seule note qui fasse foi), ou dérouler une partie
target/release/empire-train --stage war $(for n in 1 2 3 4 5; do echo --against $P/pool-$n-war.json; done) \
  --longest 100 --rank 40 --from s63-1-war.json --measure 500 --gpu
target/release/empire-train --stage war --against ... --from s63-1-war.json --show --rank 40

# faire entrer un champion dans le pool (un seul par tour)
python3 apps/empire-train/schools/promote.py $P apps/empire-train/schools/pools/B s63-*-war.json

# livrer : le meilleur génome d'une école devient un brains/<tempérament>.f32
target/release/empire-train --stage war --from s63-1-war.json --deliver libs/empire-lib/brains/soldat.f32
```

Toute modification des règles, de la vue ou du décodage incrémente `empire_lib::RULES` ; les
fichiers d'école d'une autre version sont refusés et le pool est régénéré.

## 7. Ouvert

- ~~Le recall sur une course longue~~ : tranché par s62c (600 gén.) — seul, il rejoint le
  témoin (44 %) après un départ plus rapide ; avec la Chronique il fait 50 %, la seule
  lecture au-dessus du témoin. Le journal baisse tout : à retirer de la vue (une décision
  de règles, `RULES` à incrémenter, 8 800 poids de moins).
- Élevage × largeur (128–256 neurones, où le tirage trouvait déjà des conquérants rapides).
- L'échelle de mutation (0,3 converge plus vite mais s'aplatit vers 300 gén. ; 0,1 monte
  encore), à faire décroître le long de la course.
- Le tirage sur la carte (moyenne et sigma envoyés, la population tirée dans le noyau) :
  supprime l'envoi du pool et la mémoire CPU de la première génération, nécessaire au-delà
  de 100 000 poids.
- La fenêtre glissante complète (encodeur par année) si le journal ne suffit pas ; le
  dimensionnement est dans le brouillon « Brain à fenêtre glissante » du 13 septembre.
- Livrer sept tempéraments v2 dans `brains/` et mettre `Brain::schools()` à jour — à
  scolariser en `--told --recall` (s62c), le meilleur essai (59 %) candidat au pool B.
