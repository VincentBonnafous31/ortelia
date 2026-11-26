# Spec Delta

## ADDED Requirements

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
