# Tâches : modele-et-donnees

Toutes les commandes `cargo` se lancent dans WSL2, avec `CARGO_TARGET_DIR=~/target/ortelia` (voir `CLAUDE.md`). Chaque tâche laisse le workspace compilable, Clippy sans avertissement et les tests au vert. « Un test par scénario » renvoie aux scénarios des spécifications du changement.

## 1. Métamodèle

- [x] 1.1 Vérifier à la compilation les bornes des attributs et des références (`ModelError::InvalidBounds`), et les littéraux en double d'un énuméré (`ModelError::DuplicateLiteral`). Ajouter `Attribute::is_required()` et `Class::required_attributes()`, avec les attributs hérités (design.md, § 13). Vérifier : un test par scénario des exigences « Bornes cohérentes » et « Littéraux uniques dans un énuméré » de `metamodel`.
- [x] 1.2 Ajouter la clé `$schema` à `ModelFile`, puis `Model::load(&[NamedModelFile])` : version `0.1` vérifiée, et erreurs `ModelLoadError` qui citent le fichier (ligne et colonne pour la syntaxe, fichier de la classe en cause pour la compilation). Vérifier : un test par scénario des exigences « Erreurs situées dans leur fichier », « Version du format des fichiers de modèle » et « Clé `$schema` acceptée dans les fichiers de modèle ».
- [x] 1.3 Enrichir `examples/annuaire/annuaire.model.json` (classe `Racine`, énuméré `SituationFamiliale`, type dérivé `TexteLong`, nouveaux attributs de `Personne` : design.md, § 16) et compléter les dix personnes de `personnes.json`. Vérifier : le test du scénario « Modèle de l'annuaire » (modifié) passe, ainsi que les tests de `file.rs` sur ce fichier.

## 2. Store

- [x] 2.1 Ajouter `Store::descendants` (pile explicite, parent d'abord, enfants dans l'ordre) et `Store::delete_many` (un seul `retain` par liste, positions d'origine relevées, ancêtre et descendant dans le même lot traités une fois : design.md, § 14). Vérifier : un test par scénario des exigences « Parcours d'un sous-arbre » et « Suppression en lot » de `object-store`, dont l'arbre de 100 000 générations et le lot de 50 000 personnes en moins d'une seconde en compilation de test.

## 3. Document et transactions

- [x] 3.1 Écrire `src/document.rs` : `OwnedValue`, `Change`, `Transaction`, `Document::new` (racine concrète), `revision`, `changes_since` (`partition_point`), `store`, et le garde `Edit` (`create_child`, `set`, `delete`, `delete_many`, enregistrement dans `Drop`, `set` sans effet ignoré, racine non supprimable : design.md, §§ 3 et 4). Commenter sur `Document::edit` l'emprunt en écriture. Vérifier : un test par scénario des exigences « Racine du Document », « Transactions », « Changement de valeur enregistré », « Création et suppression enregistrées » et « Consultation des changements » de `change-tracking`.

## 4. Fragment et intégration

- [x] 4.1 Écrire `Shape` et `Fragment`. Faire construire un fragment par le `Builder` (`Builder::new(model, shape, base)`, `begin_head` conforme à la forme, seconde tête refusée pour une forme simple). Écrire `Store::absorb` et `Edit::replace` (vérification de la forme, du modèle et de la racine non vide ; ancien contenu libéré ; renumérotation ; `Change::Replaced` : design.md, §§ 6 et 7). Écrire la fonction de test de cohérence d'un Store (design.md, Risks). Adapter le lecteur JSON (forme de la racine, même comportement qu'avant), l'exemple `s1` (fragment de racine intégré dans un `Document` ; mesures A par `document.store()`, mesures E par des transactions) et le test de `gen-data`. Ajouter à `docs/essais/s1-stockage.md` la phrase sur les mesures E (design.md, § 17). Vérifier :
  - un test par scénario des exigences « Intégration d'un fragment dans une branche » et « Fragment non conforme refusé » de `change-tracking`, avec la cohérence vérifiée après chaque intégration, y compris après deux intégrations successives dans la même branche ;
  - un test par scénario des exigences « Construction des objets dans l'ordre du fichier » (modifiée) et « Fragment transférable entre threads » de `data-loading` ;
  - `cargo run --release -p ortelia-service --example s1 -- all ~/s1/reseau-10M.json` affiche un tableau complet.
