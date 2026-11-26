# Spec Delta

## MODIFIED Requirements

### Requirement: Colonnes de la présentation
Les colonnes SHALL être celles de la table de la présentation de la classe affichée, dans leur ordre. Sans présentation pour cette classe, celle de la classe mère la plus proche qui en a une SHALL s'appliquer. Quand l'état de session de la vue est restauré, l'ordre des colonnes SHALL être celui qu'il enregistre (voir `session-state`).

#### Scenario: Colonnes configurées
- **WHEN** la présentation de `Personne` déclare les colonnes `nom`, `prenom` et `date_naissance`
- **THEN** la table a ces trois colonnes, dans cet ordre

#### Scenario: Présentation héritée
- **WHEN** la classe affichée n'a pas de présentation et que sa classe mère en a une
- **THEN** la table a les colonnes de la présentation de la classe mère

### Requirement: Tri initial
Au démarrage, la table SHALL être triée selon le tri de l'état de session de sa vue, s'il est restauré (voir `session-state`), même vide ; sinon selon le `sort` de sa présentation, s'il y en a un ; sinon dans l'ordre de la liste.

#### Scenario: Tri initial de l'annuaire
- **WHEN** la présentation de `Personne` déclare le tri `nom` croissant, sans état de session
- **THEN** la première ligne est Bernard et la dernière Robert

#### Scenario: Tri de l'état de session
- **WHEN** la présentation de `Personne` déclare le tri `nom` croissant, et que l'état de session restauré enregistre le tri `nom` décroissant
- **THEN** la première ligne est Robert et la dernière Bernard

#### Scenario: Tri retiré par l'utilisateur
- **WHEN** la présentation de `Personne` déclare le tri `nom` puis `prenom`, et que l'état de session restauré enregistre un tri vide
- **THEN** la table est dans l'ordre de la liste

### Requirement: Largeur des colonnes
Chaque colonne SHALL avoir une largeur : celle de l'état de session de sa vue, s'il est restauré et l'enregistre (voir `session-state`), sinon celle de sa présentation, sinon une largeur par défaut selon son type. Le redimensionnement d'une colonne SHALL changer la largeur de cette colonne seulement, et cette largeur SHALL la suivre quand elle change de place.

#### Scenario: Largeur qui suit sa colonne
- **WHEN** on élargit la colonne `nom` à 200 points, puis qu'on la déplace en troisième position
- **THEN** la colonne `nom` a toujours 200 points de large, et les autres colonnes gardent leur largeur

#### Scenario: Largeur de l'état de session
- **WHEN** la présentation donne 140 points à la colonne `nom`, et que l'état de session restauré lui en donne 180
- **THEN** la colonne `nom` a 180 points de large
