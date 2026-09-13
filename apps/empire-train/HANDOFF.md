# Passation — l'école d'Empire sur la machine GPU (13 septembre 2026, soir)

Pour l'instance de Claude qui reprend sur `threadreaper` (RTX 3090, 32 threads, 31 Go).
Tout est en français avec l'utilisateur. Ce fichier remplace la mémoire de session de
l'instance précédente ; `libs/empire-lib/BRAIN.md` est la référence de fond.

## 1. Où en est le code

- Branche **`empire-v2`** (commits `d2dcf0b` puis `251161d`), présente sur le GitLab
  (`gitlab.raptors.pizza/raptor-1/rwire`) et dans le dépôt local `~/rwire` (origin =
  GitHub, sans cette branche : on pousse par SSH ou on tire depuis GitLab). `main` n'a rien
  de tout ça.
- **Règles v2** (`empire_lib::RULES = 2`) : achats = envie⁴ × ce que le trésor permet ;
  table finie au premier Empereur ; au plus un bélier par dixième de mur monte à l'assaut
  par cadence ; la vue porte le journal de 4 ans (public + rapports d'éclaireur du siège).
  Un fichier d'école d'une autre version est **refusé** (`load` panique) ; les sept `.f32`
  livrés dans `libs/empire-lib/brains` sont de la v1, le jeu tourne avec eux mais ils
  achètent peu : **à régénérer et livrer** (voir § 5).
- Trainer `apps/empire-train` : `--breed 0.3` (élevage : élite = 100 parents, 9 enfants
  chacun, sélection parmi tous), `--trials N` (N écoles dans une passe GPU, `{n}` dans
  `--out` et `--from`), `--hidden H`, `--told --recall --journal` (ce que lit une école de
  zéro ; par défaut rien), `--keep N` (instantanés `<out>.g<gen>.json`), `--measure N --gpu`
  (un siège contre cinq des `--against`, la seule note qui fasse foi), `--show`,
  `--deliver`. L'en-tête imprime la carte choisie.
- GPU : `libs/empire-gpu`, noyau WGSL une table par voie, parité CPU/GPU vérifiée par
  `cargo test -p empire-gpu --release` (0 année divergente sur ~640). `Gpu::open` prend la
  carte discrète ; si jamais un autre pilote Vulkan répond, forcer
  `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json`. Passes de 50 000
  table-années (× 32 / hidden) pour rester sous le délai du pilote ; une carte
  réinitialisée se voit à `the GPU came back with zeroed tables`.
- Clippy propre, `cargo fmt --all`, `cargo test --workspace` verts au dernier commit.

## 2. Ce qu'on sait (le résumé qui compte)

1. **Le plafond à 15 % contre les rivaux était la méthode** : l'entropie croisée (tirage
   autour d'une moyenne) jette la diversité ; l'élevage la garde. s59 : 45 % de médiane à
   300 gén. contre 17 %, sacre an 19–27 ; 33 % dès la gén. 100.
2. **La largeur** ne bouge pas la médiane (s58, 4 à 256 neurones) mais rend la lignée des
   couronnes sûre dès 128 et ouvre la queue haute à 256 (40–43 %). h32 prolongé à 600 gén.
   par tirage ne bouge pas.
3. **Le recall** (32 sorties relues l'an d'après) sert d'automate à 2–3 états (un bit de
   phase « assaut final » qui bascule sur un seuil de trésor et d'armée) ; coupé, s59 tombe
   de 55 à 3 %. À 100 gén. la Chronique donnée d'office fait mieux (s60 v1 : 40 contre
   26 %). L'utilisateur pense que le recall a « plus de potentiel, demande plus
   d'entraînement » : à tester sur 300–600 gén., ne pas le réduire.
4. **s62 (v2, contre le pool A)** : à 100 gén. toute entrée en plus ralentit (témoin sans
   mémoire 16 % de médiane, recall 13, les trois 11, Chronique seule 3). À 300 gén. :
   témoin 31 % (plafonné depuis 150), les trois 27 % (monte encore). Styles : témoin gros
   et muré, lecture complète légère et précoce. La prolongation des six autres lectures à
   300 était en cours sur la machine d'origine (`s62/chain.sh`, mesures dans
   `s62/measures-300-*.csv`) ; ses résultats n'ont pas encore été lus.
5. **L'élevage envoie des éclaireurs** (4 à 30 lectures par siège et par partie) ; le
   tirage n'en envoyait jamais.
