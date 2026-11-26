# Tâches : vue-table

Toutes les commandes `cargo` se lancent dans WSL2, avec `CARGO_TARGET_DIR=~/target/ortelia` (voir `CLAUDE.md`). Chaque tâche laisse le workspace compilable, Clippy sans avertissement et les tests au vert. « Un test par scénario » renvoie aux scénarios des spécifications du changement. Consulter Context7, ou les sources des crates dans `~/.cargo/registry/src/`, avant d'écrire du code qui utilise egui, egui_extras, egui_kittest, serde, schemars ou icu_normalizer.

## 1. Format 0.4 de `app.json`

- [x] 1.1 Ajouter à `AppFile` les types de `languages`, `labels`, `presentations` et `window`, avec `Content` et `Widget` étiquetés par `_type_` (design.md, D2), et passer la version à `0.4`. Vérifier d'abord, par un test, que les clés inconnues sont refusées dans `View` et dans `Table` malgré l'étiquette interne (design.md, Risks). Passer `examples/annuaire/app.json` en 0.4, avec langues, libellés français et anglais de toutes les colonnes et de tous les littéraux, présentation de `Personne` (toutes les colonnes sauf `notes`, `date_naissance` en `dd/MM/yyyy`, tri `nom` puis `prenom`) et fenêtre de 1000 × 700. Régénérer les schémas par `cargo xtask gen-schemas`. Vérifier :
  - un test par scénario des exigences « Contenu de `app.json` », « Version du format », « Langues déclarées », « Fenêtre » et « Contenu de la fenêtre » d'`app-config` ;
  - les scénarios de `config-schemas`, dont « Contenu polymorphe de la fenêtre » ;
  - `cargo xtask gen-schemas --check` se termine avec le code 0.
- [x] 1.2 Écrire l'analyse des motifs de date (`DatePattern` : champs, texte entre apostrophes, `''`, lettres refusées, champs d'heure refusés pour une date : design.md, D3) dans `ortelia-config`. Vérifier : un test par scénario de l'exigence « Motif invalide refusé » de `value-formats`, et des tests d'analyse des motifs valides de cette spécification.

## 2. Vérification de la configuration contre le modèle

- [x] 2.1 Vérifier le chemin `data` de la vue (même parcours que les cibles, dernière référence multiple), les présentations, les colonnes, le tri et les formats ; rassembler toutes les erreurs ; ajouter à `CheckedApp` la fenêtre, la vue et la classe affichée, et les présentations indexées par classe. Vérifier : un test par scénario des exigences « Données d'une vue vérifiées contre le modèle », « Présentations vérifiées contre le modèle », « Colonnes vérifiées contre le modèle » et « Tri initial vérifié » d'`app-config`, et du scénario « Format sur un entier » de `value-formats`.
- [x] 2.2 Vérifier les libellés (langue non déclarée, groupe en double, clé en double : erreurs ; clé qui ne désigne rien : avertissement) et construire leur table de recherche (design.md, D7). Ajouter `CheckedApp::warnings` et `AppContext::warnings()`. Vérifier : un test par scénario des exigences « Déclaration des libellés », « Forme des clés », « Clé qui ne désigne rien », « Clé en double » et « Langue non déclarée » de `labels`, et « Avertissements de la configuration » d'`app-context`.

## 3. Valeurs, libellés et pliage dans la présentation

- [x] 3.1 Ajouter `icu_normalizer` 2.4 au workspace et à `ortelia-presentation`, et écrire `fold` (minuscules, NFD, retrait des marques combinantes, raccourci ASCII : design.md, D6). Vérifier : des tests sur les accents, la casse, les ligatures et les textes ASCII, et `cargo xtask check-layers` se termine avec le code 0.
- [x] 3.2 Écrire le formatage des valeurs : `Cell`, textes sur une ligne, entiers, décimaux selon la langue, booléens, énumérés par libellé, dates et dates-heures par défaut en UTC, rendu des motifs LDML avec les noms français et anglais, alignements par défaut (design.md, D3 et D5). Vérifier : un test par scénario de `value-formats`, sauf ceux de « Motif invalide refusé », couverts par la tâche 1.2.
- [x] 3.3 Écrire la recherche des libellés : langue courante, remontée des classes mères, `#clé#`, relevé unique des clés manquantes, journalisé en `WARN` (design.md, D7). Vérifier : un test par scénario des exigences « Langue courante », « Libellé d'un attribut hérité », « Libellé manquant visible » et « Avertissement des libellés manquants » de `labels`.

