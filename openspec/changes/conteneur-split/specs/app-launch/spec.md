# Spec Delta

## MODIFIED Requirements

### Requirement: Contenu de la fenêtre principale fourni par la présentation
La couche de présentation SHALL fournir le titre de la fenêtre principale, sa taille initiale et son contenu, calculés depuis l'application ouverte : titre de `window`, taille de l'état de session s'il en donne une (voir `session-state`), sinon taille de `window`, et arbre de disposition de son contenu (voir `window-layout`). L'interface ne calcule pas ces éléments.

#### Scenario: Textes de la fenêtre principale
- **WHEN** la présentation est créée, sans interface et sans état de session, pour l'application `examples/annuaire` ouverte
- **THEN** le titre de la fenêtre vaut `Annuaire`, sa taille initiale 1000 × 700 points, et son contenu est le split `principal`, qui contient la table des personnes et la vue vide `proprietes`

#### Scenario: Taille de l'état de session
- **WHEN** la présentation est créée pour `examples/annuaire`, avec un état de session qui donne une fenêtre de 1200 × 800 points
- **THEN** le titre de la fenêtre vaut `Annuaire` et sa taille initiale 1200 × 800 points

#### Scenario: Application qui ne s'ouvre pas
- **WHEN** la présentation est créée pour une application dont l'ouverture a échoué
- **THEN** le titre de la fenêtre vaut `Ortelia`, et la fenêtre ne contient que la boîte d'erreur d'ouverture
