# Spec Delta

## MODIFIED Requirements

### Requirement: Capture de la fenêtre principale
La fenêtre principale SHALL avoir un test de capture, rendu par l'application egui complète ouverte sur `examples/annuaire`, avec son contenu fourni par la présentation : le split de l'annuaire, avec à gauche la table des personnes, ses en-têtes, son tri initial et son filtre vide, la séparation, et à droite le formulaire vide de `Personne`.

#### Scenario: Fenêtre principale conforme
- **WHEN** on rend la fenêtre principale de l'annuaire sur la plateforme de rendu retenue
- **THEN** l'image concorde avec sa référence `main_window`

## ADDED Requirements

### Requirement: Capture du formulaire
Le formulaire SHALL avoir un test de capture, rendu par l'application egui complète ouverte sur `examples/annuaire`. La séparation y est placée aux parts 0,4 et 0,6, et la vue `proprietes` montre la personne `Dupont`. L'image fait voir les libellés, les valeurs, la case, la liste, le groupe dans son cadre titré et le séparateur.

#### Scenario: Formulaire conforme
- **WHEN** on rend la fenêtre de l'annuaire dont le formulaire montre la personne `Dupont`, sur la plateforme de rendu retenue
- **THEN** l'image concorde avec sa référence `form`
