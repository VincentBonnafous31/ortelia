# Proposition : captures-images

## Why

Les tests de l'interface vérifient aujourd'hui l'arbre d'accessibilité (egui_kittest sans rendu) et le démarrage de la vraie fenêtre, mais rien ne vérifie le rendu lui-même : une colonne décalée, un texte coupé ou une couleur changée passent inaperçus. C'est le niveau 3 de la stratégie de tests de l'interface (`CLAUDE.md`), reporté jusqu'ici : comparer l'image rendue à une image de référence.

Il faut d'abord choisir où ces images sont rendues, car le rendu doit être identique d'une exécution à l'autre : une machine macOS de la CI, avec un vrai GPU, ou le rendu logiciel lavapipe sous Linux, qu'on trouve aussi dans WSL2 sur le poste de développement. Le choix décide de la façon de mettre à jour les images de référence : en local, ou seulement dans la CI.

Ce changement est le premier des trois changements de l'**incrément 2** (vue avec une table) de `docs/feuille-de-route-annuaire.md` ; suivront la table (`vue-table`), puis l'état de session (`etat-de-session`). Il remplit le critère « Fini quand » **la plateforme des captures d'images est choisie**. Les autres critères de l'incrément (`app.json` affiche les personnes ; 50 000 personnes défilent de façon fluide ; la disposition est retrouvée au redémarrage) reviennent aux deux changements suivants. Mené avant eux, l'essai permet au changement de la table de figer son rendu dans ses propres tâches.

## What Changes

- **Tests de capture** dans `ortelia-ui-egui`, avec egui_kittest 0.37 et ses features `wgpu` et `snapshot` : le rendu d'une image de l'interface est comparé à une image de référence PNG versionnée, avec un seuil de tolérance par pixel et un nombre de pixels fautifs tolérés, réglés dans un fichier `kittest.toml` à la racine du dépôt.
- **Une première capture** : la fenêtre principale actuelle, à taille, échelle et thème fixés. Le changement `vue-table` la complétera par la table.
- **L'essai** :
  - les images de référence sont produites dans WSL2 (Ubuntu 26.04, lavapipe) ;
  - les mêmes tests tournent contre elles dans la CI, sur quatre configurations : Linux (runner Ubuntu, lavapipe), Linux dans un conteneur Ubuntu 26.04 (la même version de Mesa que WSL2), macOS (Metal, vrai GPU) et Windows (WARP, rendu logiciel de Direct3D 12) ;
  - on relève, pour chacune, le nombre de pixels fautifs selon le seuil, que donne egui_kittest, ainsi que les images de différence, et on évalue ce que coûte la mise à jour des références ;
  - une scène d'essai, réservée à l'essai, rend plus de widgets que la fenêtre principale (textes, grille rayée, cases à cocher, boutons, champ de texte, barre de défilement), pour que la mesure représente les tables et les formulaires à venir ;
  - le compte rendu va dans `docs/essais/captures-images.md`.
- **Le résultat appliqué** (attendu : lavapipe sous Linux, à confirmer par les mesures) :
  - les tests de capture tournent sur la plateforme retenue, de façon bloquante, dans le job `checks` de la CI et dans `tools/check.sh` ; ailleurs, ils sont marqués ignorés ;
  - les paquets du rendu logiciel sont installés avant les tests dans les jobs Linux qui exécutent les tests (`checks` et `coverage`) ;
  - en cas d'échec, les images `*.new.png` et `*.diff.png` sont publiées comme artefact de l'exécution ; elles sont ignorées par git ;
  - la procédure de mise à jour (`UPDATE_SNAPSHOTS=1`) est décrite dans `CLAUDE.md` ;
  - la scène d'essai et le workflow de comparaison sur les quatre configurations sont supprimés, comme la variante écartée de l'essai S1.

### Hors périmètre

- La capture de la table : changement `vue-table`. Celle du formulaire : incrément 3.
- Les captures sous plusieurs thèmes, échelles ou langues : une seule configuration par capture pour l'instant.
- Les tests de capture sous Windows et macOS de façon permanente, sauf si l'essai désigne l'une de ces plateformes.
- Le test de fumée et les tests egui_kittest sans rendu, inchangés.
- Le stockage des images par Git LFS : quelques images de petite taille n'en ont pas besoin.

## Capabilities

### New Capabilities

- `ui-snapshots` : tests de capture de l'interface : comparaison du rendu à une image de référence versionnée, tolérances, plateforme de rendu unique, capture de la fenêtre principale.

### Modified Capabilities

- `continuous-integration` : les tests de capture tournent sur la plateforme retenue, avec le rendu logiciel installé avant les tests, et les images d'un échec sont publiées comme artefact.

## Impact

- **Code** : `ortelia-ui-egui` (tests de capture de la fenêtre principale, scène d'essai temporaire).
- **Nouveaux fichiers** : `kittest.toml` (racine), `crates/ortelia-ui-egui/tests/snapshots/*.png` (références), `docs/essais/captures-images.md` (compte rendu), un workflow temporaire de comparaison sur les quatre configurations.
- **Fichiers modifiés** : `.github/workflows/ci.yml` (paquets avant les tests, artefact des images d'un échec), `.gitignore` (images `*.new.png`, `*.diff.png`, `*.old.png`), `.gitattributes` (PNG binaires), `CLAUDE.md` (niveau 3 de la stratégie de tests, mise à jour des références), `docs/feuille-de-route-annuaire.md` (résultat de l'essai).
- **Dépendances** : egui_kittest 0.37.0, déjà présent, avec ses features `wgpu` et `snapshot` en plus de `eframe`, en dépendance de développement de `ortelia-ui-egui` seulement. Elles ajoutent les crates `image` et `dify` (comparaison d'images) ; wgpu est déjà compilé pour eframe. L'exécutable n'en dépend pas.
- **Poste de développement** : lavapipe est déjà installé dans WSL2 (`mesa-vulkan-drivers`, Mesa 26.0.8).
