# Feuille de route — Application 1 « Annuaire des personnes »

*2026-11-24, incrément 1 précisé puis réalisé le 2026-11-25. Proposé, à valider. Vue d'ensemble et principes : [`feuille-de-route.md`](feuille-de-route.md).*

Le modèle et les données d'abord, sans interface ; puis une table, un formulaire en lecture seule, la langue, l'édition et les contraintes. Chaque incrément ajoute une brique qu'on peut voir ou tester.

`examples/annuaire/` contient le modèle (`annuaire.model.json`), les données (`personnes.json`) et la configuration (`app.json`). Ces fichiers s'enrichissent d'un incrément à l'autre : l'`app.json` actuel ne contient que ce qu'utilise l'incrément 1, le modèle et les sources.

| Inc. | Contenu | Essai technique en début d'incrément |
|---|---|---|
| 1 | Modèle et données, sans interface | S1 : stockage dynamique |
| 2 | Vue avec une table | captures d'images : lavapipe sous Linux, retenu |
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
| `modele-et-donnees` | `app.json`, erreurs claires, validation par le modèle, modification et notification, JSON Schema |

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
- **Lire puis intégrer** (changement 2, `architecture-couches.md`, D30 et D32) :
  - un lecteur remplit toujours un fragment neuf (un Store détaché et les objets de sa branche), sans liste des changements ;
  - l'intégration greffe le fragment sur sa branche (`/`, `/annuaire/personnes`…) en remplaçant son contenu, sans recopier les objets, et ne produit qu'un seul changement ;
  - à l'ouverture, la racine désignée par `app.json` et les objets intermédiaires des cibles sont créés d'abord ; chaque source alimente sa propre branche ;
  - la fusion avec des objets existants viendra avec la source REST.
- **Création, lecture, modification et suppression d'objets**, couvertes par des tests unitaires (changements 1 et 2).
- **Notification des modifications** (changement 2, `architecture-couches.md`, D31) : toute modification passe par une transaction du Document, seul à modifier le Store. Le Document tient un numéro de révision et le journal des changements (créations, suppressions, anciennes et nouvelles valeurs, remplacements de branche).
  - La présentation lira la révision et les changements depuis celle qu'elle a vue, sans observateurs, pour savoir qu'un cache est périmé, par exemple l'index trié de la table.
  - Le même journal servira à Appliquer / Annuler et à l'indicateur « modifié » (voir `architecture-couches.md`, § 3).
- **JSON Schema des fichiers de configuration** (`app.json`, `*.model.json`, changement 2), généré depuis les types Rust : validation et autocomplétion dans l'éditeur de texte, tant que ces fichiers s'écrivent à la main.

**Fini quand** : mesures de S1 consignées et choix du stockage arrêté (changement 1) ; le modèle, `app.json` et `personnes.json` se chargent, et des fichiers fautifs produisent des erreurs claires ; les tests de création, modification, suppression et notification passent (changement 2).

**Résultat** : l'ouverture de `examples/annuaire/app.json` (`AppContext::open`) compile le modèle, crée la racine et l'annuaire, puis y greffe les dix personnes ; les erreurs de `app.json`, du modèle et des données sont rassemblées, chacune avec son fichier ou son chemin, et le journal ne contient aucune valeur des données. `cargo xtask gen-schemas` génère les schémas de `schemas/`, que la CI vérifie.

---

## Incrément 2 — Vue avec une table

On ajoute à la fenêtre une vue contenant la table des personnes.