## 4. Table dans la présentation

- [x] 4.1 Écrire `TableView` : colonnes de la présentation, héritée ou déduite du modèle, en-têtes avec unité, largeurs par défaut, lignes dans l'ordre de la liste, cellules calculées à la demande, compteur réservé aux tests des lignes formatées (design.md, D5). Vérifier : un test par scénario des exigences « Lignes de la table », « Colonnes de la présentation », « Colonnes déduites du modèle », « Texte des en-têtes » et « Seules les lignes visibles sont formatées » de `table-view`.
- [x] 4.2 Écrire le tri : tri initial, cycle des clics, Maj+clic, indicateurs et rangs, comparateur (absents en dernier, énumérés par indice, booléens, clés de texte pliées gardées par révision), tri stable. Vérifier : un test par scénario des exigences « Tri initial », « Tri sur une colonne », « Tri multiple », « Indicateurs de tri », « Ordre des valeurs » et « Comparaison des textes » de `table-view`.
- [x] 4.3 Écrire le filtre : texte de chaque ligne préparé à la première saisie (séparateur `\u{1F}`), mots pliés, filtre appliqué à l'ordre trié, compteur. Vérifier : un test par scénario des exigences « Filtre global » et « Filtre et tri combinés » de `table-view`.
- [x] 4.4 Écrire les largeurs et l'ordre des colonnes (`ResizeColumn`, `MoveColumn`, numéro de disposition : design.md, D8), puis `prepare()` avec ses clés de cache (révision, tri, filtre, colonnes). Vérifier : un test par scénario des exigences « Largeur des colonnes », « Ordre des colonnes » et « Index recalculés à chaque révision » de `table-view` ; la modification de valeur passe par une transaction du Document.

## 5. État de l'application dans la présentation

- [x] 5.1 Remplacer `AppState::new` par `AppState::open(path)` : application ouverte ou en erreur, titre et taille de la fenêtre, avertissements rassemblés (configuration, libellés, diagnostics des sources), boîtes `OpenFailed`, `SourcesFailed` et `Warnings` (vingt erreurs au plus, puis le nombre des autres), `Intent`, `dispatch`, `quit_requested()` et `exit_code()` (design.md, D4). Adapter `ortelia-ui-egui` au minimum pour qu'il compile et que ses tests passent, en ouvrant `examples/annuaire`. Vérifier : un test par scénario de l'exigence « Contenu de la fenêtre principale fourni par la présentation » d'`app-launch`, et des tests de la présentation pour les états de « Application qui ne s'ouvre pas », « Source en échec » et « Barre d'état des avertissements ».

## 6. Générateur et mesure

- [x] 6.1 Ajouter à `cargo xtask gen-data` les options `--model annuaire`, `--count` et `--out <dossier>` : `app.json` et modèle recopiés par `include_str!`, personnes générées en flux, avec des noms accentués, des particules et des attributs facultatifs absents (design.md, D10). Mettre à jour `USAGE`. Vérifier : un test par scénario de l'exigence « Générateur de l'annuaire » de `storage-benchmark`, et les tests existants de `gen-data` passent toujours.
- [x] 6.2 Écrire l'exemple `table-bench` de `ortelia-presentation` (design.md, D11), avec son test sur `examples/annuaire`. Le lancer en release sur 100 000 personnes générées. Vérifier : le scénario « Mesure sur 100 000 personnes » de `table-view` (tri et filtre sous 200 ms, préparation sous une seconde), et noter les temps mesurés pour le bilan.

## 7. Interface egui

