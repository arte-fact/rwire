# Le Renseignement — éclaireurs et agents, refonte

État : **fait** (décidé le 10 septembre 2026, avec `BATIMENTS.md` ; règles `src/intel.rs`,
arène, web `apps/empire-web`). A remplacé l'éclaireur qui rentrait l'an d'après et l'agent
entretenu à l'année. Le terminal (`apps/empire`) n'a pas de renseignement pour le joueur
humain ; ses ordinateurs envoient leur éclaireur comme dans l'arène. Les valeurs en *italique* sont des
propositions.

## 1. Le principe

Tout ce qui fait la force d'un royaume est **secret** : ses arpents, ses soldats, ses
fortifications, ses habitants, son trésor, ses greniers, ses bâtiments. On l'apprend en
envoyant quelqu'un — et il peut se faire prendre. La réponse est **immédiate**, pendant
l'Extérieur : on sait avant de décider où marcher.

Reste **public** : le nom et le titre du seigneur, son étal au marché (grain à vendre,
prix), et tout ce qui se passe **sur le champ de bataille** — l'armée envoyée, la garnison
qui s'est battue, les arpents pris, les hommes perdus, ce qui a brûlé ou été pris, les
annexions. Ce sont des informations **partielles** : on voit l'ost qui a marché, pas
l'armée restée au pays ; la garnison d'un jour, pas le royaume. C'est cohérent : on
regarde la bataille, on ne lit pas les registres.

## 2. L'éclaireur

- **Une unité** achetée à l'Intendance : **16 francs** (deux fois un soldat), gardée en
  réserve (`kingdom.scouts`).
- À l'Extérieur, sur un royaume : **Envoyer un éclaireur · 150 francs** (la mission).
  *Une mission par royaume et par an.*
- **Résolution immédiate** :
  - **1 chance sur 6** d'être **pris** : l'éclaireur est perdu, les 150 francs aussi ; on
    n'apprend rien ; **l'adversaire le sait** (journal, Chronique : « Un éclaireur de la
    Bretagne a été pris sur nos terres ») ;
  - sinon il **rentre avec son rapport**, daté de l'année : **arpents**, **soldats**
    (garnison), **fortifications** (dixièmes) et **l'efficacité de l'ost**, qu'il voit à
    l'exercice (sans elle le pronostic de bataille ne se calcule pas).
- Le rapport reste dans le dossier avec sa date ; un vieux rapport vaut ce qu'il vaut
  (pronostic à ±20 % l'an d'après, ±40 % à deux ans, rien au-delà — inchangé).

## 3. L'agent

- Pas d'unité : **Envoyer un agent · 400 francs**, à l'Extérieur, sur un royaume, *une
  fois par royaume et par an*. Plus d'agent « en place » ni de lettre annuelle.
- **Résolution immédiate**, même règle : **1 chance sur 6** d'être pris (l'adversaire le
  sait, l'argent est perdu), sinon il **lit les registres** : trésor, greniers, habitants
  (nobles · marchands · serfs), bâtiments (foires, moulins, fonderies, chantiers, palais,
  hospice), le titre visé et les critères atteints, les achats de grain de l'année.

## 4. Ce que ça change ailleurs

- **Ce qu'on ignore n'est pas affiché** — jamais de `?`. L'écran est déjà chargé ; une
  case absente veut dire qu'on ne sait pas.
- **Extérieur** : la ligne d'un royaume n'a que son nom, son titre et les rumeurs tant
  qu'aucun éclaireur n'a rapporté ; avec un rapport, une ligne de chiffres datée. Les
  badges « éclaireur ✓ » / « agent en place » deviennent le résultat de l'année
  (« éclaireur rentré », « ✕ éclaireur pris »).
- **Fiche du royaume** : sans rapport, les deux boutons de mission et les rumeurs ; avec,
  les blocs « Ce qu'a vu l'éclaireur » et « Ce qu'a lu l'agent », datés.
- **Campagne** : inchangée — la bataille reste publique (garnison du jour, hommes envoyés,
  murs abattus par les béliers, butin). Le schéma montre les fortifications du défenseur
  **au moment de la bataille** (elles tombent sous les béliers, tout le monde le voit).
- **Chronique / journal** : « Votre éclaireur est rentré de Castille » / « … a été pris en
  Castille » / « Un éclaireur de la Bretagne a été pris sur nos terres » ; idem agent.
- **Brain** : la surface « entendue » (arrondie aux 500 dès l'an 3) a disparu de la vue —
  ce serait tricher ; à la place, la surface et les fortifications du dernier rapport
  d'éclaireur. Le retour immédiat se lit à l'Extérieur **en deux temps** : `missions`
  (éclaireur et agents), puis `expeditions` avec les rapports frais. C'est la
  rescolarisation s27 de `BRAIN.md`, avec les bâtiments.
- Ce qui saute : `Mission` résolu l'an d'après, `Writing::Letter` annuelle, le
  « rappel » de l'éclaireur, l'agent entretenu.

## 5. Maquettes

Maquettes HTML à la largeur de la colonne de jeu : `renseignement-maquettes.html`
(artefact de session). En texte :

```
EXTÉRIEUR · an 7                              1 éclaireur en réserve
─────────────────────────────────────────────────────────────────
Castille · Isabelle · ordinateur                        Prince
─────────────────────────────────────────────────────────────────
Germanie · Otto                                  11 200 arpents
84 ⚔ · murs 30 % · an 7               [rapport de l'an 7]  [60 ✓]
   ⚔ l'an dernier, la Germanie a marché sur la Perse avec 40 …
─────────────────────────────────────────────────────────────────
Perse · Darius                                             Duc
                                           [✕ éclaireur pris]
```

```
FICHE · Germanie · Otto                     d'après votre éclaireur de l'an 7
  Ce qu'a vu l'éclaireur
  Terres        11 200 arpents · an 7      Garnison       84 ⚔ · an 7
  Fortifications 30 % · an 7              Efficacité     92 · an 7
  L'agent — lit les registres : trésor, greniers, habitants, bâtiments, titre visé
  [Envoyer un agent · 400 francs · 1 chance sur 6 d'être pris]
  Pronostic : 60 hommes → 180–420 arpents, 12–25 perdus, murs 30 % → 10 %
  Béliers  [ 0 | 1 | 2 ]  (2 en réserve · 10 soldats par bélier)
  [Marcher avec 60 hommes et 2 béliers]
```

```
BATAILLE · Bretagne → Germanie                          échange 12
  Bretagne      ████████░░  48 / 60      2 béliers · coup dans 2 échanges
  Germanie      ██████░░░░  51 / 84      murs ██████░░░░ 60 %  (×1,6)
  ✕ un bélier détruit (6 soldats par bélier · 40 %)
```

```
VERDICT · VICTOIRE · 310 arpents pris · 18 perdus
  Murs abattus : 4 dixièmes (100 % → 60 %) · 1 bélier perdu, 1 rentré
```
