# table-view Specification

## Purpose

Afficher les objets d'une liste de l'application dans une table : colonnes configurées ou déduites du modèle, tri multiple, filtre global, largeur et ordre des colonnes modifiables, avec des performances tenues jusqu'à 100 000 lignes.

## Requirements

### Requirement: Lignes de la table
Une vue table SHALL afficher une ligne par objet de la liste désignée par son chemin de données, dans l'ordre de la liste tant qu'aucun tri ni filtre ne s'applique.

#### Scenario: Personnes de l'annuaire
- **WHEN** la table de `examples/annuaire` est calculée, sans tri ni filtre
- **THEN** elle a dix lignes, dans l'ordre de `personnes.json`, la première étant Marie Dupont

### Requirement: Colonnes de la présentation
Les colonnes SHALL être celles de la table de la présentation de la classe affichée, dans leur ordre. Sans présentation pour cette classe, celle de la classe mère la plus proche qui en a une SHALL s'appliquer. Quand l'état de session de la vue est restauré, l'ordre des colonnes SHALL être celui qu'il enregistre (voir `session-state`).

#### Scenario: Colonnes configurées
- **WHEN** la présentation de `Personne` déclare les colonnes `nom`, `prenom` et `date_naissance`
- **THEN** la table a ces trois colonnes, dans cet ordre

#### Scenario: Présentation héritée
- **WHEN** la classe affichée n'a pas de présentation et que sa classe mère en a une
- **THEN** la table a les colonnes de la présentation de la classe mère

