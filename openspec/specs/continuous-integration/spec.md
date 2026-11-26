# continuous-integration Specification

## Purpose

Vérifier automatiquement chaque modification du dépôt sous Linux et sous Windows (format, lint, tests, règle des couches) et produire les binaires des deux systèmes. La version Windows n'est compilée nulle part ailleurs.

## Requirements

### Requirement: Déclenchement
La CI SHALL s'exécuter à chaque poussée sur la branche `main`, à chaque pull request vers `main`, et à la demande depuis GitHub.

#### Scenario: Pull request
- **WHEN** une pull request vers `main` est ouverte ou mise à jour
- **THEN** les vérifications s'exécutent sur la pull request et leur résultat y est affiché

### Requirement: Vérifications sous Linux et Windows
Sous Linux et sous Windows, la CI SHALL exécuter : l'analyse clippy de tout le workspace, cibles de test comprises, avec les avertissements traités comme des erreurs ; tous les tests du workspace ; la vérification des couches. Le contrôle du format (`cargo fmt --check`) et la vérification que les JSON Schema générés sont à jour (`cargo xtask gen-schemas --check`) SHALL être exécutés au moins sous Linux. Un seul échec MUST faire échouer l'exécution.

#### Scenario: Code mal formaté
- **WHEN** une modification contient du code qui ne respecte pas `rustfmt`
- **THEN** l'exécution de la CI échoue sur le contrôle du format

#### Scenario: Avertissement clippy
- **WHEN** une modification introduit un avertissement clippy
- **THEN** l'exécution de la CI échoue sous Linux et sous Windows

#### Scenario: Violation de la règle des couches
- **WHEN** une modification fait dépendre `ortelia-presentation` de `egui`
- **THEN** l'exécution de la CI échoue sur la vérification des couches

#### Scenario: JSON Schema périmé
- **WHEN** une modification change les définitions de `app.json` sans régénérer `schemas/app.schema.json`
- **THEN** l'exécution de la CI échoue sur la vérification des schémas

### Requirement: Test de fumée de l'exécutable
À chaque exécution, pull requests comprises, la CI SHALL lancer l'exécutable `ortelia` sur l'application `examples/annuaire`, avec un affichage, et vérifier le scénario « Démarrage réel » de `app-launch`. Sous Linux, dans un écran virtuel avec rendu logiciel, un échec MUST faire échouer l'exécution. Sous Windows, le test est exécuté et son résultat reste visible, mais un échec ne fait pas échouer l'exécution.

#### Scenario: Échec du test de fumée sous Linux
- **WHEN** l'exécutable ne dessine pas sa première image dans l'écran virtuel, ou qu'Ortelia journalise une erreur
- **THEN** l'exécution de la CI échoue

#### Scenario: Échec du test de fumée sous Windows
- **WHEN** le test de fumée échoue sous Windows et que toutes les autres vérifications réussissent
- **THEN** l'exécution de la CI réussit, et l'étape du test de fumée Windows apparaît en échec dans le détail de l'exécution

### Requirement: Dépendances verrouillées
La CI MUST compiler avec les versions exactes de `Cargo.lock`, qui est versionné. Un `Cargo.lock` absent ou qui ne correspond plus aux manifestes fait échouer l'exécution.

#### Scenario: Cargo.lock non mis à jour
- **WHEN** une modification ajoute une dépendance à un `Cargo.toml` sans mettre à jour `Cargo.lock`
- **THEN** l'exécution de la CI échoue

### Requirement: Même version de Rust partout
La CI et le poste de développement SHALL utiliser la version de Rust et les composants (rustfmt, clippy) fixés à la racine du dépôt.

#### Scenario: Changement de version de Rust
- **WHEN** la version de Rust fixée à la racine du dépôt est modifiée
- **THEN** l'exécution suivante de la CI utilise cette version sous Linux et sous Windows

### Requirement: Binaires Linux et Windows
Après des vérifications réussies sur `main` ou à la demande, la CI SHALL compiler l'exécutable `ortelia` en mode release sous Linux et sous Windows, lancer `ortelia --version` avec chacun d'eux, et publier les deux exécutables comme artefacts de l'exécution, nommés `ortelia-linux-x86_64` et `ortelia-windows-x86_64`. Les pull requests ne produisent pas de binaires.

#### Scenario: Poussée sur main
- **WHEN** une modification est poussée sur `main` et que les vérifications réussissent
- **THEN** les artefacts `ortelia-linux-x86_64` et `ortelia-windows-x86_64` sont téléchargeables depuis l'exécution

#### Scenario: Binaire qui ne démarre pas
- **WHEN** l'exécutable produit ne répond pas correctement à `--version`
- **THEN** l'exécution de la CI échoue et aucun artefact n'est publié pour ce système

### Requirement: Tests de capture dans la CI
Sous Linux, la CI SHALL exécuter les tests de capture avec le rendu logiciel lavapipe, installé avant les tests, et un échec MUST faire échouer l'exécution. Quand une capture échoue, la CI SHALL publier comme artefact de l'exécution les nouvelles images et les images de différence, pour qu'on puisse les regarder sans relancer les tests. Sous Windows, les tests de capture sont ignorés.

#### Scenario: Capture différente sous Linux
- **WHEN** une modification change le rendu de la fenêtre principale sans mettre à jour sa référence
- **THEN** l'exécution de la CI échoue sous Linux, et un artefact de l'exécution contient la nouvelle image et l'image de différence de la fenêtre principale

#### Scenario: Captures réussies
- **WHEN** toutes les captures concordent avec leurs références
- **THEN** aucun artefact d'images de capture n'est publié

#### Scenario: Captures sous Windows
- **WHEN** la CI exécute les tests sous Windows
- **THEN** les tests de capture sont ignorés, et leur absence ne fait pas échouer l'exécution