6. **Comparer dans la même campagne, avec un témoin.** Les chiffres v1 et v2 ne se
   comparent pas. Les pools de référence sont figés et numérotés
   (`apps/empire-train/schools/pools/A`, 8 écoles de zéro élevées entre elles sous v2,
   sans les parents) ; `promote.py <pool> <pool suivant> <candidats…>` fait entrer **un
   champion à la fois** (le meilleur candidat contre le pool et contre ses pairs remplace
   le membre le plus faible en tournoi).
7. **3090 contre 6750 XT** : débit égal à 3 % près (≈ 19 000 tables/s à 7 génomes, ≈ 8 000
   sur un pool de 1 024), 0,3 s par essai et par génération, linéaire de 8 à 40 essais.
   Le noyau (état en mémoire privée par voie) est borné par la latence, pas par la carte.
   Ce que la 3090 apporte : 24 Go (40 essais sans lots) et un CPU deux fois plus rapide.

## 3. Comment on travaille (règles de l'utilisateur)

- Réponses en français ; rapports détaillés avec courbes (pages HTML publiées en
  artefact : générateurs `report-s5x.py` dans le scratchpad de l'instance précédente,
  à réécrire au besoin — un CSV de mesures + les logs suffisent).
- **Commit seulement sur demande explicite**, `git add -A -- . ':!.claude'`, message
  terminé par `Claude-Session: <url de session>`. **Ne jamais pousser** sans demande.
  Les génomes des campagnes (`schools/s*/*.json`) sont ignorés par git (dizaines de Mo
  chacun avec les parents) ; les pools, logs et CSV sont gardés.
- Zéro warning clippy, `cargo fmt --all`, `cargo test --workspace` avant tout commit.
  Pas de code mort, pas de chemin de compatibilité : on jette les modèles et on
  régénère (`RULES` à incrémenter à chaque changement de règle, de vue ou de décodage).
- Édition via Bash (python heredoc, sed) ; tuer un processus **par PID** (jamais
  `pkill -f`) ; pas de chaînes de `sleep`.
- **Lancer les campagnes détachées** (`nohup setsid … & disown`) : l'outil de session tue
  les tâches d'arrière-plan longues même avec 20 Go libres. Les suivre avec un `Monitor`
  qui lit les journaux.
- Mémoire : la première génération (10 000 génomes × essais) est ce qui fixe la RAM ; le
  trainer groupe les essais par tranches de 6 Go (`MEMORY_BUDGET`).

## 4. Les scripts de campagne

- `run-s62.py` : étape A (pool de zéro, 8 essais, 100 gén.) puis les 8 lectures contre le
  pool, mesures dans `s62/measures.csv`. `run-s62b.py <lectures…>` prolonge de 200 gén.
- `run-s58.py`/`run-s59.py`/`run-s60.py` : largeur, élevage, recall/told (v1, historiques ;
  leur fonction `measure` est réutilisée par les autres).
- `promote.py` : promotion d'un champion.
- Une campagne de 8 essais × 100 gén. à h32 ≈ 5 min sur la 3090 ; 300 gén. ≈ 15 min.

## 5. La suite, dans l'ordre où l'utilisateur la voit

1. Lire les mesures de la prolongation s62 (les six lectures restantes à 300), conclure
   sur la mémoire ; si rien ne dépasse le témoin, prolonger à 600 ou durcir le pool.
2. **Promouvoir** les meilleurs du témoin (46–48 % contre le pool A) dans un pool B avec
   `promote.py`, relancer contre B.
3. **Régénérer et livrer sept brains v2** dans `libs/empire-lib/brains` (tempéraments
   distincts, `--deliver`), mettre `Brain::schools()` et `BRAIN.md` § 4 à jour.
4. Élevage × largeur (128–256), l'échelle de mutation (0,3 s'aplatit vers 300 gén., 0,1
   monte encore), le recall sur course longue.
5. Pour la 3090 : le noyau « compagnie partagée par groupe de 32 » (les poids lus une
   fois par groupe) ou moins de mémoire privée par voie ; le tirage de la population sur
   la carte (moyenne et sigma envoyés) au-delà de 100 000 poids.
6. Décider du siège sacré (il joue hors score jusqu'à la fin de table ; sous v2 la table
   s'arrête, la question est close sauf pour les tables de clones).
