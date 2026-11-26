# Proposition : etat-de-session

## Why

Depuis `vue-table`, l'utilisateur peut trier la table, la filtrer, élargir et déplacer ses colonnes, et redimensionner la fenêtre ; mais tout est perdu à la fermeture, et l'application repart à chaque fois de `app.json`. Ce changement enregistre cette disposition à la fermeture et la restaure à l'ouverture, dans un fichier d'état distinct, sans jamais toucher à `app.json`.

C'est le troisième et dernier changement de l'**incrément 2** de `docs/feuille-de-route-annuaire.md`, après `captures-images` et `vue-table`. Il remplit le dernier critère « Fini quand » de l'incrément : **la disposition est retrouvée au redémarrage**. Les trois autres (`app.json` affiche les personnes, 50 000 personnes défilent de façon fluide, la plateforme des captures d'images est choisie) l'ont été par les deux changements précédents.

Le principe vient de la feuille de route : `app.json` est la définition de l'application ; la disposition choisie à l'usage est un état d'utilisation, qui n'a pas sa place dans ce fichier, sinon chaque utilisation le modifierait. En mode run, `app.json` n'est jamais écrit ; seul un futur mode design le fera.

## What Changes

- **État de session** (`ortelia-presentation`) : à la fermeture de la fenêtre, Ortelia enregistre, pour l'application ouverte :
  - la taille de la fenêtre, quand elle n'est ni maximisée, ni en plein écran, ni réduite ;
  - pour chaque vue table : l'ordre et la largeur des colonnes, le tri (colonnes et sens) et le texte du filtre.
- **Fichier d'état à côté de `app.json`** : `app.session.json`, dans le dossier de l'application. Plus généralement, le nom du fichier de l'application dont l'extension `.json` devient `.session.json` (`ortelia dossier/annuaire.json` donne `dossier/annuaire.session.json`). L'écriture passe par un fichier temporaire renommé, pour qu'une fermeture interrompue ne laisse pas un fichier tronqué. Un échec d'écriture (dossier en lecture seule…) est journalisé et n'empêche pas la fermeture.
- **Restauration** à l'ouverture suivante : l'état enregistré remplace la taille de `window` et la disposition de la présentation de la table. Il est restauré avec indulgence, puisque `app.json` a pu changer entre-temps :
  - une colonne, une vue ou un critère de tri enregistrés qui n'existent plus sont oubliés ;
  - une colonne de la présentation absente de l'état enregistré est ajoutée à la fin, avec sa largeur de `app.json` ;
  - les largeurs et la taille de la fenêtre sont ramenées dans leurs bornes ;
  - un fichier illisible, ou d'une autre version de format, est ignoré (avertissement dans le journal) : l'application s'ouvre avec la disposition de `app.json`, et le fichier est remplacé à la fermeture.
- **`.gitignore`** : les fichiers `*.session.json` sont ignorés, pour qu'un lancement de `examples/annuaire` ne produise pas de fichier à versionner.
- **Confidentialité** : le texte du filtre est enregistré dans le fichier d'état, mais n'apparaît jamais dans le journal.

### Hors périmètre

- Un état **par utilisateur**, dans son dossier personnel (`~/.local/state`, `%LOCALAPPDATA%`) : pour l'instant, l'état est celui du dossier de l'application, partagé par qui l'ouvre. Il pourra déménager plus tard sans changer de format.
- La **position** de la fenêtre : elle n'est pas connue sous Wayland (WSLg compris), et sa restauration sur un écran débranché demande des précautions. Seule la taille est enregistrée.
- L'état **maximisé** ou plein écran : la fenêtre rouvre à la dernière taille normale.
- La langue choisie (incrément 4) et la position de la séparation d'un `Split` (incrément 3) : le format du fichier est prévu pour les recevoir.
- Un enregistrement en cours d'utilisation : seule la fermeture normale de la fenêtre enregistre ; un arrêt brutal perd les changements de la session.
- Plusieurs instances de la même application ouvertes en même temps : la dernière fermée écrase l'état des autres.
- Une commande pour revenir à la disposition de `app.json` : il suffit de supprimer `app.session.json`. Un menu viendra avec le menu de configuration de l'incrément 4.

## Capabilities

### New Capabilities
- `session-state` : l'état de session d'une application (taille de la fenêtre, disposition des tables), son fichier à côté de `app.json`, son enregistrement à la fermeture et sa restauration indulgente à l'ouverture.

### Modified Capabilities
- `table-view` : le tri initial, l'ordre et la largeur des colonnes viennent de l'état de session quand il y en a un, avant ceux de la présentation.
- `app-launch` : la taille initiale de la fenêtre vient de l'état de session quand il y en a un, avant celle de `window`.

## Impact

- `ortelia-presentation` : nouveau module `session` (format du fichier, lecture, écriture, restauration) ; `AppState::open_with_session`, `AppState::save_session`, intention `WindowResized` ; `TableView` capture et restaure sa disposition. Nouvelles dépendances : `serde` et `serde_json`, déjà dans le workspace. `ortelia-config` : `SortOrder` devient sérialisable.
- `ortelia-ui-egui` : intention de redimensionnement de la fenêtre à chaque changement de taille ; enregistrement dans `on_exit` d'eframe.
- `ortelia` : ouverture avec état de session.
- Tests : la présentation (capture, restauration, cas dégradés, lecture et écriture dans un dossier temporaire), egui_kittest (intention de redimensionnement, enregistrement à la fermeture puis restauration). Les tests de l'interface et les captures d'images ouvrent toujours les applications sans état de session, pour ne pas dépendre d'un `app.session.json` laissé par un lancement manuel.
- Dépôt : `.gitignore` (`*.session.json`).
- Documents : `CLAUDE.md` (état du projet, fichier d'état), `docs/feuille-de-route-annuaire.md` (emplacement du fichier d'état, incrément 2 terminé).
