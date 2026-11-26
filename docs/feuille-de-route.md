# Feuille de route

*2026-11-24, complété le 2026-11-25 (lecture des données, scénario de charge de la carte). Proposé, à valider.*

Le développement avance par **applications jalons** : chacune est construite avec Ortelia et réutilise ce que la précédente a apporté. Chaque application a sa propre feuille de route, découpée en incréments courts. Un incrément donne un ou plusieurs changements OpenSpec (proposition → spécifications → conception → tâches → code → tests → archivage), avec des commits git ; un essai technique qui décide de la conception du reste forme son propre changement. Les incréments sont numérotés à la suite d'une application à l'autre.

| Étape | Incréments | Feuille de route | État |
|---|---|---|---|
| Socle | 0 | ci-dessous | terminé |
| Application 1 « Annuaire des personnes » | 1 à 6 | [`feuille-de-route-annuaire.md`](feuille-de-route-annuaire.md) | en cours : incrément 2 terminé |
| Application 2 « Éditeur de modèles » | 7 et 8 | [`feuille-de-route-editeur-modeles.md`](feuille-de-route-editeur-modeles.md) | à venir |

La suite se décidera à la fin de l'application 2. Les pistes connues sont en fin de document.

---

## Principe directeur : Ortelia se décrit avec ses propres modèles

- **Le métamodèle est lui-même un modèle Ortelia** (`examples/model-editor/metamodel.model.json`). Un fichier de modèle est donc une donnée comme une autre, et l'application 2 l'édite avec les widgets génériques (arbre, formulaires).
- **Le format de `app.json` sera lui aussi décrit par un modèle Ortelia.** Le mode design réutilisera alors l'arbre et les formulaires de l'application 2 pour éditer la configuration.

**Conséquence dès l'incrément 1** : les fichiers de configuration n'utilisent que ce que le métamodèle sait décrire.
- Contenance et références par identifiant (noms), comme dans le reste du modèle.
- Polymorphisme par le champ `_type_` : les containers sont des `View`, `Stack` ou `Split`.
- Pas de dictionnaires libres : les libellés sont des listes `{ key, text }` par langue.

`examples/annuaire/app.json` (format 0.3) et `examples/annuaire/annuaire.model.json` appliquent déjà ces règles.

---

## Incrément 0 — Socle (terminé)

- Dépôt GitHub, `openspec init` avec `config.yaml` en français, CLAUDE.md (conventions egui 0.37, procédure WSL2).
- Workspace Cargo en couches (voir `architecture-couches.md`) : `ortelia-model`, `ortelia-expr`, `ortelia-config`, `ortelia-service`, `ortelia-presentation`, `ortelia-ui-egui`, exécutable `ortelia`. La règle de dépendance est vérifiée par la CI.
- CI GitHub Actions Linux + Windows : format, lint, tests, binaires.
- Journalisation dans un fichier.
- Test manuel de la version Windows dans le bac à sable Windows (`tools/sandbox/`).

**Fini quand** : CI verte sur les deux OS ; l'exécutable Windows de la CI s'ouvre dans le bac à sable.

Terminé le 2026-11-24 (changement OpenSpec `socle-workspace`, archivé). Les essais techniques S1 à S3 ont lieu au début des incréments qui en ont besoin : S1 à l'incrément 1, S3 au 3, S2 au 6.

---

## Après l'application 2 : pistes

L'ordre et le contenu se décideront à la fin de l'application 2.

### Mode design

Objectif : **reconstruire entièrement l'application 1 sans éditer de JSON**. C'est le cœur du produit.

- Éditer la configuration :
  - bascule design / run sans redémarrage ; les actions de design n'apparaissent qu'en mode design ;
  - modèle du format de `app.json` (`app.model.json`) ;
  - arbre de la configuration et formulaires générés : sources, libellés, présentation par classe, colonnes. C'est la réutilisation directe de l'application 2 ;
  - éditeur de modèles de l'application 2 intégré au mode design ;
  - aperçu immédiat : l'application se redessine à chaque modification de la configuration.
