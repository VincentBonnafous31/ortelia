# Tâches : stockage-dynamique

Toutes les commandes `cargo` se lancent dans WSL2, avec `CARGO_TARGET_DIR=~/target/ortelia` (voir `CLAUDE.md`). Chaque tâche laisse le workspace compilable et les tests au vert. Les gros fichiers d'essai vont dans `~/s1/`, jamais sous `/mnt/c`.

## 1. Dépendances et métamodèle

- [x] 1.1 Ajouter au `Cargo.toml` racine serde 1.0.229 (feature `derive`), serde_json 1.0.154, jiff 0.2.43 (`default-features = false`, feature `std`) et fastrand 2.5.0, puis les déclarer dans `ortelia-model` (serde, serde_json, jiff). Vérifier :
  - `cargo build --workspace` réussit ;
  - `cargo tree -p ortelia-model -i jiff` ne montre que jiff 0.2.43 ;
  - un test de `ortelia-model` lit `2026-03-12T10:15:00.123456Z` en `jiff::Timestamp` avec ces features ;
  - `cargo xtask check-layers` n'affiche aucune violation.
- [x] 1.2 Écrire `src/metamodel/file.rs` : les types serde du format 0.1, avec `deny_unknown_fields` et `rename_all = "camelCase"`, y compris les clés lues mais pas encore utilisées (design.md, Decision 2). Vérifier par des tests :
  - `annuaire.model.json` et `metamodel.model.json` se lisent ;
  - une clé `atributes` fait échouer la lecture avec une erreur qui la cite.
- [x] 1.3 Écrire la compilation (`src/metamodel/mod.rs`, `compile.rs`) : `Model`, `ClassId`, `AttrId` et `RefId` (avec leur classe déclarante), `AttrType`, types de base, énumérés, types dérivés, disposition avec les attributs hérités d'abord, chaîne des ancêtres et test de sous-classe en temps constant, présence de sous-classes, recherche par nom. Erreurs : type ou super-classe introuvable, cycle, classe en double, attribut ou référence redéclaré, attribut multivalué. Vérifier : un test par scénario de la spécification `metamodel`, sur l'annuaire et sur de petits modèles écrits dans les tests.
- [x] 1.4 Écrire `examples/reseau-meteo/reseau.model.json` (design.md, Decision 10) et l'ajouter au tableau des documents de `CLAUDE.md`. Vérifier par un test :
  - le modèle se compile ;
  - `Thermometre` a pour ancêtres `Capteur` puis `Equipement` ;
  - `numero_serie` a le même identifiant depuis `Thermometre` et depuis `Camera`.

## 2. Valeurs, chaînes et Stores

- [x] 2.1 Écrire `src/value.rs` : `Value<'a>`, encodage de `Date` (`année × 512 + mois × 32 + jour`) et de `DateTime` (microsecondes). Vérifier par des tests :
  - aller-retour exact pour des dates et des instants, années négatives comprises ;
  - l'ordre des entiers obtenus est l'ordre chronologique.
- [x] 2.2 Écrire `src/strings.rs` : arène en blocs de 1 Mio jamais agrandis, `StrRef` sur 8 octets, bloc dédié pour une chaîne plus longue qu'un bloc. Vérifier par des tests :
  - lecture d'une chaîne vide, d'une chaîne accentuée (UTF-8 sur plusieurs octets) et d'une chaîne plus longue qu'un bloc ;
  - des milliers de chaînes réparties sur plusieurs blocs ;
  - une ancienne référence reste lisible après d'autres ajouts.
- [x] 2.3 Écrire `src/store/mod.rs` : `ObjectId`, `Position`, `StoreError` et le trait d'essai `ObjectStore` (design.md, Decision 5). Vérifier : `cargo build` et `cargo clippy --all-targets -- -D warnings` réussissent.
- [x] 2.4 Écrire V1 (`src/store/boxed.rs`) et la suite de tests commune `tests/store.rs` : des fonctions génériques sur `S: ObjectStore`, instanciées pour V1 par une petite macro commentée, avec le test `assert_send`. Vérifier : un test par scénario de la spécification `object-store` passe pour V1.
- [x] 2.5 Écrire V2 (`src/store/heap.rs`) et instancier la suite commune pour V2. Vérifier :
  - tous les tests de la suite passent pour V2 ;
  - un test crée assez d'objets pour remplir plusieurs blocs, sans qu'aucun enregistrement ne chevauche deux blocs ;
  - un test distingue un entier non renseigné d'un entier qui vaut 0.

## 3. Constructeur et lecteur JSON

- [x] 3.1 Écrire `src/builder.rs` : `Builder<S>`, pile de contexte, conversions par `set_text`, valeurs typées, `LoadError` et `DataPath`. Vérifier, pour V1 et V2, un test par scénario des exigences « Construction des objets dans l'ordre du fichier », « Conversion des valeurs texte » et « Valeurs déjà typées » de `data-loading`, ainsi que l'affichage d'un chemin (`/stations/0/equipements/1/mesures/2/valeur`).
- [x] 3.2 Écrire `examples/reseau-meteo/reseau-exemple.json`, à la main (design.md, Decision 10). Écrire aussi `src/json.rs` dans `ortelia-service` : `read_json`, `read_json_slice`, les `DeserializeSeed`, `_type_` connu, en premier ou rejoué, l'erreur du constructeur conservée, et la conversion des erreurs de syntaxe. Ajouter serde et serde_json à `ortelia-service`. Vérifier, pour V1 et V2, un test par scénario des exigences de lecture de `data-loading`, ainsi que le scénario « Modèle et exemple chargés » de `storage-benchmark` (objets à la profondeur 4).

## 4. Générateur de données

