# Proposition : stockage-dynamique

## Why

Le modèle de données d'une application Ortelia n'est connu qu'à l'exécution : les objets ne peuvent pas être des structs Rust. Tout Ortelia repose donc sur un stockage dynamique d'objets, et sa disposition en mémoire décide de trois choses :
- la vitesse d'accès aux attributs et de navigation dans l'arbre des objets, qui fera la performance de l'affichage (tri des tables, extraction de milliers de polygones pour la carte, expressions) ;
- la mémoire consommée, avec des fichiers d'environ 500 Mo ;
- le temps de chargement.

Il faut choisir cette disposition sur des mesures, avant de bâtir le reste dessus : c'est l'essai technique S1.

Ce changement est le premier des deux changements de l'**incrément 1** de `docs/feuille-de-route-annuaire.md`. Il remplit le premier critère « Fini quand » de cet incrément : **mesures de S1 consignées et choix du stockage arrêté**. Le second changement (modèle et données) remplira les autres critères : le modèle, `app.json` et `personnes.json` se chargent ; des fichiers fautifs produisent des erreurs claires ; les tests de création, modification, suppression et notification passent.

## What Changes

- **Métamodèle intégré** (`ortelia-model`) :
  - lecture des fichiers `*.model.json` (format 0.1 actuel) ;
  - compilation : packages, classes avec héritage simple et classes abstraites, attributs à une seule valeur, contenance simple et multiple, énumérés, types dérivés d'un type de base ;
  - types de base : `String`, `Int`, `Float`, `Boolean`, `Date`, `DateTime` (UTC, à la microseconde) ;
  - chaque attribut reçoit une position fixe, la même dans toutes les sous-classes ; les noms sont résolus une fois en identifiants.
- **Store d'objets** (`ortelia-model`) :
  - identifiant d'objet stable pendant la session et jamais réutilisé ;
  - création, lecture et modification des attributs ;
  - navigation vers les enfants et vers le conteneur ;
  - insertion et retrait dans une liste, suppression d'un sous-arbre.
- **Deux implémentations d'essai**, centrées sur l'objet et non sur des tables par classe, derrière la même API et avec la même suite de tests :
  - **V1** : un bloc de valeurs typées par objet ;
  - **V2** : un tas d'objets, avec des enregistrements contigus dont les cases non typées sont décrites par la classe.

  Les chaînes vont dans une arène commune aux deux.
- **Constructeur** (`ortelia-model`) : la seule API de création des données pour tous les lecteurs, présents et futurs (JSON, XML, plugins de lecture qui connaissent le modèle).
  - Le lecteur l'appelle dans l'ordre du fichier : début d'objet, valeur d'attribut, fin d'objet.
  - Une valeur donnée sous forme de texte est convertie selon le type de l'attribut.
- **Lecteur JSON en flux** (`ortelia-service`) :
  - lecture sans arbre intermédiaire, avec la correspondance identité : une clé porte le nom d'un attribut ou d'une référence, et `_type_` donne la classe concrète, même quand ce n'est pas la première clé ;
  - le fichier contient l'objet racine ;
  - une donnée non conforme refuse le chargement, avec son chemin dans le modèle.
- **Modèle d'essai et générateur de données** :
  - un réseau de stations météo de profondeur 4, avec listes polymorphes, héritage sur trois niveaux et tous les types de base, ainsi que des zones polygonales ou circulaires découpées en mailles ;
  - un générateur (`cargo xtask gen-data`) avec une taille visée, une graine, la part de texte long et la part d'objets dont `_type_` n'est pas en premier.
- **Programme de mesure** (exemple de `ortelia-service`) :
  - mémoire (pic, après chargement, allocations) ;
  - chargement (plancher sans construction, lecture en flux ou depuis un tampon) ;
  - accès aux attributs et navigation ;
  - modification.

  `serde_json::Value` sert de point de repère (V0).
