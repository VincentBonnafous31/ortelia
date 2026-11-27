# config-schemas Specification

## Purpose

Fournir un JSON Schema pour chaque type de fichier de configuration écrit à la main (`app.json`, `*.model.json`), pour la validation et l'autocomplétion dans l'éditeur de texte, toujours conforme à ce que le système accepte réellement.

## Requirements

### Requirement: Schémas générés depuis le code
La commande `cargo xtask gen-schemas` SHALL écrire `schemas/app.schema.json` et `schemas/model.schema.json`, générés depuis les définitions que le système utilise pour lire ces fichiers. Les schémas MUST refléter les règles de lecture : clés autorisées, clés obligatoires, types des valeurs, et refus des clés inconnues.

#### Scenario: Génération des deux schémas
- **WHEN** on lance `cargo xtask gen-schemas`
- **THEN** les fichiers `schemas/app.schema.json` et `schemas/model.schema.json` sont écrits, chacun est un JSON Schema valide, et chacun interdit les clés inconnues à la racine

#### Scenario: Clés du schéma de `app.json`
- **WHEN** on lit `schemas/app.schema.json`
- **THEN** il décrit les clés `$schema`, `formatVersion`, `id`, `name`, `languages`, `labels`, `model`, `sources`, `presentations` et `window`, et une source y a les clés obligatoires `id`, `format`, `file` et `target`

#### Scenario: Contenu polymorphe de la fenêtre
- **WHEN** on lit `schemas/app.schema.json`
- **THEN** le contenu de la fenêtre y est décrit comme un objet dont `_type_` vaut `View` ou `Split`, les enfants d'un split comme des contenus de ce même type, et le widget d'une vue comme un objet dont `_type_` vaut `Table` ou `Empty`

#### Scenario: Direction d'un split
- **WHEN** on lit `schemas/app.schema.json`
- **THEN** la direction d'un split n'y admet que les valeurs `horizontal` et `vertical`

#### Scenario: Génération reproductible
- **WHEN** on lance deux fois `cargo xtask gen-schemas` sans changer le code
- **THEN** les fichiers écrits sont identiques, octet pour octet

### Requirement: Schémas à jour
La commande `cargo xtask gen-schemas --check` SHALL comparer les schémas générés à ceux du dépôt sans rien écrire. Elle MUST se terminer avec le code 1 si l'un d'eux diffère ou manque, en citant le fichier et la commande qui le régénère, et avec le code 0 sinon.

#### Scenario: Schéma périmé
- **WHEN** une clé est ajoutée aux définitions de `app.json` sans que `schemas/app.schema.json` soit régénéré
- **THEN** `cargo xtask gen-schemas --check` se termine avec le code 1 et cite `schemas/app.schema.json`

#### Scenario: Schémas à jour
- **WHEN** les schémas du dépôt viennent d'être générés
- **THEN** `cargo xtask gen-schemas --check` se termine avec le code 0

### Requirement: Fichiers d'exemple reliés à leur schéma
Les fichiers `app.json` et `*.model.json` de `examples/` SHALL déclarer leur schéma par la clé `$schema`, avec un chemin relatif vers `schemas/`.

#### Scenario: Exemple de l'annuaire
- **WHEN** on lit `examples/annuaire/app.json` et `examples/annuaire/annuaire.model.json`
- **THEN** leurs clés `$schema` désignent `../../schemas/app.schema.json` et `../../schemas/model.schema.json`, qui existent