### Requirement: Colonnes déduites du modèle
Si aucune présentation ne s'applique, ou si elle ne déclare pas de table, les colonnes SHALL être les attributs monovalués de la classe affichée, hérités compris, dans l'ordre du modèle (attributs des classes mères d'abord), avec la largeur, l'alignement et le format par défaut de leur type.

#### Scenario: Annuaire sans présentation
- **WHEN** `app.json` ne déclare aucune présentation pour `Personne`
- **THEN** la table a neuf colonnes : `nom`, `prenom`, `date_naissance`, `nombre_enfants`, `taille`, `contact_autorise`, `situation`, `inscription` et `notes`

### Requirement: Texte des en-têtes
L'en-tête d'une colonne SHALL être le libellé de son attribut. Si l'attribut déclare une unité, elle SHALL suivre le libellé entre parenthèses.

#### Scenario: En-tête avec unité
- **WHEN** la colonne `taille`, d'unité `m`, a pour libellé `Taille`
- **THEN** son en-tête est `Taille (m)`

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

### Requirement: Tri sur une colonne
Un clic simple sur un en-tête SHALL trier sur cette seule colonne, en remplaçant le tri en cours. Les clics suivants sur la même colonne SHALL faire passer son sens de croissant à décroissant, puis à sans tri, puis de nouveau à croissant.

#### Scenario: Cycle du tri
- **WHEN** on clique trois fois sur l'en-tête `nom` d'une table sans tri
- **THEN** la table est triée par nom croissant, puis décroissant, puis revient à l'ordre de la liste

#### Scenario: Remplacement du tri
- **WHEN** la table est triée par `nom` puis `prenom`, et qu'on clique sur `date_naissance`
- **THEN** la table n'est plus triée que par `date_naissance` croissant

### Requirement: Tri multiple
Un Maj+clic sur un en-tête SHALL ajouter sa colonne en dernier critère de tri, en sens croissant, si elle n'est pas triée ; sinon il SHALL faire avancer son sens dans le même cycle, et la retirer du tri au passage à « sans tri », sans toucher aux autres critères.

#### Scenario: Ajout d'un second critère
- **WHEN** la table est triée par `situation` croissant, et qu'on fait Maj+clic sur `nom`
- **THEN** la table est triée par situation, puis par nom pour une même situation

#### Scenario: Retrait d'un critère
- **WHEN** la table est triée par `situation` croissant puis `nom` décroissant, et qu'on fait Maj+clic sur `nom`
- **THEN** la table n'est plus triée que par `situation` croissant

### Requirement: Indicateurs de tri
L'en-tête d'une colonne triée SHALL montrer le sens de son tri. Si plusieurs colonnes sont triées, il SHALL montrer aussi son rang, à partir de 1.

#### Scenario: Deux critères
- **WHEN** la table est triée par `situation` croissant puis `nom` décroissant
- **THEN** l'en-tête de `situation` indique croissant et le rang 1, celui de `nom` décroissant et le rang 2, et les autres aucun tri

### Requirement: Ordre des valeurs
Le tri SHALL être stable : à égalité sur tous les critères, l'ordre de la liste est conservé. Les valeurs absentes SHALL venir en dernier, quel que soit le sens. Les nombres, dates et dates-heures SHALL suivre leur ordre naturel, `false` SHALL précéder `true`, et les énumérés SHALL suivre l'ordre de leurs littéraux dans le modèle.

#### Scenario: Valeur absente en dernier
- **WHEN** la table est triée par `situation`, croissant puis décroissant, et qu'une personne n'a pas de situation
- **THEN** cette personne est la dernière ligne dans les deux cas

#### Scenario: Énumérés dans l'ordre du modèle
- **WHEN** la table est triée par `situation` croissant
- **THEN** les célibataires viennent d'abord, puis les mariés, les pacsés, les divorcés et les veufs, quel que soit leur libellé

#### Scenario: Égalité
- **WHEN** la table est triée par `contact_autorise` croissant
- **THEN** les personnes sans autorisation viennent d'abord, et dans chaque groupe les personnes gardent l'ordre de `personnes.json`

### Requirement: Comparaison des textes
Les textes SHALL être comparés après pliage : passage en minuscules, puis décomposition canonique (NFD) et retrait des marques combinantes, comme les accents. À égalité des textes pliés, le tri reste stable.

#### Scenario: Accents et casse
- **WHEN** on trie par ordre croissant les noms `Émile`, `de La Tour`, `Dupont`, `éric` et `Durand`
- **THEN** l'ordre est `de La Tour`, `Dupont`, `Durand`, `Émile`, `éric`

### Requirement: Filtre global
La table SHALL avoir un filtre de texte. Le texte saisi est plié comme pour le tri, puis découpé en mots aux espaces ; une ligne SHALL être gardée si chaque mot apparaît dans le texte affiché, plié, d'au moins une de ses colonnes. Un filtre vide ou fait d'espaces garde toutes les lignes.

#### Scenario: Deux mots dans deux colonnes
- **WHEN** on saisit `marie dup` dans le filtre de l'annuaire
- **THEN** seule la ligne de Marie Dupont reste

#### Scenario: Recherche dans le texte affiché
- **WHEN** on saisit `1985` dans le filtre, et que la date de naissance s'affiche `12/03/1985`
- **THEN** la ligne de Marie Dupont reste

#### Scenario: Recherche dans le libellé d'un énuméré
- **WHEN** le littéral `PACSE` a pour libellé `Pacsé(e)` et qu'on saisit `pacsé`
- **THEN** seule la personne pacsée reste

#### Scenario: Cases à cocher ignorées
- **WHEN** on saisit `true` dans le filtre, et qu'aucun texte affiché ne contient `true`
- **THEN** aucune ligne ne reste, bien que des personnes aient `contact_autorise` à vrai

### Requirement: Filtre et tri combinés
Le filtre SHALL s'appliquer à l'ordre trié : les lignes gardées restent dans l'ordre du tri. La table SHALL fournir le nombre de lignes affichées et le nombre total d'objets de la liste.

#### Scenario: Compteur
- **WHEN** le filtre ne garde que trois des dix personnes
- **THEN** la table annonce 3 lignes affichées sur 10

### Requirement: Largeur des colonnes
Chaque colonne SHALL avoir une largeur : celle de l'état de session de sa vue, s'il est restauré et l'enregistre (voir `session-state`), sinon celle de sa présentation, sinon une largeur par défaut selon son type. Le redimensionnement d'une colonne SHALL changer la largeur de cette colonne seulement, et cette largeur SHALL la suivre quand elle change de place.

#### Scenario: Largeur qui suit sa colonne
- **WHEN** on élargit la colonne `nom` à 200 points, puis qu'on la déplace en troisième position
- **THEN** la colonne `nom` a toujours 200 points de large, et les autres colonnes gardent leur largeur

#### Scenario: Largeur de l'état de session
- **WHEN** la présentation donne 140 points à la colonne `nom`, et que l'état de session restauré lui en donne 180
- **THEN** la colonne `nom` a 180 points de large

### Requirement: Ordre des colonnes
On SHALL pouvoir déplacer une colonne en glissant son en-tête sur une autre position. Les autres colonnes gardent leur ordre relatif ; le tri et le filtre restent inchangés.

#### Scenario: Déplacement d'une colonne
- **WHEN** on dépose l'en-tête de `date_naissance` (troisième) avant celui de `nom` (premier)
- **THEN** l'ordre des colonnes devient `date_naissance`, `nom`, `prenom`, et les lignes ne changent pas

### Requirement: Index recalculés à chaque révision
L'ordre et la sélection des lignes SHALL être recalculés quand la révision du Document change, et seulement dans ce cas ou quand le tri, le filtre ou les colonnes changent.

#### Scenario: Modification d'une valeur triée
- **WHEN** la table est triée par `nom`, et qu'une transaction du Document renomme Bernard en Zola
- **THEN** à l'image suivante, Zola est la dernière ligne

#### Scenario: Pas de recalcul sans changement
- **WHEN** la table est affichée plusieurs fois sans changement de révision, de tri, de filtre ni de colonnes
- **THEN** l'index des lignes n'est calculé qu'une fois

### Requirement: Seules les lignes visibles sont formatées
Le texte des cellules SHALL n'être calculé que pour les lignes demandées par l'interface, c'est-à-dire les lignes visibles, sauf pour le filtre, qui a besoin du texte de toutes les lignes.

#### Scenario: Défilement d'une grande table
- **WHEN** l'interface demande les lignes 120 à 160 d'une table de 50 000 personnes sans filtre
- **THEN** seules ces 41 lignes sont formatées

### Requirement: Performances du tri et du filtre
Sur une application de 100 000 personnes produite par le générateur de l'annuaire, en compilation optimisée, le tri sur une colonne de texte SHALL prendre moins de 200 ms. La préparation du texte de toutes les lignes pour le filtre SHALL prendre moins d'une seconde, et chaque application d'un filtre ensuite moins de 200 ms.

#### Scenario: Mesure sur 100 000 personnes
- **WHEN** on lance le programme de mesure de la table sur 100 000 personnes générées
- **THEN** le tri par `nom` prend moins de 200 ms, la préparation du filtre moins d'une seconde, et le filtre `dup` moins de 200 ms

### Requirement: Interactions traduites en intentions
L'interface SHALL traduire un clic sur un en-tête en demande de tri (avec l'indication de Maj), un glisser-déposer d'en-tête en déplacement de colonne, un redimensionnement en nouvelle largeur et une saisie dans le filtre en nouveau texte de filtre ; elle SHALL NOT trier, filtrer ni formater elle-même.

#### Scenario: Clic et Maj+clic
- **WHEN** dans un test sans écran, on clique sur l'en-tête `Nom`, puis on fait Maj+clic sur l'en-tête `Prénom`
- **THEN** l'interface émet une demande de tri sur `nom` sans ajout, puis une demande de tri sur `prenom` avec ajout

#### Scenario: Saisie du filtre
- **WHEN** dans un test sans écran, on saisit `dup` dans le champ de filtre
- **THEN** l'interface émet une demande de filtre de texte `dup`

#### Scenario: Glisser-déposer d'un en-tête
- **WHEN** dans un test sans écran, on glisse l'en-tête `Né le` sur celui de `Nom`
- **THEN** l'interface émet une demande de déplacement de la troisième colonne en première position, et aucune demande de tri

### Requirement: Affichage du view-model de la table
L'interface SHALL afficher les en-têtes, leurs indicateurs de tri, le texte des cellules, les cases à cocher et le compteur fournis par la présentation.

#### Scenario: Table de l'annuaire affichée
- **WHEN** dans un test sans écran, l'application de `examples/annuaire` dessine une image
- **THEN** les en-têtes `Nom` et `Prénom`, la cellule `Dupont` et le compteur `10 / 10` sont présents