- [x] 7.1 Ajouter `egui_extras` 0.37 et la feature `monochrome_emoji_fonts` d'egui. Dessiner la table : `TableBuilder`, en-têtes avec indicateurs, cellules alignées, cases désactivées, `body.rows`, largeurs relevées en `ResizeColumn`, `reset()` au changement du numéro de disposition. Les fonctions de dessin renvoient leurs intentions. Donner à la fenêtre native le titre et la taille de la présentation. Vérifier : les tests kittest des scénarios « Affichage du view-model de la table » et « Clic et Maj+clic » de `table-view`, et « Titre de la fenêtre native » et « Texte central affiché » d'`app-launch`.
- [x] 7.2 Ajouter le glisser-déposer des en-têtes (charge `ColumnDrag`, trait d'insertion, `MoveColumn` au relâchement) et la barre de filtre (champ, bouton d'effacement, compteur). Vérifier : les tests kittest des scénarios « Glisser-déposer d'un en-tête » et « Saisie du filtre » de `table-view`.
- [x] 7.3 Ajouter les boîtes modales (`egui::Modal`), la barre d'état et la fermeture de la fenêtre sur `quit_requested()`. Vérifier : les tests kittest des scénarios « Bouton Quitter », « Bouton Continuer », « Libellés manquants » et « Aucun avertissement » d'`app-launch`.
- [x] 7.4 Mettre à jour les captures : `main_window` sur l'annuaire, nouvelle capture `open_error` avec deux erreurs fixées par le test, test de l'image différente fondé sur deux états de la table (design.md, D12). Produire les références dans WSL2 (`UPDATE_SNAPSHOTS=1`) et les regarder avant de les versionner. Vérifier : les scénarios de `ui-snapshots` passent.

## 8. Exécutable

- [x] 8.1 Accepter le chemin de l'application dans `cli::parse` et dans le texte d'aide ; résoudre le fichier (dossier, fichier, dossier courant), ouvrir l'application, écrire les erreurs d'ouverture sur la sortie d'erreur et dans le journal, et renvoyer `exit_code()` (design.md, D9). Vérifier : un test par scénario des exigences « Aide de la ligne de commande », « Argument non reconnu » et « Chemin de l'application » d'`app-launch`, et, dans `tests/startup.rs` (Linux, sans affichage), le scénario « Code de sortie ».
- [x] 8.2 Lancer le test de fumée sur `examples/annuaire`, et faire de `examples/annuaire` l'application par défaut de `tools/launch.sh`. Vérifier : le scénario « Démarrage réel » passe en local (`tools/launch.sh --check`).

## 9. Documents

- [x] 9.1 Mettre à jour `docs/feuille-de-route-annuaire.md` (incrément 2 : `vue-table` réalisé ; un libellé manquant s'affiche `#clé#`, sans repli sur l'autre langue ; la langue viendra du système à l'incrément 4) et `CLAUDE.md` (état, lancement `ortelia [chemin]`, `gen-data --model annuaire`, `table-bench`, format 0.4 de l'exemple). Vérifier : chaque commande citée s'exécute telle qu'elle est écrite.

## 10. Vérification d'ensemble

- [x] 10.1 Générer 50 000 personnes et les afficher avec `/lancer` : défilement fluide, tri, Maj+clic, filtre, redimensionnement puis déplacement d'une colonne (sa largeur la suit), boîte d'erreur sur un dossier vide. Vérifier : le critère « 50 000 personnes défilent de façon fluide » de l'incrément 2 est constaté par le porteur du projet.
- [x] 10.2 Passer `/code-review` sur le diff, puis `/simplify`, puis `/verifier` (`tools/check.sh` : format, Clippy Linux et Windows, tests, règle des couches, schémas, test de fumée). Vérifier : `/verifier` est vert.

## Workflow follow-up

- Commit et poussée sur la branche `vue-table`, puis pull request vers `main`, sur demande du porteur du projet.
- Après la fusion : `/openspec-archive-change vue-table`, puis le changement `etat-de-session`.
