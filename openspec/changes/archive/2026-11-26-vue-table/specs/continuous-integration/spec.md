# Spec Delta

## MODIFIED Requirements

### Requirement: Test de fumée de l'exécutable
À chaque exécution, pull requests comprises, la CI SHALL lancer l'exécutable `ortelia` sur l'application `examples/annuaire`, avec un affichage, et vérifier le scénario « Démarrage réel » de `app-launch`. Sous Linux, dans un écran virtuel avec rendu logiciel, un échec MUST faire échouer l'exécution. Sous Windows, le test est exécuté et son résultat reste visible, mais un échec ne fait pas échouer l'exécution.

#### Scenario: Échec du test de fumée sous Linux
- **WHEN** l'exécutable ne dessine pas sa première image dans l'écran virtuel, ou qu'Ortelia journalise une erreur
- **THEN** l'exécution de la CI échoue

#### Scenario: Échec du test de fumée sous Windows
- **WHEN** le test de fumée échoue sous Windows et que toutes les autres vérifications réussissent
- **THEN** l'exécution de la CI réussit, et l'étape du test de fumée Windows apparaît en échec dans le détail de l'exécution
