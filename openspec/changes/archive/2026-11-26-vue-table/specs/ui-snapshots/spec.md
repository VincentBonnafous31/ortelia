# Spec Delta

## MODIFIED Requirements

### Requirement: Capture de la fenêtre principale
La fenêtre principale SHALL avoir un test de capture, rendu par l'application egui complète ouverte sur `examples/annuaire`, avec son contenu fourni par la présentation : la table des personnes, avec ses en-têtes, son tri initial et son filtre vide.

#### Scenario: Fenêtre principale conforme
- **WHEN** on rend la fenêtre principale de l'annuaire sur la plateforme de rendu retenue
- **THEN** l'image concorde avec sa référence `main_window`

## ADDED Requirements

### Requirement: Capture de la boîte d'erreur d'ouverture
La boîte d'erreur d'une application qui ne s'ouvre pas SHALL avoir un test de capture, rendu par l'application egui complète, avec des erreurs fixées par le test.

#### Scenario: Boîte d'erreur conforme
- **WHEN** on rend la fenêtre d'une application dont l'ouverture a échoué avec deux erreurs, sur la plateforme de rendu retenue
- **THEN** l'image concorde avec sa référence `open_error`
