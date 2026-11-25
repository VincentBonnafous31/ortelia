# Feuille de route — Application 1 « Annuaire des personnes »

*2026-11-24, incrément 1 précisé le 2026-11-25. Proposé, à valider. Vue d'ensemble et principes : [`feuille-de-route.md`](feuille-de-route.md).*

Le modèle et les données d'abord, sans interface ; puis une table, un formulaire en lecture seule, la langue, l'édition et les contraintes. Chaque incrément ajoute une brique qu'on peut voir ou tester.

`examples/annuaire/` contient le modèle (`annuaire.model.json`), les données (`personnes.json`) et la configuration (`app.json`). Ces fichiers s'enrichissent d'un incrément à l'autre : l'`app.json` actuel correspond à la fin de l'incrément 2.

| Inc. | Contenu | Essai technique en début d'incrément |
|---|---|---|
| 1 | Modèle et données, sans interface | S1 : stockage dynamique |
| 2 | Vue avec une table | captures d'images : CI macOS ou lavapipe |
| 3 | Split et panneau de propriétés en lecture seule | S3 : `egui_tiles` |
| 4 | Choix de la langue | — |
| 5 | Éditer et enregistrer | — |
| 6 | Contraintes | S2 : langage d'expressions |

---

## Incrément 1 — Modèle et données, sans interface

Aucune interface : on charge le modèle, `app.json` et les personnes, et on vérifie les données par des tests unitaires. C'est la base de tout le reste, et l'essai S1 permet d'itérer sur le stockage avant de bâtir dessus.

L'incrément compte deux changements OpenSpec :

| Changement | Contenu |
|---|---|
| `stockage-dynamique` (essai S1) | métamodèle, Store d'objets, constructeur, lecteur JSON ; mesures et choix du stockage |
| « modèle et données » (à proposer) | `app.json`, erreurs claires, validation par le modèle, modification et notification, JSON Schema |

### Essai S1 : stockage dynamique

- **Un modèle d'essai représentatif**, distinct de l'annuaire : un réseau de stations météo (`examples/reseau-meteo/`).
  - Arbre de contenance de profondeur 4, listes polymorphes, héritage sur trois niveaux, tous les types de base.
  - Des milliers de zones (polygones ou cercles) découpées en mailles de 4 sommets, comme les données qu'affichera une carte.
- **Des fichiers générés de 500 Mo** (`cargo xtask gen-data`), avec un profil « texte » et un profil « numérique ».
- **Deux implémentations centrées sur l'objet**, et non une table par classe, derrière la même API :
  - un bloc de valeurs typées par objet ;
  - un tas d'objets, dont les cases sont décrites par la classe.
- **Mesures** : mémoire (pic, après chargement, allocations), temps de chargement, accès aux attributs et navigation, modification.
- **Critères, dans l'ordre** :
  1. accès et navigation, qui feront la performance de l'affichage (tri des tables, extraction des polygones de la carte, expressions) ;
  2. mémoire ;
  3. chargement.

  À résultats proches, on prend la variante la plus simple. Les seuils seront fixés au vu des résultats.
- **Résultat** (`docs/essais/s1-stockage.md`) : le bloc de valeurs typées par objet est retenu, devenu le type `Store`. Il est le plus rapide pour naviguer (remontée vers le conteneur, extraction des sommets), au prix d'environ 1,5 fois la taille du fichier en mémoire, contre 0,85 fois pour le tas d'objets.

### Contenu de l'incrément

- **Métamodèle intégré** (changement 1) : packages, classes avec héritage simple, attributs, contenance, énumérés, types de données.
- **Modèle de l'annuaire enrichi** (changement 2) : un attribut par type de donnée (entier, décimal, booléen, énuméré, date-heure) et un texte long, pour que le formulaire de l'incrément 3 ait de quoi afficher chacun de ses widgets. Les données d'exemple suivent.
- **Lecture des fichiers de modèle** (changement 1), puis **lecture de `app.json`** (changement 2), avec des erreurs claires : clés inconnues, références introuvables. À ce stade, `app.json` ne contient que `model` et `sources`.
- **Lecture des données en trois étages** (`architecture-couches.md`, § 9) : un lecteur de format, une couche de correspondance, puis un constructeur, seule API de création des objets. À l'incrément 1, il n'y a qu'un lecteur JSON en flux et la correspondance identité : nom de clé = nom d'attribut, `_type_` pour la classe concrète (changement 1), et `target` pour la partie de la racine qu'alimente une source (changement 2).
- **Validation des données par le modèle lui-même** (changement 2), sans JSON Schema pour les données :
  - attribut inconnu ou valeur de mauvais type : le chargement est refusé, et toutes les erreurs sont rassemblées (avec un plafond et le total), chacune avec son chemin dans le modèle ;
  - attribut obligatoire absent : l'objet est chargé, avec un diagnostic ;
  - le message montré à l'utilisateur peut citer la valeur fautive, le journal jamais.
- **Lire puis intégrer** (changement 2) :
  - un lecteur remplit toujours un Store neuf, sans liste des changements ;
  - l'intégration dans le contexte de l'application est un remplacement, qui ne produit qu'une seule entrée grossière dans la liste des changements ;
  - la fusion avec des objets existants viendra avec la source REST.