- [x] 4.2 Faire lire au lecteur JSON la valeur attendue par la forme (objet pour la racine ou une référence simple, tableau pour une liste), avec le chemin de base dans les erreurs. Vérifier : un test par scénario des exigences « Lecture JSON avec la correspondance identité », « Données non conformes refusées » et « Lecture en flux ou depuis un tampon » (toutes trois modifiées) de `data-loading`.
- [x] 4.3 Passer en `pub(crate)` les méthodes de modification du `Store` et déplacer `tests/store.rs` et `tests/common/mod.rs` dans `src/store/tests.rs` (design.md, § 2). Ajouter le doc-test `compile_fail` sur `Document::store()`. Vérifier :
  - le test du scénario « Lecture par le Store » passe ;
  - le doc-test `compile_fail` passe (`cargo test --doc -p ortelia-model`) ;
  - aucun appel à `create_root`, `create_child`, `set`, `delete` ou `delete_many` d'un `Store` ne subsiste hors de `ortelia-model`.

## 5. Erreurs rassemblées et diagnostics

- [x] 5.1 Faire rassembler les erreurs par le `Builder` : méthodes qui enregistrent l'erreur et rendent `bool` pour `begin_head` et `begin_object`, positions du fichier, plafond de 100 et total, diagnostics des attributs obligatoires dans `end_object`, et `finish() -> Result<Loaded, LoadFailure>`. Ajouter la valeur fautive à `LoadError` (40 caractères, puis « … »), un `Display` sans la valeur et `user_message()` avec (design.md, §§ 9 et 10). Vérifier : des tests du `Builder` sur le plafond, le total, l'ordre des erreurs, les diagnostics et les deux formes d'une erreur, dont les scénarios de l'exigence « Valeur fautive absente du journal » de `data-loading`.
- [x] 5.2 Faire continuer le lecteur JSON après une erreur de données : `IgnoredAny` pour une clé inconnue et pour le contenu d'un objet refusé, tampon d'un `_type_` tardif jeté, arrêt seulement sur une erreur de syntaxe ou d'entrée-sortie. Adapter l'exemple `s1` et le test de `gen-data` à `Loaded`. Vérifier : un test par scénario des exigences « Erreurs rassemblées » et « Attribut obligatoire absent » de `data-loading` ; les tests de l'exemple `s1` passent.

## 6. `app.json`

- [x] 6.1 Écrire dans `ortelia-config` les types de `app.json` (`AppFile`, `ModelSection`, `SourceDef`, `SourceFormat`, clé `$schema`) et `AppFile::parse`, avec la vérification de `formatVersion` (`0.3`) et des `ConfigError` qui citent la clé, la ligne et la colonne. Ajouter serde et serde_json à `ortelia-config`. Réduire `examples/annuaire/app.json` au format 0.3 (`model.root` à `Racine`, cible `/annuaire/personnes` : design.md, § 11). Vérifier : un test par scénario des exigences « Contenu de `app.json` », « Fichier `app.json` mal formé refusé », « Version du format » et « Clé `$schema` acceptée » de `app-config`.
- [x] 6.2 Écrire `AppFile::check(&Model)` : racine, résolution des cibles en `TargetPath`, conflits entre sources par préfixe des `RefId`, identifiants en double, toutes les erreurs rassemblées (design.md, § 8). Vérifier : un test par scénario des exigences « Racine vérifiée contre le modèle », « Cible d'une source vérifiée contre le modèle », « Un fichier, une branche » et « Erreurs de vérification rassemblées » de `app-config`, sur les modèles de l'annuaire et du réseau, et sur de petits modèles écrits dans les tests ; `cargo xtask check-layers` n'affiche aucune violation.

## 7. Ouverture d'une application