Contenu :
- `app.json` complété : fenêtre (titre, taille), container `View`, présentation de la classe `Personne` (colonnes, tri initial).
- Création de la vue et de sa table à la lecture de `app.json`.
- Liaison de la table aux données (`/annuaire/personnes`) et à la présentation de `Personne`.
- Widget table : colonnes, tri multiple, filtre, largeur et ordre des colonnes, format des dates.
- En-têtes de colonnes lus dans les libellés de `app.json`, dans la première langue de `languages` : l'incrément 4 la prendra du système et ajoutera le choix. Un libellé manquant s'affiche `#clé#` (par exemple `#Personne.prenom#`), sans repli sur l'autre langue, et devient un avertissement.
- **Fait** (changement `vue-table`) : `app.json` au format 0.4 (langues, libellés, présentations, fenêtre), table de l'annuaire (tri multiple au clic et au Maj+clic, filtre insensible aux accents et à la casse, compteur, largeur et ordre des colonnes, formats des nombres et des dates), boîtes d'erreur d'ouverture et de sources en échec, barre des avertissements, chemin de l'application en argument (`ortelia [chemin]`). Sur 100 000 personnes générées (`cargo xtask gen-data --model annuaire`, mesure `table-bench` en release dans WSL2) : tri initial 41 ms, tri par `nom` 8 ms, préparation du filtre 90 à 113 ms, filtre `dup` 2 à 4 ms.
- État de session : la disposition de la table (largeurs, ordre, tri, filtre) et la taille de la fenêtre sont enregistrées à la fermeture et restaurées à l'ouverture. `app.json`, définition partagée et versionnée de l'application, n'est jamais modifié par l'utilisation. **Fait** (changement `etat-de-session`) :
  - le fichier d'état est rangé à côté de `app.json` : `app.session.json` (plus généralement, le nom du fichier de l'application, `.json` devenant `.session.json`), ignoré par git ; le supprimer ramène la disposition de `app.json` ;
  - il est écrit à la fermeture normale de la fenêtre, par un fichier temporaire renommé, et restauré avec indulgence : éléments disparus oubliés, colonnes nouvelles ajoutées à la fin, valeurs ramenées dans leurs bornes, fichier illisible ou d'une autre version ignoré avec un avertissement dans le journal, où le texte du filtre n'apparaît jamais ;
  - la position de la fenêtre et l'état maximisé ne sont pas enregistrés ; l'état propre à chaque utilisateur, dans son dossier personnel, viendra plus tard, sans changer de format.
- Essai des tests par captures d'images (egui_kittest, features `wgpu` et `snapshot`) : choisir entre une machine macOS de la CI (vrai GPU) et le rendu logiciel lavapipe sous Linux. C'est le niveau 3 de la stratégie de tests de l'interface (`CLAUDE.md`) ; les niveaux 1, 2 et 4 existent depuis l'incrément 0. **Fait** (changement `captures-images`, compte rendu `docs/essais/captures-images.md`) :
  - les captures sont comparées sous Linux avec lavapipe, sur le runner `ubuntu-26.04` de la CI, contre des références produites dans WSL2, sans aucun pixel fautif ;
  - sous macOS (Metal) et Windows (WARP), l'anticrénelage du texte diffère, et une scène riche dépasse la tolérance ;
  - la fenêtre principale a sa capture, celle de la table de l'annuaire depuis le changement `vue-table`, ainsi que la boîte d'erreur d'ouverture.

**Fini quand** : `app.json` affiche les personnes ; 50 000 personnes défilent de façon fluide ; la disposition est retrouvée au redémarrage ; la plateforme des captures d'images est choisie. **Rempli** : les quatre critères le sont, le dernier par `etat-de-session`.

---

## Incrément 3 — Split et panneau de propriétés en lecture seule

On scinde la fenêtre : la table à gauche, à droite un panneau de propriétés qui affiche la personne sélectionnée. Pas encore d'édition.

Essai technique, en début d'incrément :
- S3 : `egui_tiles` avec egui 0.37, pour les containers. **Fait** (changement `conteneur-split`, compte rendu `docs/essais/s3-conteneurs.md`) : egui_tiles est retenu, tous les critères sont vérifiés par des tests du dépôt (split sans onglets ni glisser-déposer, split imbriqué, table dans une tuile, séparation glissée et double-cliquée dans un test sans écran, parts retrouvées après fermeture, capture sous lavapipe).

L'incrément est découpé en trois changements OpenSpec :
1. `conteneur-split` : essai S3 et container `Split`, avec une vue vide à droite de la table ;
2. `formulaire` : sa description dans `app.json`, son modèle dans la présentation et son dessin, d'abord vide, dans la vue de droite ;
3. le broker de sélection : la sélection dans la table, et la vue des propriétés, qui écoute la sélection et affiche le formulaire de la classe de l'objet sélectionné.