- **Création, lecture, modification et suppression d'objets**, couvertes par des tests unitaires (changements 1 et 2).
- **Notification des modifications** (changement 2) : toute modification passe par l'API du stockage, qui tient un numéro de révision et la liste des changements (objet, attribut, ancienne et nouvelle valeur).
  - La présentation la consultera pour savoir qu'un cache est périmé, par exemple l'index trié de la table.
  - La même liste servira à Appliquer / Annuler et à l'indicateur « modifié » (voir `architecture-couches.md`, § 3).
  - Mécanisme à arrêter dans la conception, testé sans interface.
- **JSON Schema des fichiers de configuration** (`app.json`, `*.model.json`, changement 2), généré depuis les types Rust : validation et autocomplétion dans l'éditeur de texte, tant que ces fichiers s'écrivent à la main.

**Fini quand** : mesures de S1 consignées et choix du stockage arrêté (changement 1) ; le modèle, `app.json` et `personnes.json` se chargent, et des fichiers fautifs produisent des erreurs claires ; les tests de création, modification, suppression et notification passent (changement 2).

---

## Incrément 2 — Vue avec une table

On ajoute à la fenêtre une vue contenant la table des personnes.

Contenu :
- `app.json` complété : fenêtre (titre, taille), container `View`, présentation de la classe `Personne` (colonnes, tri initial).
- Création de la vue et de sa table à la lecture de `app.json`.
- Liaison de la table aux données (`/personnes`) et à la présentation de `Personne`.
- Widget table : colonnes, tri multiple, filtre, largeur et ordre des colonnes, format des dates.
- En-têtes de colonnes lus dans les libellés de `app.json`, par la recherche définitive (langue courante, puis l'autre langue, puis nom de l'attribut) mais avec une langue fixe : l'incrément 4 n'aura qu'à ajouter le choix.
- État de session : la disposition de la table (largeurs, ordre, tri, filtre) et la taille de la fenêtre sont enregistrées à la fermeture et restaurées à l'ouverture, dans un fichier propre à l'utilisateur, rangé sous l'`id` de l'application. `app.json`, définition partagée et versionnée de l'application, n'est jamais modifié par l'utilisation.
- Essai des tests par captures d'images (egui_kittest, features `wgpu` et `snapshot`), avec la table comme premier widget dont le rendu vaut d'être figé : choisir entre une machine macOS de la CI (vrai GPU) et le rendu logiciel lavapipe sous Linux. C'est le niveau 3 de la stratégie de tests de l'interface (`CLAUDE.md`) ; les niveaux 1, 2 et 4 existent depuis l'incrément 0.

Si l'incrément s'avère trop gros, l'état de session peut en sortir pour former un incrément à part.

**Fini quand** : `app.json` affiche les personnes ; 50 000 personnes défilent de façon fluide ; la disposition est retrouvée au redémarrage ; la plateforme des captures d'images est choisie.

---

## Incrément 3 — Split et panneau de propriétés en lecture seule

On scinde la fenêtre : la table à gauche, à droite un panneau de propriétés qui affiche la personne sélectionnée. Pas encore d'édition.

Essai technique, en début d'incrément :
- S3 : `egui_tiles` avec egui 0.37, pour les containers.

Contenu :
- `app.json` complété : container `Split`, vue des propriétés, présentation du formulaire de `Personne`.
- Container `Split`, dont la position de la séparation va dans l'état de session.
- Formulaire **déduit du modèle** (widgets simples : `text`, `textArea`, `number`, `checkbox`, `combo`, `radio`, `date`, `dateTime`), mise en page en grille, champs en lecture seule.
- Broker de sélection : la table pilote le panneau de propriétés.
- Capture de référence du formulaire (la plateforme des captures est choisie à l'incrément 2).

**Fini quand** : choix de S3 tranché ; la sélection d'une personne l'affiche dans le formulaire, avec un widget par type d'attribut ; la disposition, séparation comprise, est retrouvée au redémarrage.

---

## Incrément 4 — Choix de la langue

- Menu de configuration avec le choix de la langue.
- Langue au démarrage : celle choisie par l'utilisateur, sinon celle du système si l'application la propose (`languages` de `app.json`), sinon la première de la liste.
- Libellés FR/EN : ceux de `app.json`, déjà lus depuis l'incrément 2, et ceux d'Ortelia (menus, messages d'erreur), avec un mécanisme de traduction à choisir pour ces derniers.
- La langue choisie est enregistrée dans l'état de session.

**Fini quand** : l'application s'affiche en français ou en anglais, et tous les libellés affichés sont traduits.

---

## Incrément 5 — Éditer et enregistrer

- Le panneau de propriétés devient éditable, avec les mêmes widgets.
- Édition sur copie, avec Appliquer / Annuler. L'application immédiate, champ par champ, viendra plus tard.
- Après Appliquer, la table se met à jour (notification de l'incrément 1).
- Indicateur « modifié » et enregistrement du fichier JSON.

**Fini quand** : on sélectionne une personne, on la modifie, on applique, la table montre la modification, on enregistre ; le fichier relu contient la modification.

---

## Incrément 6 — Contraintes

Essai technique, en début d'incrément :
- S2 : langage d'expressions, Rhai ou CEL.

Contenu :
- Langage d'expressions (`ortelia-expr`). C'est le même langage qui servira plus tard aux conditions de la correspondance déclarative (`feuille-de-route.md`, pistes).
- Contraintes simples déclarées dans le modèle, avec leurs sévérités (`ERROR`, `WARNING`, `INFO`).
- Affichage des contraintes non respectées dans le panneau de propriétés.

**Fini quand** : choix de S2 tranché ; une contrainte non respectée s'affiche avec sa sévérité.
