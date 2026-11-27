# Spec Delta

## MODIFIED Requirements

### Requirement: Capture de la fenêtre principale
La fenêtre principale SHALL avoir un test de capture, rendu par l'application egui complète ouverte sur `examples/annuaire`, avec son contenu fourni par la présentation : le split de l'annuaire, avec à gauche la table des personnes, ses en-têtes, son tri initial et son filtre vide, la séparation, et à droite la vue vide.

#### Scenario: Fenêtre principale conforme
- **WHEN** on rend la fenêtre principale de l'annuaire sur la plateforme de rendu retenue
- **THEN** l'image concorde avec sa référence `main_window`
