# La Campagne — refonte des combats

État : **implémenté** (`empire-lib/front.rs`, `empire-lib/campaign.rs`, `empire-web/room.rs`,
`empire-web/ui/campaign.rs`). Les règles ci-dessous remplacent `simulate_kingdom_battle`
(empire-lib `war.rs`) pour les fronts entre royaumes joués sur le web. Les expéditions contre
les barbares gardent la règle actuelle ; le terminal (`apps/empire`) et `ia.rs::execute_ai_turn`
restent sur l'ancienne règle. Les décisions prises à l'implémentation sont en fin de document ;
la **simplification** qui a suivi (plus de carte ni de pool de serfs : une marche avec
rencontres) est décrite dans la dernière section et prime sur les §1–5 quand ils divergent.

## 1. Une seule bataille par front

Un **front** = un royaume défenseur + toutes les armées qui marchent sur lui cette année.
Le front se résout en **un seul calcul**, pas une simulation par expédition sur une copie du
défenseur.

- Le défenseur est **un pool unique** : sa garnison (soldats, efficacité `soldiers_efficiency`),
  puis ses **serfs** (efficacité fixe 50) une fois la garnison tombée.
- Un **tour de bataille** : chaque armée attaquante encore debout joue une **manche** contre le
  pool, dans l'ordre du schéma (haut → bas ; à trancher : ordre aléatoire à chaque tour ?).
  - manche : `random(1, att_i) >= random(1, def)` — `def` est l'efficacité du pool *à cet
    instant* (garnison ou serfs) ;
  - manche gagnée → le pool perd `tu_i` hommes (`tu_i = envoyés_i / 15 + 1`), l'attaquant tire
    un **butin** (§3) — ou rien tant que la garnison tient (§2) ;
  - manche perdue → l'attaquant *i* perd `tu_i` hommes.
- La bataille dure tant qu'il reste **au moins un attaquant** et **des défenseurs**
  (garnison + serfs > 0).
- Chaque coup porté profite aux attaquants suivants : le pool fond au fil du tour.

## 2. La garnison défend la terre

Tant que la garnison se bat, **rien n'est pris** : pas d'arpents, pas de butin. La garnison
protège le pays.

- **Garnison tombée à 0** ⇒ **Victoire** pour tous les attaquants encore en vie, acquise quoi
  qu'il arrive ensuite. Le combat **enchaîne aussitôt sur les serfs** (efficacité 50) et va
  **jusqu'au bout** : soit les serfs tombent à 0 (§4 annexion), soit l'armée y meurt en
  emportant son butin.