Contenu :
- `app.json` complété : container `Split`, vue des propriétés, présentation du formulaire de `Personne`.
- Container `Split`, dont la position de la séparation va dans l'état de session. **Fait** (changement `conteneur-split`) :
  - `app.json` au format 0.5 : le contenu de la fenêtre est un arbre de splits (`horizontal` ou `vertical`, au moins deux enfants, `weights` facultatifs, ramenés à des parts de somme 1) et de vues, dont le nouveau widget `Empty` ; vues et splits partagent un même espace d'identifiants ;
  - l'annuaire est un split `principal` : la table `personnes` sur 65 % de la largeur, la vue vide `proprietes` sur 35 % ;
  - la structure est fixe (ni onglets, ni glisser-déposer des vues) ; la séparation se glisse, et un double-clic partage également l'espace ;
  - les parts de chaque split vont dans `app.session.json` (clé `containers`, sans changer de version de format), et sont restaurées à l'ouverture ; des parts d'un nombre faux ou non strictement positives sont ignorées.
- Formulaire **déduit du modèle** (widgets simples : `text`, `textArea`, `number`, `checkbox`, `combo`, `radio`, `date`, `dateTime`), mise en page en grille, champs en lecture seule. **Fait** (changement `formulaire`) :
  - `app.json` au format 0.6 : la présentation d'une classe peut déclarer un formulaire (`form`) : une grille (`columns`, poids `grow`) et des éléments polymorphes, `Field` (attribut, widget, clé de libellé, cellules du libellé et du widget), `Group` (titre, grille et éléments propres), `Separator` et `Text` ; une cellule couvre `span` colonnes et peut commencer une ligne (`newRow`) ;
  - huit widgets de champ, chacun vérifié contre le type de son attribut : `TextField`, `TextArea` (`rows`), `NumberField`, `CheckBox`, `ComboBox`, `RadioGroup`, `DateField` et `DateTimeField` (`format`) ; un champ sans widget prend celui du type de son attribut ;
  - nouveau widget de vue `Form`, qui nomme sa classe ; le formulaire est celui de la présentation de la classe ou de sa classe mère la plus proche, sinon il est déduit du modèle (un champ par attribut) ;
  - grille en flux dans la présentation, indépendante des formulaires : placement des cellules et largeurs des colonnes depuis les minimums que mesure l'interface ;
  - dessin en lecture seule (texte sélectionnable, cases, listes et boutons radio désactivés), groupes dans un cadre titré, défilement dans les deux sens ; chaque widget est relié à son libellé pour l'accessibilité ;
  - les valeurs de l'objet montré sont relues à chaque image : un objet supprimé ou d'une autre classe n'est pas montré ; dans l'annuaire, la vue de droite montre le formulaire vide de `Personne`, en attendant le broker ;
  - capture de référence `form` (Dupont), sous lavapipe.
- Broker de sélection : la table pilote le panneau de propriétés.

**Fini quand** : choix de S3 tranché ; la sélection d'une personne l'affiche dans le formulaire, avec un widget par type d'attribut ; la disposition, séparation comprise, est retrouvée au redémarrage.

---

## Incrément 4 — Choix de la langue

- Menu de configuration avec le choix de la langue.
- Langue au démarrage : celle choisie par l'utilisateur, sinon celle du système si l'application la propose (`languages` de `app.json`), sinon la première de la liste.
- Libellés FR/EN : ceux de `app.json`, déjà lus depuis l'incrément 2, et ceux d'Ortelia (menus, messages d'erreur), avec un mécanisme de traduction à choisir pour ces derniers.
- Internationaliser **tous** les messages d'Ortelia, en particulier les erreurs à l'ouverture d'une application. À l'incrément 2, l'interface mélange les deux langues :
  - les textes de l'interface (« Filtrer », « Quitter », barre des avertissements…) sont des constantes en français de `ortelia-presentation` ;
  - les erreurs et diagnostics (de `app.json`, du modèle, des sources) sont des phrases en anglais, écrites par l'implémentation de `Display` dans `ortelia-config`, `ortelia-model` et `ortelia-service`, puis affichées telles quelles dans la boîte d'erreur ;
  - certains messages viennent du système ou des bibliothèques (`No such file or directory (os error 2)`, erreurs de syntaxe de serde_json).

  Piste : les couches basses rendent des erreurs structurées (un genre et ses paramètres : chemin, ligne, nom de classe…), et la présentation en fait le texte dans la langue choisie, comme pour les libellés. Le journal reste en anglais.
- La langue choisie est enregistrée dans l'état de session.

**Fini quand** : l'application s'affiche en français ou en anglais, et tous les libellés et messages affichés sont traduits, erreurs à l'ouverture comprises.

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
