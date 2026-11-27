# Spec Delta

## MODIFIED Requirements

### Requirement: Arbre de disposition fourni par la présentation
La couche de présentation SHALL fournir l'arbre de disposition de la fenêtre principale, calculé depuis `window.content` : pour chaque split, son identifiant, sa direction, ses enfants dans l'ordre de `app.json` et la part de chaque enfant ; pour chaque vue, son identifiant et son widget. L'interface ne calcule pas cet arbre.

#### Scenario: Disposition de l'annuaire
- **WHEN** la présentation est créée, sans interface et sans état de session, pour `examples/annuaire`
- **THEN** l'arbre de disposition est le split horizontal `principal`, dont les enfants sont la vue table `personnes`, de part 0,65, puis la vue formulaire `proprietes`, de part 0,35

#### Scenario: Splits imbriqués
- **WHEN** la présentation est créée pour une application dont le contenu est un split horizontal de la vue `a` et d'un split vertical `droite` des vues `b` et `c`
- **THEN** l'arbre de disposition a pour racine le split horizontal, dont le second enfant est le split vertical `droite`, d'enfants `b` puis `c`

### Requirement: Disposition fixe
La structure de la disposition SHALL être celle de `app.json` : l'interface SHALL NOT afficher d'onglets ni de barre de titre de vue, et SHALL NOT permettre de déplacer une vue par glisser-déposer. Seules les séparations des splits bougent.

#### Scenario: Pas d'onglets
- **WHEN** dans un test sans écran, l'application de `examples/annuaire` dessine une image
- **THEN** aucun onglet ni aucun bouton portant le nom d'une vue n'est présent

#### Scenario: Vue glissée
- **WHEN** dans un test sans écran, on glisse le pointeur depuis le milieu de la vue formulaire `proprietes` jusqu'au bord gauche de la fenêtre de l'annuaire
- **THEN** la table des personnes reste à gauche, avec la même largeur, et l'interface n'émet aucune intention de déplacement de séparation

### Requirement: Vue vide
Une vue dont le widget est `Empty` SHALL occuper sa place dans la disposition sans rien dessiner : ni texte, ni widget.

#### Scenario: Vue de droite de l'annuaire
- **WHEN** dans un test sans écran, une copie de l'annuaire dont la vue de droite `proprietes` est une vue vide, et non une vue formulaire, dessine une image
- **THEN** aucun texte ni widget n'est présent dans la partie droite de la fenêtre, au-delà de la séparation