- [x] 4.1 Écrire `cargo xtask gen-data` (`xtask/src/gen_data.rs`) : options `--size` (suffixes K, M, G), `--seed`, `--text`, `--late-type`, `--out` ; écriture en flux qui compte les octets ; budget vérifié avant chaque objet, avec fermeture propre ; stations avec leurs équipements, mesures, annotations et interventions. Ajouter fastrand et serde_json à `xtask`. Vérifier par des tests :
  - deux générations de 200 Ko avec la même graine sont identiques ;
  - une génération de 1 Mo fait au moins 1 Mo et moins de 1,1 Mo.
- [x] 4.2 Compléter le générateur : zones (polygones et cercles), mailles et leurs coins, `--late-type`, `--text 0`. Ajouter `ortelia-model`, `ortelia-service` et `tempfile` aux dépendances de développement de `xtask`. Vérifier par des tests :
  - un fichier généré avec `--late-type 0.1` se charge avec le modèle d'essai ;
  - avec `--text 0`, ni `description`, ni `compte_rendu`, ni `texte` n'apparaissent ;
  - `cargo xtask check-layers` n'affiche aucune violation.
- [x] 4.3 Documenter `cargo xtask gen-data` dans `CLAUDE.md` : options, dossier `~/s1/`, et pourquoi pas `/mnt/c`. Vérifier : la commande documentée génère un fichier de 10 Mo dans `~/s1/`, telle qu'elle est écrite.

## 5. Programme de mesure

- [x] 5.1 Créer l'exemple `s1` de `ortelia-service` (`examples/s1/`, `[[example]]` avec `test = true`) :
  - modes `all` et `case`, avec un processus par cas ;
  - rapport en tableau Markdown ;
  - lecture de `VmHWM` et `VmRSS` sous Linux, et « non disponible » ailleurs ;
  - fastrand en dépendance de développement.

  Vérifier par des tests que le rapport se construit à partir de lignes de résultats, y compris avec la mémoire non disponible.
- [x] 5.2 Ajouter les cas de chargement : plancher (`IgnoredAny`), V0 (`serde_json::Value`), V1 et V2, en flux et depuis un tampon ; mesures C1, M1 et M2. Vérifier par un test : chaque cas rend ses mesures sur `reseau-exemple.json`.
- [x] 5.3 Ajouter l'allocateur qui compte (module avec `#![allow(unsafe_code)]` et son commentaire, activé par un `AtomicBool`), et la mesure M3. Vérifier par un test : le nombre d'allocations est positif quand le comptage est actif, et ne bouge pas quand il est inactif.
- [x] 5.4 Ajouter les mesures d'accès et de navigation A1 à A7 pour V1 et V2 : médiane de 5 passes, `black_box`, graine fixe. Vérifier par un test : chaque mesure rend une valeur sur `reseau-exemple.json`, pour les deux Stores.
- [x] 5.5 Ajouter les mesures de modification E1 à E4 pour V1 et V2. Vérifier par un test : chaque mesure rend une valeur sur `reseau-exemple.json`, et le Store reste cohérent après (la station supprimée n'est plus vivante ; les 1 000 mailles créées sont dans leur zone).
- [x] 5.6 Documenter la commande de mesure dans `CLAUDE.md`. Vérifier : `cargo run --release -p ortelia-service --example s1 -- all ~/s1/<fichier de 10 Mo>` affiche un tableau complet, tel que documenté.

## 6. Essai S1 : mesures et décision

- [x] 6.1 Générer dans `~/s1/` les fichiers de design.md (Decision 12) : profil texte de 500 Mo, profil numérique de 500 Mo, 500 Mo avec `--late-type 0.1`, 100 Mo pour V0. Lancer `all` sur chacun, en release. Vérifier : un tableau complet pour chaque fichier, sans cas en échec.
- [x] 6.2 Écrire `docs/essais/s1-stockage.md` : conditions, fichiers (taille, objets par classe), tableaux, analyse, seuils et leur justification, choix. Présenter les résultats au porteur du projet. Vérifier : le document est complet, et le porteur du projet valide le choix.
- [x] 6.3 Supprimer la variante écartée et le trait `ObjectStore` : le Store retenu devient le type concret `Store`, le constructeur et le lecteur JSON ne sont plus génériques, la suite commune devient celle du Store, et le programme de mesure garde le plancher, V0 et le Store. Vérifier :
  - `cargo test --workspace` passe ;
  - `ObjectStore` n'apparaît plus dans le code ;
  - `all` fonctionne toujours sur le fichier de 10 Mo.
- [x] 6.4 Mettre à jour `docs/feuille-de-route-annuaire.md` (résultat de S1, lien vers le document de résultats) et l'état du projet dans `CLAUDE.md`. Vérifier : les deux documents citent le Store retenu sans contradiction avec `docs/essais/s1-stockage.md`.

## 7. Vérification finale

- [x] 7.1 Lancer `/code-review` sur le diff complet et corriger les problèmes retenus. Vérifier : aucun problème important restant.
- [x] 7.2 Lancer `/simplify` et appliquer les simplifications retenues. Vérifier : le diff simplifié compile, et les tests passent.
- [x] 7.3 Lancer `/verifier` (`tools/check.sh` : format, Clippy pour Linux et pour la cible Windows, tests, règle des couches, test de fumée). Vérifier : toutes les étapes réussissent.

## Workflow follow-up

- Committer, pousser et ouvrir la pull request à la demande du porteur du projet. Après la fusion et une CI verte, archiver avec `/openspec-archive-change` : les spécifications `metamodel`, `object-store`, `data-loading` et `storage-benchmark` entrent alors dans `openspec/specs/`.
- Le premier critère « Fini quand » de l'incrément 1 (mesures de S1 consignées et choix du stockage arrêté) est alors rempli. Le second changement de l'incrément, « modèle et données », reste à proposer.
