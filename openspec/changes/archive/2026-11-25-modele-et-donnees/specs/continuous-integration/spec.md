## MODIFIED Requirements

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