- **Décision** :
  - critères dans l'ordre : accès et navigation, puis mémoire, puis chargement ; à résultats proches, la variante la plus simple ;
  - les seuils seront fixés au vu des résultats ;
  - mesures et choix consignés dans `docs/essais/s1-stockage.md` ;
  - la variante écartée est supprimée.

### Hors périmètre

- `app.json` (`model`, `sources`, `target`) et la lecture d'une source dans une partie de l'objet racine : second changement.
- Les erreurs de données rassemblées (toutes, avec un plafond et le total), le diagnostic d'attribut obligatoire absent, et des messages qui citent la valeur fautive sans qu'elle aille dans le journal : second changement. Ici, la première erreur refuse le chargement.
- Les messages d'erreur détaillés des fichiers de modèle (fichier, emplacement) : second changement. Ici, seules les erreurs qui empêchent la compilation sont détectées.
- La révision, la liste des changements, l'intégration d'un Store lu dans le contexte de l'application (remplacement, avec une seule entrée grossière) : second changement. Ici, les modifications ne sont pas enregistrées, et la place d'un objet supprimé n'est pas réutilisée.
- Les attributs multivalués (refusés à la compilation), `refersTo`, l'identifiant métier, les contraintes des types dérivés (`pattern`, `min`, `max`…), les unités et les valeurs par défaut : lus dans le fichier de modèle quand ils y figurent, mais pas encore utilisés.
- La correspondance déclarative (renommages, conversions, conditions), les lecteurs XML et les plugins de lecture, la fusion d'une source avec des objets existants (REST), l'écriture des fichiers JSON.
- Les géométries GeoJSON : le modèle d'essai décrit les sommets comme des objets `Point`, et le générateur les écrit en objets `{ "lat": …, "lon": … }`.
- Le JSON Schema des fichiers de configuration : second changement.
- Les tests de performance dans la CI : le générateur les prépare, ils viendront plus tard.
- La lecture en tâche de fond, avec progression et annulation : le Store est seulement rendu transférable entre threads.

## Capabilities

### New Capabilities

- `metamodel` : lecture des fichiers de modèle et compilation du métamodèle (classes, héritage simple, attributs, contenance, énumérés, types de base et dérivés), avec la position fixe de chaque attribut et la résolution des noms.
- `object-store` : stockage dynamique des objets : identité, création, lecture et modification des attributs, navigation dans l'arbre de contenance, suppression.
- `data-loading` : création des données par le constructeur, conversion des valeurs, lecture des fichiers JSON en flux avec la correspondance identité et `_type_`, refus des données non conformes.
- `storage-benchmark` : modèle d'essai, générateur de données reproductible, et programme de mesure des performances du stockage.

### Modified Capabilities

Aucune.

## Impact

- **Code** :
  - `ortelia-model` : métamodèle, Store V1 et V2, constructeur ;
  - `ortelia-service` : lecteur JSON, exemple de mesure `s1` ;
  - `xtask` : commande `gen-data`.
- **Nouveaux fichiers** : `examples/reseau-meteo/` (modèle d'essai et petit fichier de données) et `docs/essais/s1-stockage.md` (résultats).
- **Fichiers modifiés** : `CLAUDE.md` (arborescence, commandes du générateur et de la mesure), `docs/feuille-de-route-annuaire.md` (résultat de S1).
- **Dépendances externes** :
  - serde 1.0.229 (feature `derive`), serde_json 1.0.154 et jiff 0.2.43 ;
  - fastrand 2.5.0, pour le générateur ;
  - serde, serde_json et fastrand sont déjà présents dans `Cargo.lock`, comme dépendances indirectes. jiff 0.2 est la série qu'utilise `egui_extras` 0.37 (sélecteur de date) : on évite ainsi deux versions de jiff le jour où la table et le formulaire arriveront.
- **Poste de développement** : les mesures se font dans WSL2, en compilation release, avec des fichiers générés dans le système de fichiers Linux (`~/s1/`), et non sous `/mnt/c`.