- **Attaquant réduit à 0 avant la chute de la garnison** ⇒ **Anéanti**, il repart sans rien
  (cohérent : rien n'était pris). L'ancien « butin de défaite » (terrain ÷ random(1,4)) disparaît.
- **Attaquant réduit à 0 après la chute de la garnison** ⇒ verdict **Victoire**, il garde tout
  le butin déjà tiré ; le compte de « hommes perdus » dit le prix payé.

## 3. La carte à une dimension

Un royaume est vu comme **une ligne d'arpents**, de l'arpent 1 (la frontière) à l'arpent `N`
(la capitale). Tout ce que possède le royaume est **posé sur cette ligne** au début du front :
serfs, blé, or, marchands, nobles, moulins, fonderies, chantiers, champs de foire, palais.

- Le défenseur tient le segment `[conquis+1, N]`. Tant que la garnison se bat, la ligne ne bouge
  pas (§2).
- Chaque manche gagnée contre les serfs fait **avancer** l'attaquant de la manche de
  `random(1, tu×26) − random(1, tu+5)` arpents (le seul tirage qui subsiste). Le segment gagné,
  **avec tout ce qui est dessus**, lui revient : il ramasse ou détruit ce qu'il rencontre.
- Le butin n'est plus tiré au sort ni calibré : prendre 10 % de la terre rapporte mécaniquement
  ~10 % de ce qui y était. Un royaume riche en blé cède du blé, un royaume à 12 moulins perd des
  moulins.
- Ce que le défenseur perd est exactement ce qui n'est plus sur `[conquis+1, N]`. Plus de
  `collateral_damage`, plus de seuil de « grosse bataille » : une grosse bataille est une longue
  avancée.

### Répartition sur la ligne (à trancher)

| Bien                         | Proposition                                                   |
|------------------------------|---------------------------------------------------------------|
| serfs, blé, marchands        | uniforme sur toute la ligne                                   |
| moulins, foires, fonderies, chantiers | uniformes (un bâtiment tous les `N/n` arpents)       |
| palais, trésor, nobles       | **concentrés vers la capitale** (fond de ligne) : l'or et les nobles ne tombent que sur une percée profonde ; une annexion complète ramasse tout |

**Tranché** : gradient vers la capitale. Les raids courts sont pauvres en or, les percées
profondes très rentables.

### Ramassé ou détruit

| Rencontré           | Sort proposé                                                              |
|---------------------|---------------------------------------------------------------------------|
| terre               | conquise (règle actuelle)                                                 |
| serfs du segment    | passent au conquérant avec la terre (comme l'annexion actuelle)           |
| marchands           | passent avec la terre                                                     |
| blé, or             | **ramassés** par le pillard                                               |
| bâtiments           | **tranché** : deux **pris** pour un **brûlé** — chaque bâtiment franchi est pris avec probabilité 2/3, brûlé avec probabilité 1/3 (tirage indépendant par bâtiment) |
| nobles              | tués, ou faits prisonniers (rançon = or) ?                                |
| palais              | annexé avec la capitale, ou brûlé ?                                        |

## 4. Plusieurs attaquants : une ligne par attaquant

Le pool défenseur est commun (§1 : un seul calcul, garnison puis serfs), mais le royaume est
**réparti entre autant de lignes que d'attaquants** : terre, serfs, blé, or, marchands, nobles,
bâtiments — chaque ligne reçoit sa part, avec son propre gradient vers la capitale.

- Clé de répartition (**tranché**) : **à parts égales** entre les attaquants, quel que soit
  l'effectif envoyé (France 30 / Germanie 15 ⇒ ½ · ½). Une grosse armée n'a pas une plus grande
  ligne : elle avance plus loin sur la sienne.
- Chaque attaquant **avance sur sa ligne** et ne prend que ce qui est sur elle. Plus de partage a
  posteriori (`share()` disparaît), plus de segments alternés.
- Ce qu'un attaquant n'a pas pris de sa ligne reste au défenseur ; un attaquant anéanti avant la
  chute de la garnison laisse sa ligne intacte.
- Annexion complète = toutes les lignes prises (ou serfs à 0) ; le royaume est déjà réparti.

## 5. Mise en scène séquentielle (UI, `ui.rs` / `room.rs` ticker)

Tout est calculé d'un coup côté serveur ; la campagne est **rejouée** front par front,
**automatiquement**, sans bouton pendant le déroulé :

1. **Ordre de bataille** seul (le grand schéma) — le prochain front **clignote** (~2 s).
2. **Carte du front** seule : la **ligne d'arpents** (une par attaquant) vue par une **fenêtre
   glissante** qui suit le front (**15 % de la ligne**, proportionnelle au royaume — tranché ; le
   front au tiers gauche, la ligne défile sous lui) ; les bâtiments sont des **pictos SVG** posés à leur arpent, la bande dorée sous la piste
   dit la densité trésor/nobles (gradient) ; une vue d'ensemble du royaume situe la fenêtre et le
   terrain pris ; au-dessus, les jauges
   d'effectifs — une par attaquant + une pour le pool défenseur (garnison puis serfs, changement
   de couleur au basculement) — animées manche par manche.
3. **Verdict** affiché sur la carte, **pause de lecture** (~3 s ou proportionnelle au nombre
   d'attaquants) : Victoire/Anéanti par attaquant, hommes perdus, serfs tombés, arpents/blé/or
   pris, bâtiments brûlés, annexion.
4. Retour au schéma : ce front est tranché, le suivant clignote → 2.
5. Dernier front tranché → schéma complet avec tous les verdicts + **Continuer** (unique bouton ;
   l'an tourne quand tous les seigneurs vivants ont continué — déjà implémenté, non déployé).

Ordre des fronts : celui du schéma. Un spectateur qui arrive en cours de route voit l'étape
courante (le ticker porte l'état `front courant / phase / manche`).

## 6. Résumé des dégâts — à vérifier

Contrôler que chaque surface rend compte d'une grosse bataille, côté attaquant **et** défenseur :

- verdict du front (carte) ; Chronique de l'an suivant (« La Perse a marché sur vous… ») ;
  Journal ; cartes de royaumes (effectifs, terre).
- Nouveaux éléments à faire apparaître : arpents, blé, or, serfs, marchands, nobles, bâtiments
  pris/brûlés/cédés, garnison balayée puis serfs entamés, annexion. Aujourd'hui
  `collateral_damage` est calculé mais jamais montré dans la Chronique web (à confirmer en
  relisant `ui.rs`) ; il disparaît au profit du contenu des segments pris.

## 7. Impacts code

- `empire-lib/war.rs` : nouvelle `simulate_front(defender, &[(attacker, sent)])` → `FrontResult`
  (la ligne d'arpents avec ses biens ; par attaquant : verdict, restants, segments pris et leur
  contenu ; défenseur : garnison/serfs restants, ce qui reste sur la ligne, annexion) ;
  `collateral_damage` supprimé ;
  `simulate_kingdom_battle` et `defeat_spoils` supprimés si plus appelés (IA terminal `apps/empire`
  à migrer aussi, ou à laisser sur l'ancienne règle ? à trancher).
- `empire-lib/campaign.rs` : `march()` groupe les expéditions par défenseur, un `fight()` par front,
  `share()` supprimé, `apply_battle()` transfère le contenu des segments (terre, serfs, blé, or,
  marchands, bâtiments) et retire du défenseur ce qui a été brûlé.
- `empire-web/room.rs` : ticker de campagne piloté par `(front, phase, frame)` au lieu des
  `BATTLE_FRAMES` par expédition.
- `empire-web/ui.rs` : schéma clignotant, carte multi-jauges, verdicts enrichis, Chronique.
- Tests : front à 2 attaquants (pool commun, victoire commune, segments par attaquant) ; garnison
  intacte ⇒ rien pris ; la somme des segments + le reste du défenseur = le royaume de départ
  (conservation) ; annexion complète ; gradient capitale (un raid court ne touche pas au trésor).

## 8. Maquettes (UI)

Colonne de jeu (~416 px). Séquence automatique : A → B → C → A' → B → C … → D.
Couleurs : chaque royaume a sa teinte (Bretagne rouge, Castille jaune, Moscovie orange,
Perse gris…) ; « VOUS » en badge doré comme aujourd'hui.

### A — Ordre de bataille, le prochain front clignote

```
La Campagne                      An 4 · 3 expéditions · 3 fronts · 0 tranché

ORDRE DE BATAILLE                                    les armées se battent

BRETAGNE  VOUS   ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─►  ▌CASTILLE ▐        ◄ clignote
20 hommes                                    13 en garnison
                                             10 453 arpents

MOSCOVIE   ╲                            ╱   BRETAGNE  VOUS
 7 hommes   ╲                          ╱    2 065 en garnison (serfs)
             ╲ ╱                      ╱     10 219 arpents
              ╳
PERSE        ╱ ╲                     ╱      PERSE
31 hommes   ╱   ╲───────────────────╱       3 en garnison
                                            10 247 arpents

                 ● Front 1/3 · Bretagne marche sur la Castille
```

- Le trait de la bataille à venir est **animé** (tirets qui défilent vers la cible) ; la cible
  pulse (opacité 100 % ↔ 60 %). Les autres traits sont éteints (gris).
- Ligne de statut en bas (là où se trouve aujourd'hui « Chaque bataille se joue sous vos yeux »).
- Durée : ~2 s, puis bascule sur B.

### B — La bataille : effectifs et carte à une dimension

```
FRONT 1/3                                          Bretagne → Castille

BRETAGNE  VOUS                  ⚔                          CASTILLE
 14 / 20 hommes                                 garnison  4 / 13
 ████████████████░░░░░░░░                       ██████░░░░░░░░░░░░░░
                                                serfs   2 310
                                                ████████████████████

LA CASTILLE                                      10 453 arpents · tout tenu
frontière ─────────────────────────────────────────────────── capitale
│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│
   ⚙       ⚙   ▲     ⚙    ⌂         ⚙     ▲      ⚙   ⌂       ♜♜ ♛
```

Puis, garnison tombée (bascule marquée : la jauge garnison s'éteint, la jauge serfs devient la
cible, un flash « La garnison castillane est balayée — Victoire ») :

```
BRETAGNE  VOUS                  ⚔                          CASTILLE
 11 / 20 hommes                                 garnison  0 / 13   ✗
 ████████████░░░░░░░░░░░░                       ░░░░░░░░░░░░░░░░░░░░
                                                serfs   2 106 / 2 310
                                                ██████████████████░░

LA CASTILLE                                  9 611 arpents · 842 pris
frontière ─────────────────────────────────────────────────── capitale
│▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░│
   ⚙       ⚙   ▲     ⚙    ⌂         ⚙     ▲      ⚙   ⌂       ♜♜ ♛
   ·       ·   ·     ·    ·          ·     ·      ·   ·
 pris    pris  ✗    pris brûlé      pris  pris   pris brûlé

BUTIN                          +842 arpents · +178 serfs · +2 140 bx · +96 livres
                               ⚙ 5 moulins pris · ⌂ 1 foire brûlée · ▲ 1 fonderie brûlée
```

- La ligne d'arpents est à l'échelle du royaume ; les icônes sont posées aux arpents où
  sont les bâtiments (⚙ moulin, ▲ fonderie, ⌂ champ de foire, ⛵ chantier, ♜ palais, ♛ nobles/
  trésor près de la capitale). Densités (serfs, blé, or) : pas d'icône, juste le compteur BUTIN.
- Le segment pris se colore à la teinte de l'attaquant, **de la frontière vers la capitale**,
  manche après manche ; sous chaque icône franchie apparaît son sort (pris / brûlé / ✗ tué).
- Le compteur BUTIN s'incrémente au fil des manches.
- Avec plusieurs attaquants : une jauge par attaquant à gauche, et la ligne se colore **par
  segments**, chacun à la teinte de l'attaquant de la manche :

```
FRANCE                          ⚔                          BRETAGNE  VOUS
 22 / 30 hommes                                 garnison  0 / 20   ✗
 ████████████████░░░░░░░░                       serfs   1 640 / 2 065
GERMANIE                                        ████████████████░░░░
  9 / 15 hommes
 ████████████░░░░░░░░░░░░

LA BRETAGNE                                  8 903 arpents · 1 316 pris
frontière ─────────────────────────────────────────────────── capitale
│▓▓▓▓▓▓▓▓▒▒▒▒▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░│
   ▓ France 942     ▒ Germanie 374
```

### C — Verdict, pause de lecture (~3 s)

```
FRONT 1/3                                          Bretagne → Castille

VICTOIRE                                          la garnison est tombée
Bretagne  VOUS   20 hommes partis · 20 perdus · l'armée y est restée

  +842 arpents        +178 serfs        +2 140 boisseaux        +96 livres
  ⚙ 5 moulins pris    ⌂ 1 foire brûlée  ▲ 1 fonderie brûlée

CASTILLE          13 d'armes tombés · 204 serfs tombés · 9 611 arpents restants
```

Variante défaite (garnison intacte) :

```
ANÉANTIE                                          la garnison a tenu
Bretagne  VOUS   20 hommes partis · 20 perdus · pas un arpent

CASTILLE          12 d'armes tombés · 1 en garnison
```

Variante annexion (ligne entièrement prise ou serfs à 0) :

```
ANNEXION                                    la Castille n'est plus
Bretagne  VOUS   +10 453 arpents · +2 310 serfs · +23 moulins …
```

- Après la pause, retour à A' : le front 1 est éteint et porte son verdict en une ligne
  (« +842 arpents » / « anéantie »), le front 2 clignote.

### D — Fin de campagne

Le schéma A complet, tous les traits éteints, les verdicts sous chaque armée (comme aujourd'hui :
« anéantie », « +29 arpents »), et en bas :

```
                 An 4 · 3 fronts · toutes tranchées

                 [           Continuer            ]
                 L'an s'achève quand tous auront continué · France
```

### Questions de maquette

1. Icônes ou pictos SVG ? Densités (serfs/blé/or) : rien sur la ligne, ou une texture ?
2. Un front avec beaucoup de bâtiments (30 moulins) : regrouper en « ⚙×3 » par tranche
   d'arpents plutôt qu'une icône par bâtiment ?
3. Le spectateur défenseur voit-il la même chose (sa ligne qui se fait grignoter) — oui, la
   même scène, seuls les badges « VOUS » changent de côté.
4. Vitesse : une manche par frame (100 ms) rend une bataille à 200 manches trop longue ;
   comprimer à `BATTLE_FRAMES` = 40 frames par front comme aujourd'hui, la ligne avance donc par
   paquets de manches. Pause verdict 3 s. Schéma 2 s. Une campagne à 3 fronts ≈ 27 s.
5. Sur mobile étroit, la ligne d'arpents passe sous les jauges ; les icônes restent lisibles à
   ~12 px ?

## Décisions prises à l'implémentation

- **Ordre des attaquants dans une manche** : celui du schéma (première apparition de la cible
  dans les ordres, puis ordre des armées sur ce front).
- **Nobles franchis** : tués (`Spoils::nobles_killed`), pas rançonnés. **Palais** : ignoré par la
  ligne (ni pris ni brûlé).
- **Terminal et `execute_ai_turn`** : gardent `war.rs` (ancienne règle) ; seul le web passe par
  `campaign::march` + `front::simulate_front`.
- **Annexion** : le reste du royaume (ce qu'aucune ligne n'a franchi) va à l'armée victorieuse la
  plus avancée (puis la plus nombreuse) ; les serfs répartis sur les lignes sont les survivants.
- **Ligne entièrement franchie** : l'armée cesse de combattre (elle a tout pris de sa part).
- **Barbares** : chaque expédition est son propre front (règle inchangée, plafond courant sur la
  surface barbare) ; la ligne barbare est nue (ni biens ni bâtiments).
- **Rythme** : batailles rejouées l'une après l'autre, 25 à 50 frames par front (100 ms),
  schéma 2 s, verdict 3 s + 1 s par armée supplémentaire ; aucun bouton avant le dernier
  schéma (`Staging::Done` → « Continuer »).
- **Chronique** : `News::Expeditions` (butin cumulé : arpents, biens, bâtiments, pertes,
  expéditions anéanties) et `News::Attacked` (armées, repoussée/levée, garnison et serfs tombés,
  ce qui a été pris/brûlé). `collateral_damage` a disparu.

## Simplification (seconde passe)

Le pool de serfs et la carte à une dimension ont disparu ; la ligne d'arpents reste le modèle
(`front::Line`), mais elle n'est plus dessinée.

- **Garnison d'abord, inchangé** : toutes les armées la frappent ensemble, rien n'est pris tant
  qu'elle tient ; une armée à 0 avant sa chute est anéantie (rien gardé) ; les survivantes ont la
  victoire quoi qu'il arrive ensuite.
- **La marche** : garnison tombée, chaque armée avance sur sa ligne à chaque manche de
  `random(1, tu·26) − random(1, tu+5)` arpents (formule d'origine, `tu = envoyés/15 + 1`), sans
  jet contre un pool. Elle prend ce qui est au sol (arpents, blé uniforme, or croissant vers la
  capitale, bâtiments) et **rencontre** les gens qui vivent sur le tronçon franchi.
- **Habitants répartis sur la ligne** (`Line::people`) : serfs et marchands uniformément, nobles
  vers la capitale. À chaque rencontre : **1/3 se rallie** (passe à l'attaquant : `Spoils::rallied`),
  **2/3 se battent** contre un homme d'armes : `random(1, efficacité attaquant) >= random(1,
  efficacité défenseur)` (le même jet que contre la garnison) ⇒ la personne est **tuée** (`Spoils::killed`) et l'armée avance ; sinon l'homme d'armes meurt
  (**un** homme, pas `tu`), la personne survit et reste au défenseur. Même règle pour serfs,
  marchands et nobles ; la jauge « serfs » du défenseur a disparu.
- **Fin d'une armée** : 0 homme (elle s'arrête à la porte de celui qui l'a tuée : le reste du
  tronçon n'est pas franchi) ou ligne entièrement franchie. **Annexion** = toutes les lignes
  franchies jusqu'au bout ; le reliquat (arrondis, gens non rencontrés) va à l'armée la plus avancée.
- **Bâtiments** : **un pris pour deux brûlés** (tirage indépendant par bâtiment, `Building::burned`).
- **Calibrage arpents / homme** : le gain par manche est celui d'origine ; la marche coûte
  ≈ 0,2 serf/arpent × 2/3 × P(perdre le duel) hommes par arpent, soit ~15 arpents par homme à
  efficacités égales (P = 0,5) — les habitants d'un royaume mieux entraîné coûtent plus cher.
  Mesuré (200 tirages, Castille 2 000 serfs, efficacités 150 des deux côtés) : 20 hommes contre
  20 en garnison → 48 arpents en moyenne (victoire une fois sur deux) ; 20 contre 0 → 297 ;
  10 contre 0 → 143 ; 100 contre 20 → 1 184 ; 300 contre 20 → 4 150 ; 600 contre 20 → 8 372,
  jamais annexé. Une armée qui ne conquiert pas le royaume y meurt toujours (règle « fin = 0 homme »).
- **UI** (`ui/campaign.rs::march`) : par attaquant, une **barre** (arpents pris sur sa part) et
  dessous trois lignes mises à jour à chaque frame — biens (+or, +boisseaux), gens (ralliés, tués),
  bâtiments (pris, brûlés). Verdict et Chronique distinguent ralliés (« 74 serfs ralliés » /
  « passés à l'ennemi » côté défenseur) et tués (« 96 serfs tués » / « tombés »).
- **Supprimé** : `St::Picto/PictoDot/Frontline/LaneTrack/LaneDensity/OverviewFrame/BgTextMuted`,
  `At::Cx/Cy/R`, `Round::serfs`, `FrontResult::serfs_*`, `Spoils::peasants/merchants/nobles_killed`
  (→ `rallied`/`killed: People`), `News::Attacked::serfs_fallen`.
