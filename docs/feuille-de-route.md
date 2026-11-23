# Feuille de route

*2026-11-23. Proposé, à valider.*

Le développement avance par **trois applications jalons**, chacune construite avec Ortelia et réutilisant ce que la précédente a apporté. Chaque incrément suit le cycle OpenSpec (proposition → spécifications → conception → tâches → code → tests → archivage), avec des commits git.

---

## Principe directeur : Ortelia se décrit avec ses propres modèles

- **Le métamodèle est lui-même un modèle Ortelia** (`examples/model-editor/metamodel.model.json`). Un fichier de modèle est donc une donnée comme une autre, et l'application 2 l'édite avec les widgets génériques (arbre, formulaires).
- **Le format de `app.json` sera lui aussi décrit par un modèle Ortelia.** Le mode design (application 3) réutilisera alors l'arbre et les formulaires de l'application 2 pour éditer la configuration.

**Conséquence dès l'incrément 1** : les fichiers de configuration n'utilisent que ce que le métamodèle sait décrire.
- Contenance et références par identifiant (noms), comme dans le reste du modèle.
- Polymorphisme par le champ `_type_` : les containers sont des `View`, `Stack` ou `Split`.
- Pas de dictionnaires libres : les libellés sont des listes `{ key, text }` par langue (structure proche de votre premier `app.json`).

`examples/annuaire/app.json` (format 0.2) et `examples/annuaire/annuaire.model.json` appliquent déjà ces règles.

---

## Incrément 0 — Socle

- Dépôt GitHub, `openspec init` avec `config.yaml` en français, CLAUDE.md (conventions egui 0.37, procédure WSL2).
- Workspace Cargo en couches (voir `architecture-couches.md`) : `ortelia-model`, `ortelia-expr`, `ortelia-config`, `ortelia-service`, `ortelia-presentation`, `ortelia-ui-egui`, exécutable `ortelia`. La règle de dépendance est vérifiée par la CI.
- CI GitHub Actions Linux + Windows : format, lint, tests, binaires.
- Journalisation dans un fichier.
- Essais techniques S1 (stockage dynamique, 500 Mo), S2 (Rhai ou CEL), S3 (`egui_tiles` avec egui 0.37).

**Fini quand** : CI verte sur les deux OS ; mesures de S1 consignées ; choix de S2 et S3 tranchés.

---

## Jalon A — Application 1 « Annuaire des personnes »

### Incrément 1 — Afficher

- Métamodèle intégré : packages, classes avec héritage simple, attributs, contenance, énumérés, types de données.
- Lecture des fichiers `*.model.json` et de `app.json`, avec erreurs claires (clés inconnues, références introuvables).
- Lecture des données JSON dans le stockage dynamique (`target`, `_type_`).
- Fenêtre et containers (`View`, `Stack`, `Split`).
- Widget table : colonnes, tri multiple, filtre, largeur et ordre des colonnes, format des dates.
- Libellés FR/EN, choix de la langue.
- État de session séparé de `app.json`.

**Fini quand** : `app.json` écrit à la main affiche les personnes ; 50 000 personnes défilent de façon fluide ; la disposition est retrouvée au redémarrage.

### Incrément 2 — Éditer

- Formulaire **déduit du modèle** (widgets simples : `text`, `textArea`, `number`, `checkbox`, `combo`, `radio`, `date`, `dateTime`), mise en page en grille.
- Broker de sélection : la table pilote un panneau de propriétés.
- Édition sur copie avec Appliquer / Annuler (Q11), sauvegarde JSON.
- Langage d'expressions et contraintes simples, avec leurs sévérités.

**Fini quand** : on sélectionne une personne, on la modifie, on applique, on enregistre ; une contrainte non respectée s'affiche avec sa sévérité.

---

## Jalon B — Application 2 « Éditeur de modèles »

Ses données sont des fichiers de modèle ; son modèle est le métamodèle. Son `app.json` reste écrit à la main (ou généré par Claude).

### Incrément 3 — Arbre et actions

- Widget arbre (nœuds et libellés issus de la présentation par classe).
- Actions de menu contextuel : créer un package, une classe, un énuméré, un type de donnée, un attribut, une référence ; supprimer. Les actions possibles se déduisent du modèle (contenances de l'objet sélectionné).
- Panneau de propriétés : le formulaire généré suit la **classe réelle** de l'objet sélectionné.
- Nouveau / Ouvrir / Enregistrer / Enregistrer sous : le fichier de données est choisi par l'utilisateur, et non plus fixé dans `app.json`.

**Fini quand** : on crée de zéro le modèle de l'annuaire, et on l'enregistre.

### Incrément 4 — Références et validation

- Widgets de référence par identifiant : `refCombo` (type d'un attribut, super-classe), `refPicker`.
- `valueList` (littéraux d'un énuméré), `optional`.
- Contraintes du métamodèle : identifiants valides, bornes cohérentes, unicité des noms, références introuvables après une suppression.

**Fini quand** : un modèle édité dans l'application 2 est directement utilisé par l'application 1 (boucle complète).

---

## Jalon C — Application 3 « Mode design »

Objectif : **reconstruire entièrement l'application 1 sans éditer de JSON**.

### Incrément 5 — Éditer la configuration

- Bascule design / run sans redémarrage ; les actions de design n'apparaissent qu'en mode design.
- Modèle du format de `app.json` (`app.model.json`).
- Arbre de la configuration + formulaires générés : sources, libellés, présentation par classe, colonnes. C'est la réutilisation directe de l'application 2.
- Éditeur de modèles de l'application 2 intégré au mode design.
- Aperçu immédiat : l'application se redessine à chaque modification de la configuration.

### Incrément 6 — Disposition à la souris

- Clic droit sur la fenêtre vivante : ajouter une vue, une pile d'onglets, un slider ; scinder ; supprimer.
- Configuration d'une vue : type de widget, chemin des données, action de chargement (lecture d'un fichier JSON).
- Enregistrement de `app.json`.

**Fini quand** : partant d'une application vide, on obtient en mode design un `app.json` équivalent à celui de l'application 1.

---

## Ensuite (ordre à confirmer au fil de l'eau)

| Inc. | Contenu |
|---|---|
| 7 | Plugins natifs + SDK + modèle de projet de plugin ; source REST (essai S4 avant) |
| 8 | Widget graphique (courbes, axe temporel, seuils, marqueurs, sélection synchronisée) |
| 9 | Carte 2D/3D, un seul moteur (essai S5 avant) |
| 10 | Gantt |
| — | Vues ouvertes à la volée : un onglet par objet (Q13) ; pourrait remonter dans le jalon B |
| — | Imports XML/XSD, CSV, JSON Schema → modèle ; déduction d'un modèle à partir de données |
| — | Rafraîchissement périodique d'une source (Q12) |
| — | Plusieurs fenêtres du système d'exploitation |