- [x] 7.1 Écrire `AppContext::open` dans `ortelia-service` (`src/app.rs`) : lecture de `app.json` et du modèle, vérification, `Document`, squelette en une transaction, puis lecture et intégration de chaque source, avec `SourceState` et `SourceStatus` (design.md, § 12). Ajouter tempfile aux dépendances de développement de `ortelia-service`. Vérifier : un test par scénario des exigences « Ouverture d'une application », « Racine et squelette créés à l'ouverture », « Alimentation des branches par les sources », « Une source en échec n'empêche pas l'ouverture » et « État de chaque source » de `app-context`, avec des applications du réseau écrites dans des dossiers temporaires.
- [x] 7.2 Journaliser l'ouverture (`info!`, `warn!` : design.md, § 12), en ajoutant tracing à `ortelia-service` et tracing-subscriber à ses dépendances de développement. Vérifier : le test du scénario « Valeur fautive non journalisée » passe, avec un subscriber qui capture le journal et des tests sérialisés.
- [x] 7.3 Mettre à jour `docs/architecture-couches.md`, § 9 : le schéma, D30, et les nouvelles D31 et D32 (design.md, § 19). Vérifier : le document décrit le fragment, la greffe dans une branche, le Document et les sources, sans contredire les spécifications du changement.

## 8. JSON Schema

- [x] 8.1 Ajouter schemars 1.2.3 (features `derive` et `preserve_order`) au `Cargo.toml` racine, la feature `json-schema` à `ortelia-model` et `ortelia-config`, et les dérivations `JsonSchema` conditionnelles sur les types des fichiers (design.md, § 15). Vérifier :
  - `cargo build --workspace` réussit ;
  - `cargo tree -p ortelia -i schemars` ne trouve pas schemars ;
  - `cargo xtask check-layers` n'affiche aucune violation.
- [x] 8.2 Écrire `cargo xtask gen-schemas [--check]` (`xtask/src/schemas.rs`, draft 7), générer `schemas/app.schema.json` et `schemas/model.schema.json`, et ajouter la clé `$schema` à `examples/annuaire/app.json`, à `examples/annuaire/annuaire.model.json`, à `examples/reseau-meteo/reseau.model.json` et à `examples/model-editor/metamodel.model.json`. Vérifier : un test par scénario des exigences de `config-schemas` (les deux schémas écrits et interdisant les clés inconnues, clés du schéma de `app.json`, génération reproductible, codes de `--check`, chemins `$schema` des exemples qui existent).
- [x] 8.3 Ajouter `cargo xtask gen-schemas --check` au job `checks` de `.github/workflows/ci.yml` (Linux) et à `tools/check.sh`. Documenter la commande et `schemas/` dans `CLAUDE.md`. Vérifier : `tools/check.sh` affiche l'étape des schémas en réussite, puis en échec après une modification volontaire d'un schéma, annulée ensuite.

## 9. Vérification finale

- [x] 9.1 Mettre à jour `docs/feuille-de-route-annuaire.md` (incrément 1 : `app.json`, « Lire puis intégrer », « Notification des modifications », résultat sous « Fini quand ») et l'état du projet dans `CLAUDE.md`. Vérifier : les deux documents décrivent l'incrément 1 terminé sans contredire `docs/architecture-couches.md` ni les spécifications.
- [x] 9.2 Lancer `/code-review` sur le diff complet et corriger les problèmes retenus. Vérifier : aucun problème important restant.
- [x] 9.3 Lancer `/simplify` et appliquer les simplifications retenues. Vérifier : le diff simplifié compile et les tests passent.
- [x] 9.4 Lancer `/verifier` (`tools/check.sh` : format, Clippy pour Linux et pour la cible Windows, tests, règle des couches, schémas, test de fumée). Vérifier : toutes les étapes réussissent.

## Workflow follow-up

- Committer, pousser et ouvrir la pull request à la demande du porteur du projet. Après la fusion et une CI verte, archiver avec `/openspec-archive-change` : les spécifications `change-tracking`, `app-config`, `app-context` et `config-schemas` entrent dans `openspec/specs/`, et les deltas de `metamodel`, `object-store`, `data-loading` et `continuous-integration` y sont fusionnés.
- Les critères « Fini quand » de l'incrément 1 sont alors tous remplis. Prochaine étape : l'incrément 2 (vue avec une table), qui commence par l'essai des captures d'images.