- Disposition à la souris :
  - clic droit sur la fenêtre vivante : ajouter une vue, une pile d'onglets, un slider ; scinder ; supprimer ;
  - configuration d'une vue : type de widget, chemin des données, action de chargement (lecture d'un fichier JSON) ;
  - enregistrement de `app.json`.

**Fini quand** : partant d'une application vide, on obtient en mode design un `app.json` équivalent à celui de l'application 1.

### Globe 3D et carte : une bibliothèque séparée

Le globe 3D et la carte 2D (un seul moteur pour les deux) seront développés comme une **bibliothèque indépendante, dans son propre dépôt**, à la manière de NASA World Wind Java : utilisable par toute application Rust, et par Ortelia comme une dépendance ordinaire. Ortelia n'y ajoute que son intégration : le view-model de la carte (couches, objets issus du modèle, sélection synchronisée avec les autres vues) et sa configuration dans `app.json`.

- **Une réimplémentation, pas un portage** : on reprend les concepts de World Wind (couches, tuiles, niveaux de détail), pas son code. Sa conception Java / OpenGL ne se transpose pas en Rust / wgpu, et un portage serait une œuvre dérivée, soumise à sa licence.
- **Des couches, comme dans Ortelia** : un cœur sans interface ni GPU (géodésie, caméra, tuiles et caches, sélection, préparation de la scène), un moteur de rendu wgpu, et une intégration egui mince. La présentation d'Ortelia ne dépend que du cœur ; seule `ortelia-ui-egui` utilise le rendu et le widget.
- **Des versions liées** : pour dessiner dans la fenêtre egui, le moteur doit utiliser la même version de wgpu qu'eframe, et l'intégration egui suit l'API d'egui, qui change à chaque version mineure. Les montées de version se font en même temps dans les deux dépôts.
- **Essai S5 en premier** : un globe minimal (imagerie, relief, objets et polygones drapés au sol, sélection ; images par seconde et mémoire), après examen de l'existant en Rust, par exemple `walkers` (carte en tuiles pour egui) ou `galileo` (moteur cartographique).
- **Scénario de charge de S5** :
  - des milliers de zones (polygones ou cercles) et des dizaines de milliers de mailles de 4 sommets, soit de l'ordre d'un million de sommets ;
  - drapées sur le relief, avec survol et sélection, à 60 images par seconde.

  Pour le Store, ce volume est modeste : l'extraction des sommets se compte en millisecondes, et elle n'a lieu qu'au chargement et après une modification. Le risque est dans le rendu :
  - des tampons GPU persistants, et non le `Painter` d'egui, qui retriangule ses formes à chaque image ;
  - un index spatial pour le survol ;
  - des niveaux de détail pour la vue globale.

### Lecture des données : correspondance, autres formats, fusion

L'architecture de lecture (`architecture-couches.md`, § 9) est posée dès l'incrément 1. Ce qui viendra ensuite :

- **Correspondance déclarative dans `app.json`**, entre la forme du fichier et celle du modèle :
  - renommage d'attributs ;
  - conversions de valeurs simples (tables de valeurs, unités, formats de date) ;
  - conditions qui calculent des valeurs, avec le langage d'expressions de l'incrément 6.

  Exemple : des positions GeoJSON `[lon, lat]` lues comme des objets `Point` du modèle.
- **Lecteurs XML** (avec import de XSD) et **plugins de lecture**. Un plugin de lecture connaît le modèle et crée les objets directement par le constructeur.
- **Fusion d'une source avec des objets déjà présents**, pour la source REST :
  - rapprochement par identifiant métier ;
  - créations, modifications et suppressions enregistrées dans la liste des changements ;
  - à décider alors : objets absents de la réponse supprimés ou gardés ; objet en cours d'édition.

### Autres pistes

- Plugins natifs, SDK et modèle de projet de plugin ; source REST (essai S4 avant).
- Widget graphique : courbes, axe temporel, seuils, marqueurs, sélection synchronisée.
- Gantt.
- Vues ouvertes à la volée : un onglet par objet. Pourrait remonter dans l'application 2.
- Imports XML/XSD, CSV, JSON Schema → modèle ; export d'un modèle en JSON Schema ; déduction d'un modèle à partir de données.
- Rafraîchissement périodique d'une source.
- Plusieurs fenêtres du système d'exploitation.
