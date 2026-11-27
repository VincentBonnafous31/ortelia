# Tâches : formulaire

Chaque tâche laisse le workspace compilable et les tests au vert. Les commandes se lancent dans WSL2, avec `CARGO_TARGET_DIR=~/target/ortelia`.

## 1. Format 0.6 de `app.json` (`ortelia-config`)

- [x] 1.1 Ajouter le widget de vue `Form` (`FormWidget { class }`), `CheckedView::Form { id, class }`, le refus de `data` pour une vue formulaire (`UnexpectedData` reçoit le genre du widget) et la vérification de la classe (`UnknownFormClass`). Passer `APP_FORMAT_VERSION` à `0.6`, ainsi que les textes `app.json` des tests. Dans `examples/annuaire/app.json`, la vue `proprietes` devient `{ "_type_": "Form", "class": "Personne" }`. En attendant la tâche 3.2, `LayoutNode::build` traite une vue formulaire comme une vue vide, pour que la présentation et l'interface restent inchangées (design.md, D1 et D2). Vérifier : un test par scénario des exigences « Version du format », « Contenu de la fenêtre » (dont « Vue formulaire » et « Widget inconnu »), « Données selon le widget » et « Classe d'une vue formulaire vérifiée » (dont « Formulaire sans classe ») du delta d'`app-config` ; tous les tests existants passent, captures comprises.
- [x] 1.2 Écrire les définitions du formulaire et leur vérification, dans un nouveau module `form` (design.md, D1 et D2) :
  - définitions : `FormDef`, `GridColumnDef`, `CellDef`, `Element` (`Field`, `Group`, `Separator`, `Text`), `FieldWidget` (huit widgets) et `PresentationDef::form` ;
  - vérification : `check_form` avec les défauts résolus (grille `[0, 1]`, étendues, widget par défaut), `default_widget` et `accepts`, et les erreurs situées par `form.elements[i]…` ;
  - annuaire : `examples/annuaire/app.json` reçoit le formulaire de `Personne` de design.md (D6), sans encore le libellé `Personne.identite`.

  Vérifier :
  - un test par scénario des exigences « Contenu de `app.json` », « Présentations vérifiées contre le modèle », « Éléments d'un formulaire », « Clés d'un champ », « Clés d'un groupe, d'un séparateur et d'un texte », « Grille d'un formulaire », « Étendue d'une cellule vérifiée », « Champs vérifiés contre le modèle », « Widgets d'un champ », « Widget compatible avec son attribut » et « Erreurs d'un formulaire situées » du delta d'`app-config` ;
  - un test vérifie que `accepts(default_widget(t), t)` est vrai pour chaque type d'attribut.
- [x] 1.3 Rassembler les clés de libellé citées par les formulaires (`form_label_keys`, sur les définitions brutes), et ne plus signaler `UnknownLabelKey` pour elles. Ajouter à l'annuaire les libellés `Personne.identite` (`Identité`, `Identity`). Vérifier : un test par scénario des exigences « Forme des clés » (« Clé libre d'un formulaire », sur la recherche de `ortelia-config`) et « Clé qui ne désigne rien » du delta de `labels`. L'annuaire s'ouvre sans avertissement.
- [x] 1.4 Régénérer les schémas (`cargo xtask gen-schemas`), et adapter les tests des schémas. Vérifier : un test par scénario de l'exigence « Schémas générés depuis le code » du delta de `config-schemas`, dont « Contenu polymorphe de la fenêtre » et « Éléments polymorphes d'un formulaire ». `cargo xtask gen-schemas --check` se termine avec le code 0, et `examples/annuaire/app.json` est valide pour `schemas/app.schema.json`.

## 2. Grille (`ortelia-presentation`, module `grid`)

- [x] 2.1 Écrire `CellSpec`, `Placement` et `Grid::new`, qui place les cellules en flux (design.md, D3). Vérifier : un test par scénario des exigences « Colonnes de la grille », « Cellules placées en flux », « Passage automatique à la ligne » et « Nouvelle ligne demandée » de `grid-layout`.
- [x] 2.2 Écrire `Grid::min_widths` et `Grid::widths` (`ColumnWidths` : largeurs, positions, total), dans l'ordre de design.md (D3). Vérifier :
  - un test par scénario des exigences « Largeur minimale des colonnes », « Cellule plus large que ses colonnes », « Largeur disponible répartie » et « Largeur disponible insuffisante » de `grid-layout` ;
  - un test sur quelques grilles vérifie que les largeurs ne sont jamais sous les minimums, et que le total égale la largeur disponible quand une colonne s'élargit et que la place suffit.

## 3. Formulaire dans la présentation (`ortelia-presentation`, module `form`)

- [x] 3.1 Préparer les briques partagées :
  - extraire `ValueFormat::text(value, keep_lines)` de `ValueFormat::cell`, qui l'appelle avec `keep_lines = false` ;
  - déplacer dans `labels` la fonction qui ajoute l'unité au libellé d'un attribut ;
  - ajouter `LabelLookup::key(key)` pour une clé libre (`#clé#` si elle manque, comptée une fois).

  Vérifier :
  - les tests existants de `value-formats` et de la table passent sans changement ;
  - un test montre qu'avec `keep_lines`, `Ligne 1\nLigne 2` garde son saut de ligne ;
  - un test de `LabelLookup::key` couvre la clé trouvée, la clé manquante et le compte unique d'une clé manquante cherchée deux fois.
