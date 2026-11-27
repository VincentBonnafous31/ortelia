# Spec Delta

## Purpose

Placer des cellules dans une grille en flux, des colonnes dont certaines s'élargissent, et calculer la largeur de chaque colonne depuis les largeurs minimales des cellules et la largeur disponible. Ce composant de mise en page est indépendant de ce que contiennent les cellules, et réutilisable par tout widget.

## ADDED Requirements

### Requirement: Colonnes de la grille
Une grille SHALL avoir au moins une colonne. Chaque colonne SHALL avoir un poids d'élargissement, un nombre fini positif ou nul, nul par défaut. Seules les colonnes de poids strictement positif reçoivent la largeur disponible en plus de leur minimum.

#### Scenario: Colonne sans poids
- **WHEN** une grille a deux colonnes, la première sans poids et la seconde de poids 1
- **THEN** la première a le poids 0 et ne s'élargit pas, la seconde s'élargit

### Requirement: Cellules placées en flux
Les cellules SHALL être placées dans leur ordre, de gauche à droite, à partir de la première colonne de la première ligne. Une cellule SHALL occuper, sur une seule ligne, autant de colonnes consécutives que son étendue, 1 par défaut, et la cellule suivante SHALL commencer à la colonne qui suit.

#### Scenario: Quatre cellules sur deux colonnes
- **WHEN** quatre cellules d'étendue 1 sont placées dans une grille de deux colonnes
- **THEN** elles sont, dans l'ordre, en ligne 0 colonne 0, ligne 0 colonne 1, ligne 1 colonne 0 et ligne 1 colonne 1, et la grille a deux lignes

#### Scenario: Cellule sur trois colonnes
- **WHEN** une cellule d'étendue 1 puis une cellule d'étendue 3 sont placées dans une grille de quatre colonnes
- **THEN** la première est en ligne 0 colonne 0, la seconde en ligne 0 et occupe les colonnes 1 à 3

### Requirement: Passage automatique à la ligne
Une cellule SHALL commencer une nouvelle ligne quand la ligne courante est pleine, ou quand il reste moins de colonnes sur la ligne courante que son étendue. Les colonnes laissées libres en fin de ligne restent vides.

#### Scenario: Ligne pleine
- **WHEN** trois cellules d'étendue 1 sont placées dans une grille de deux colonnes
- **THEN** la troisième est en ligne 1 colonne 0

#### Scenario: Cellule qui ne tient plus
- **WHEN** trois cellules d'étendue 1 puis une cellule d'étendue 2 sont placées dans une grille de quatre colonnes
- **THEN** les trois premières sont en ligne 0, colonnes 0 à 2, la colonne 3 de la ligne 0 reste vide, et la quatrième est en ligne 1 et occupe les colonnes 0 et 1

### Requirement: Nouvelle ligne demandée
Une cellule qui demande une nouvelle ligne SHALL commencer à la première colonne de la ligne suivante si la ligne courante a déjà une cellule. Elle SHALL NOT créer de ligne vide quand la ligne courante n'a encore aucune cellule.

#### Scenario: Nouvelle ligne après une cellule
- **WHEN** deux cellules d'étendue 1 sont placées dans une grille de quatre colonnes, la seconde demandant une nouvelle ligne
- **THEN** la première est en ligne 0 colonne 0, la seconde en ligne 1 colonne 0

#### Scenario: Nouvelle ligne après une ligne pleine
- **WHEN** une cellule d'étendue 2 puis une cellule d'étendue 1 qui demande une nouvelle ligne sont placées dans une grille de deux colonnes
- **THEN** la seconde est en ligne 1 colonne 0, et la grille a deux lignes

### Requirement: Largeur minimale des colonnes
La largeur minimale d'une colonne SHALL être la plus grande des largeurs minimales des cellules d'étendue 1 qu'elle contient, ou 0 si elle n'en contient pas. Les largeurs minimales des cellules sont fournies par l'interface, qui les mesure. La largeur minimale de la grille SHALL être la somme des largeurs minimales de ses colonnes et des espacements entre colonnes, fournis par l'interface.

#### Scenario: Colonne des libellés
- **WHEN** une grille de deux colonnes, d'espacement 8, a en colonne 0 des cellules de largeurs minimales 60 et 80, et en colonne 1 des cellules de largeurs minimales 100 et 120
- **THEN** les largeurs minimales des colonnes sont 80 et 120, et celle de la grille 208

### Requirement: Cellule plus large que ses colonnes
Quand une cellule d'étendue supérieure à 1 est plus large que ses colonnes, espacements compris, la différence SHALL être ajoutée aux largeurs minimales de celles de ses colonnes qui s'élargissent, au prorata de leurs poids. Si aucune d'elles ne s'élargit, elle SHALL être ajoutée à la dernière de ses colonnes.

#### Scenario: Différence donnée à la colonne qui s'élargit
- **WHEN** une grille a deux colonnes, de poids 0 et 1, d'espacement 10, une cellule de largeur minimale 60 en colonne 0 et une cellule d'étendue 2 de largeur minimale 300
- **THEN** les largeurs minimales des colonnes sont 60 et 230

#### Scenario: Différence donnée à la dernière colonne
- **WHEN** une grille a deux colonnes de poids 0, sans espacement, une cellule de largeur minimale 50 dans chaque colonne et une cellule d'étendue 2 de largeur minimale 200
- **THEN** les largeurs minimales des colonnes sont 50 et 150

### Requirement: Largeur disponible répartie
Quand la largeur disponible dépasse la largeur minimale de la grille, le surplus SHALL être réparti entre les colonnes qui s'élargissent, au prorata de leurs poids, et la grille occupe alors toute la largeur disponible. Sans colonne qui s'élargit, chaque colonne SHALL garder sa largeur minimale. La position de chaque colonne SHALL être la somme des largeurs et des espacements des colonnes qui la précèdent.

#### Scenario: Colonne des valeurs élargie
- **WHEN** une grille a deux colonnes, de poids 0 et 1, de largeurs minimales 80 et 120, d'espacement 8, et dispose de 400 points
- **THEN** les colonnes ont pour largeurs 80 et 312, et pour positions 0 et 88

#### Scenario: Deux colonnes de poids différents
- **WHEN** une grille a deux colonnes, de poids 1 et 3, de largeurs minimales 50 et 50, sans espacement, et dispose de 300 points
- **THEN** les colonnes ont pour largeurs 100 et 200

#### Scenario: Aucune colonne qui s'élargit
- **WHEN** une grille a deux colonnes de poids 0, de largeurs minimales 80 et 100, d'espacement 10, et dispose de 500 points
- **THEN** les colonnes ont pour largeurs 80 et 100, et la grille occupe 190 points

### Requirement: Largeur disponible insuffisante
Quand la largeur disponible est inférieure à la largeur minimale de la grille, chaque colonne SHALL garder sa largeur minimale, et la grille SHALL être plus large que la largeur disponible.

#### Scenario: Vue trop étroite
- **WHEN** une grille a deux colonnes, de poids 0 et 1, de largeurs minimales 80 et 150, d'espacement 10, et dispose de 200 points
- **THEN** les colonnes ont pour largeurs 80 et 150, et la grille occupe 240 points
