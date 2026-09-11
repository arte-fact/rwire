# Fortifications, béliers, hospice

État : **fait** (décidé le 10 septembre 2026, avec `RENSEIGNEMENT.md`) : règles d'`empire-lib`,
web (`apps/empire-web`) ; au terminal (`apps/empire`), fortifications et hospice à l'Intendance,
mais pas de béliers — son moteur `war.rs` n'a pas de siège. Les
valeurs en *italique* sont des propositions acceptées par défaut, à ajuster au jeu.

## 1. Fortifications

- Achetées par **dixièmes**, comme le palais : **5 000 le dixième**, plafond 10
  (`kingdom.fortifications` en dixièmes).
- **Bonus de garnison** : la garnison d'aujourd'hui vaut 100 % ; chaque dixième ajoute
  10 %. À 100 % de fortification la garnison se bat à **200 %** — le multiplicateur va
  de 1 à 2 par crans de 0,1.
  `garrison_strength(eff, tenths) = eff × 3/2 × (10 + tenths) / 10`
  (le ×3/2 est le bonus de murs actuel, conservé ; il devient le 100 %).
- Les fortifications ne servent qu'à la défense : elles ne marchent pas.

## 2. Béliers

- Achetés à l'Intendance, gardés en stock (`kingdom.rams`) ; *prix : 2 500 le bélier*.
- Chaque expédition contre un royaume emmène **0 à N béliers** (nouveau champ de l'ordre ;
  inutiles contre les barbares, qui n'ont pas de murs). On connaît les murs de
  l'adversaire par son éclaireur (`RENSEIGNEMENT.md`) — ou on marche à l'aveugle.
- Les béliers travaillent **pendant le siège**, pas avant. La bataille se joue en
  échanges (à chaque tour, chaque armée tire un duel contre la garnison) ; **tous les
  `RAM_PACE` échanges** (*4*), chaque bélier encore debout donne un coup :
  - le coup **casse un dixième** de fortification, **durablement** (à racheter) ; la
    garnison se bat aussitôt avec le bonus réduit (×2 → ×1,9 → … → ×1) ;
  - au moment du coup, le bélier prend son risque : **escorte = soldats encore debout de
    l'armée ÷ béliers** ; à 10 par bélier il est **invulnérable**, en dessous il a **10 %
    de chances d'être détruit par soldat manquant** (6 par bélier → 40 %, 0 → perdu à
    coup sûr). Une armée qui fond expose ses béliers au fil de la bataille.
  - plusieurs armées sur un front : leurs béliers frappent chacune à leur tour, sur les
    mêmes murs, dans l'ordre du schéma.
- **Garnison tombée** : les béliers ont fini ; la marche dans le royaume se fait comme
  aujourd'hui, mais elle commence plus tôt et avec plus d'hommes — c'est là que le
  deux-contre-un disparaît.
- Les béliers survivants rentrent avec l'armée ; perdus si elle est anéantie. Devant un
  royaume sans murs, un bélier ne casse rien mais prend quand même ses risques à chaque
  coup.
- Replay : une jauge de murs à côté de celle de la garnison, la ligne des béliers
  (« 2 béliers · coup dans 2 échanges », « ✕ un bélier détruit ») ; verdict et
  Chronique : « murs abattus : 4 dixièmes · 1 bélier perdu ».
- Ordre de grandeur : une garnison de 100 face à 60 hommes tient ~40 échanges ; à un coup
  tous les 4, un bélier seul rase des murs à 100 % sur toute la bataille, trois béliers en
  un tiers de bataille.

## 3. Hospice

- Par **dixièmes**, 5 000 le dixième, plafond 10 (`kingdom.hospices`).
- Chaque dixième réduit de **5 %** les morts de **maladie** et de **peste** : −50 % à 100 %.
  `victimes × (20 − dixièmes) / 20`, sur la maladie ordinaire (`disease_cap = pop / 22`)
  et sur la part que la peste prend à chaque classe.

## 4. Couronne

*Un minimum par titre, palais en rappel (2 / 6 / 10) :*

| Titre | Fortifications | Hospice |
|---|---|---|
| Prince | 1 | 1 |
| Roi | 3 | 3 |
| Empereur | 5 | 5 |

Deux exigences de plus dans `Requirements` / `Requirement::ALL` (9 au lieu de 7) — donc
dans la vue du brain (`criteria`) et dans `progress` de l'arène.

## 5. L'ordinateur

Le brain s26 ne connaissait ni ces achats ni ces exigences ; le temps de la rescolarisation,
un réflexe d'intendant hors réseau lui achetait le dixième manquant. Il a disparu avec le
brain s27 (`BRAIN.md`) : sa vue porte les murs, l'hospice et les béliers en stock, ses
achats vont à `Fortifications`, `Hospices` et `Rams` comme au palais, et sa première
expédition contre un royaume emmène la part de béliers qu'il décide (0 à tous ceux en
réserve). Le terminal, dont le moteur n'a pas de siège, laisse les béliers au pays.

## 6. Où ça touche

- `investments.rs` : `InvestmentType::{Fortifications, Hospices, Rams}`, coûts, achat par
  dixième (comme `Palaces`).
- `kingdom.rs` : les trois champs, `Requirements` + `Requirement::{Fortifications, Hospices}`.
- `front.rs` / `campaign.rs` : béliers dans l'ordre d'expédition, leurs coups au fil des
  échanges (`Round` gagne les murs et les béliers), `garrison_strength` recalculé à chaque
  coup, retour des béliers.
- `demography.rs`, `events.rs` : réduction des victimes.
- `brain.rs` : `criteria` à 9, murs / hospice / béliers dans la vue, trois achats de plus,
  béliers en sortie d'expédition ; `arena.rs` : `progress`.
- web : Intendance (achats), Extérieur (béliers par expédition), fiche de guerre et replay
  (murs cassés, béliers perdus), journal / Chronique ; terminal : fortifications et hospice
  à l'Intendance (7 et 8), sans béliers.
