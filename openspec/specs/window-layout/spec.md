# window-layout Specification

## Purpose

Disposer le contenu de la fenêtre principale en conteneurs et en vues : des splits qui partagent l'espace entre leurs enfants selon des poids, des vues qui dessinent leur widget, et le suivi des séparations déplacées par l'utilisateur, dans une structure fixée par `app.json`.

## Requirements

### Requirement: Arbre de disposition fourni par la présentation
La couche de présentation SHALL fournir l'arbre de disposition de la fenêtre principale, calculé depuis `window.content` : pour chaque split, son identifiant, sa direction, ses enfants dans l'ordre de `app.json` et la part de chaque enfant ; pour chaque vue, son identifiant et son widget. L'interface ne calcule pas cet arbre.

#### Scenario: Disposition de l'annuaire
- **WHEN** la présentation est créée, sans interface et sans état de session, pour `examples/annuaire`
- **THEN** l'arbre de disposition est le split horizontal `principal`, dont les enfants sont la vue table `personnes`, de part 0,65, puis la vue vide `proprietes`, de part 0,35

#### Scenario: Splits imbriqués
- **WHEN** la présentation est créée pour une application dont le contenu est un split horizontal de la vue `a` et d'un split vertical `droite` des vues `b` et `c`
- **THEN** l'arbre de disposition a pour racine le split horizontal, dont le second enfant est le split vertical `droite`, d'enfants `b` puis `c`

### Requirement: Parts calculées depuis les poids
La part de chaque enfant d'un split SHALL être son poids divisé par la somme des poids du split, de sorte que les parts d'un split font 1. Sans poids dans `app.json`, les enfants SHALL avoir des parts égales.

#### Scenario: Poids en pourcentages
- **WHEN** un split déclare les poids `[65, 35]`
- **THEN** ses enfants ont les parts 0,65 et 0,35

#### Scenario: Poids quelconques
- **WHEN** un split de trois enfants déclare les poids `[2, 1, 1]`
- **THEN** ses enfants ont les parts 0,5, 0,25 et 0,25

#### Scenario: Poids absents
- **WHEN** un split de trois enfants ne déclare pas de poids
- **THEN** chacun de ses enfants a une part d'un tiers

### Requirement: Dessin selon les parts
L'interface SHALL dessiner chaque split en partageant son espace entre ses enfants selon leurs parts : côte à côte pour un split `horizontal`, l'un au-dessus de l'autre pour un split `vertical`, à l'épaisseur des séparations près. Quand la fenêtre change de taille, les parts SHALL être gardées.

#### Scenario: Table de l'annuaire à gauche
- **WHEN** dans un test sans écran, l'application de `examples/annuaire` dessine une image dans une fenêtre de 1000 points de large
- **THEN** la table des personnes est à gauche et occupe environ 650 points de large, à l'épaisseur de la séparation près

#### Scenario: Split vertical imbriqué
- **WHEN** dans un test sans écran, une application dont le contenu est un split horizontal de la table `a` et d'un split vertical, de poids `[1, 3]`, des tables `b` et `c`, dessine une image
- **THEN** `a` occupe toute la hauteur à gauche, `b` est à droite en haut et `c` à droite en bas, trois fois plus haute que `b`

#### Scenario: Fenêtre élargie
- **WHEN** dans un test sans écran, la fenêtre de l'annuaire passe de 1000 à 1500 points de large
- **THEN** la table des personnes occupe environ 975 points de large, et l'interface n'émet aucune intention de déplacement de séparation

### Requirement: Disposition fixe
La structure de la disposition SHALL être celle de `app.json` : l'interface SHALL NOT afficher d'onglets ni de barre de titre de vue, et SHALL NOT permettre de déplacer une vue par glisser-déposer. Seules les séparations des splits bougent.

#### Scenario: Pas d'onglets
- **WHEN** dans un test sans écran, l'application de `examples/annuaire` dessine une image
- **THEN** aucun onglet ni aucun bouton portant le nom d'une vue n'est présent

#### Scenario: Vue glissée
- **WHEN** dans un test sans écran, on glisse le pointeur depuis le milieu de la vue vide `proprietes` jusqu'au bord gauche de la fenêtre de l'annuaire
- **THEN** la table des personnes reste à gauche, avec la même largeur, et l'interface n'émet aucune intention de déplacement de séparation

### Requirement: Vue vide
Une vue dont le widget est `Empty` SHALL occuper sa place dans la disposition sans rien dessiner : ni texte, ni widget.

#### Scenario: Vue de droite de l'annuaire
- **WHEN** dans un test sans écran, l'application de `examples/annuaire` dessine une image
- **THEN** aucun texte ni widget n'est présent dans la moitié droite de la fenêtre, au-delà de la séparation

### Requirement: Séparation déplacée traduite en intention
Quand l'utilisateur déplace une séparation, l'interface SHALL émettre une intention qui désigne le split par son identifiant et donne les nouvelles parts de ses enfants ; elle SHALL NOT en émettre tant qu'aucune séparation n'a été déplacée. Un double-clic sur une séparation SHALL partager également l'espace entre ses deux voisins, et émettre aussi cette intention.

#### Scenario: Séparation glissée
- **WHEN** dans un test sans écran, on glisse la séparation de l'annuaire de 100 points vers la gauche, dans une fenêtre de 1000 points de large
- **THEN** l'interface émet une intention de déplacement du split `principal`, dont la première part vaut environ 0,55, et la table occupe environ 550 points de large

#### Scenario: Double-clic sur la séparation
- **WHEN** dans un test sans écran, on double-clique sur la séparation de l'annuaire
- **THEN** l'interface émet une intention de déplacement du split `principal` aux parts 0,5 et 0,5

#### Scenario: Aucune séparation déplacée
- **WHEN** dans un test sans écran, l'application de `examples/annuaire` dessine dix images sans interaction
- **THEN** l'interface n'émet aucune intention de déplacement de séparation

### Requirement: Parts reçues par la présentation
La présentation SHALL remplacer les parts d'un split par celles d'une intention de déplacement, ramenées à une somme de 1. Elle SHALL ignorer une intention qui désigne un split inconnu, qui n'a pas autant de valeurs que le split a d'enfants, ou dont une valeur n'est pas un nombre fini strictement positif.

#### Scenario: Parts reçues
- **WHEN** la présentation de l'annuaire reçoit pour le split `principal` les valeurs 1,1 et 0,9
- **THEN** les parts du split `principal` valent 0,55 et 0,45

#### Scenario: Nombre de valeurs faux
- **WHEN** la présentation de l'annuaire reçoit pour le split `principal` trois valeurs
- **THEN** les parts du split `principal` restent 0,65 et 0,35

#### Scenario: Valeur invalide
- **WHEN** la présentation de l'annuaire reçoit pour le split `principal` les valeurs 0 et 1
- **THEN** les parts du split `principal` restent 0,65 et 0,35

#### Scenario: Split inconnu
- **WHEN** la présentation de l'annuaire reçoit des parts pour un split `lateral`
- **THEN** l'arbre de disposition est inchangé

### Requirement: Intentions d'une vue adressées à cette vue
Une intention émise par une vue table SHALL désigner sa vue par son identifiant, et la présentation SHALL l'appliquer à cette seule vue.

#### Scenario: Deux tables des mêmes données
- **WHEN** une application montre les personnes de l'annuaire dans deux vues table, `a` et `b`, et qu'on trie la vue `a` par `prenom`
- **THEN** la vue `a` est triée par `prenom`, et la vue `b` garde le tri de la présentation de `Personne`