- [x] 3.2 Construire le formulaire à l'ouverture (design.md, D4) :
  - `FormView`, `FormGrid` et `FormCell` ;
  - résolution par la présentation la plus proche, et formulaire déduit du modèle ;
  - aplatissement des champs en deux cellules, grilles des groupes ;
  - libellés par le `LabelLookup` de l'ouverture ;
  - `ViewContent::Form` et `ViewRef::Form`, ce dernier encore ignoré par l'interface ;
  - fin du traitement provisoire de la tâche 1.1.

  Vérifier :
  - un test par scénario des exigences de `form-view` « Vue formulaire », « Formulaire de la présentation », « Formulaire déduit du modèle », « Widget par défaut », « Cellules d'un champ », « Cellule d'un groupe, d'un séparateur, d'un texte », « Libellé d'un champ », « Titre d'un groupe et texte fixe » et « Formulaire vide » ;
  - un test par scénario de l'exigence « Avertissement des libellés manquants » du delta de `labels` ;
  - un test pour « Disposition de l'annuaire » du delta de `window-layout`, et un pour « Textes de la fenêtre principale » du delta d'`app-launch`.
- [x] 3.3 Montrer un objet (design.md, D4) :
  - `FormRef::value` (valeur vide sans objet, ou pour un objet mort ou d'une autre classe) ;
  - `AppState::set_form_object` et `TableRef::object(row)` ;
  - réexport de `ObjectId`.

  Vérifier : un test par scénario des exigences « Valeurs de l'objet montré », « Booléen dans une case », « Énuméré en liste ou en boutons radio », « Objet montré » et « Valeurs à jour » de `form-view`. Les scénarios « Objet retiré » et « Prénom modifié » travaillent sur une copie de l'annuaire dans un dossier temporaire, par une transaction du Document.

## 4. Dessin (`ortelia-ui-egui`, module `form`)

- [x] 4.1 Relire dans les sources d'egui 0.37.0 les points de l'API cités par design.md (contexte et D5), et corriger design.md en cas d'écart. Puis écrire le module `form` (design.md, D5) :
  - mesure des cellules, et dessin ligne par ligne dans des `Ui` enfants, avec la réservation finale ;
  - widgets en lecture seule, chacun relié à son libellé par `labelled_by` ;
  - cadre titré des groupes, séparateur, texte fixe ;
  - `ScrollArea::both` ;
  - branchement de `ViewRef::Form` dans `pane_ui`.

  Le test de la vue vide passe sur une copie de l'annuaire dont la vue de droite est vide.

  Vérifier, avec egui_kittest :
  - un test par scénario des exigences « Affichage du formulaire », « Dessin en grille », « Lecture seule » et « Défilement du formulaire » de `form-view` ;
  - un test par scénario des exigences « Disposition fixe » (« Vue glissée ») et « Vue vide » du delta de `window-layout` ;
  - les positions sont lues dans les rectangles des nœuds d'accessibilité, jamais écrites en dur ;
  - les tests existants de la table, du split et de la fenêtre principale passent.
- [x] 4.2 Renouveler la capture `main_window` et écrire la capture `form` : parts 0,4 et 0,6 par `Intent::SplitResized`, et `Dupont` par `set_form_object` (`UPDATE_SNAPSHOTS=1 cargo test -p ortelia-ui-egui`). Regarder les deux images avant de les versionner. Vérifier : les scénarios « Fenêtre principale conforme » et « Formulaire conforme » du delta d'`ui-snapshots` passent en local avec lavapipe, et la capture `open_error` n'a pas changé.

## 5. Essai manuel et documents

- [x] 5.1 Essai manuel par `/lancer`, en regardant :
  - le formulaire vide de `Personne` à droite, avec le groupe « Identité » dans son cadre, le séparateur, et les notes sous leur libellé ;
  - le défilement quand la fenêtre est réduite ;
  - la séparation glissée, qui élargit les champs.

  Vérifier : ces constats, un journal sans `ERROR` ni avertissement de libellé, et `git status` sans `app.session.json`. Si WSLg ne s'ouvre pas, faire l'essai sous Xvfb et le dire dans le bilan.
- [x] 5.2 Mettre à jour les documents :
  - `CLAUDE.md` : état du projet (changement `formulaire` de l'incrément 3), `app.json` au format 0.6, description de `examples/annuaire` ;
  - `docs/feuille-de-route-annuaire.md` : changement 2 de l'incrément 3 fait, avec ce qu'il apporte ;
  - `docs/architecture-couches.md` : grille dans la présentation, mesures par l'interface, catalogue des widgets partagé entre vues et champs, noms de `_type_` uniques dans `app.json`.

  Vérifier : aucun renvoi à `docs/cadrage-prive/` dans les fichiers versionnés modifiés, et les commandes citées s'exécutent telles qu'écrites.

## 6. Revue et vérification

- [x] 6.1 `/code-review` sur le diff, puis `/simplify`, puis `/verifier` (`tools/check.sh` : format, Clippy pour Linux et pour la cible Windows, tests, règle des couches, schémas, test de fumée). Vérifier : le bilan de `/verifier` est entièrement vert, et `openspec validate formulaire --strict` réussit.

## Workflow follow-up

- Après fusion de la pull request, archiver le changement (`/openspec-archive-change`) sur une branche dédiée, puis vérifier que `openspec/specs/` contient `grid-layout` et `form-view`, et que les deltas y sont fusionnés.
