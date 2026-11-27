# form-view Specification

## Purpose

Montrer un objet dans un formulaire : une hiérarchie de champs, de groupes, de séparateurs et de textes, placés dans une grille, décrite par la présentation de la classe ou déduite du modèle, avec un widget par type d'attribut, en lecture seule.

## Requirements

### Requirement: Vue formulaire
Une vue dont le widget est `Form` SHALL montrer le formulaire de la classe que déclare ce widget. Tant qu'aucun objet ne lui est donné, elle SHALL montrer ce formulaire vide.

#### Scenario: Vue de droite de l'annuaire
- **WHEN** la présentation est créée, sans interface et sans état de session, pour `examples/annuaire`
- **THEN** la vue `proprietes` est une vue formulaire de la classe `Personne`, sans objet montré

### Requirement: Formulaire de la présentation
Le formulaire d'une classe SHALL être celui de la présentation de cette classe ou, si elle n'en a pas, de la présentation de sa classe mère la plus proche qui en a une. Ses éléments SHALL être ceux de `form`, dans leur ordre.

#### Scenario: Formulaire de l'annuaire
- **WHEN** la présentation est créée pour `examples/annuaire`
- **THEN** le formulaire de `Personne` contient, dans l'ordre :
  - le groupe `Personne.identite`, qui contient les champs `nom`, `prenom` et `date_naissance` ;
  - les champs `nombre_enfants`, `taille`, `contact_autorise`, `situation` et `inscription` ;
  - un séparateur ;
  - le champ `notes`, en zone de texte.

#### Scenario: Formulaire hérité
- **WHEN** une vue formulaire montre la classe `Employe`, sous-classe de `Personne`, sans présentation, et que la présentation de `Personne` déclare un formulaire
- **THEN** la vue montre le formulaire de `Personne`

### Requirement: Formulaire déduit du modèle
Si aucune présentation ne s'applique, ou si elle ne déclare pas de formulaire, le formulaire SHALL avoir un champ par attribut de la classe, hérités compris, dans l'ordre du modèle (attributs des classes mères d'abord), chacun avec le widget par défaut de son type, dans une grille de deux colonnes : la première sans poids, pour les libellés, la seconde de poids 1, pour les widgets.

#### Scenario: Annuaire sans formulaire
- **WHEN** la présentation de `Personne` ne déclare pas de formulaire
- **THEN** le formulaire a neuf champs, `nom`, `prenom`, `date_naissance`, `nombre_enfants`, `taille`, `contact_autorise`, `situation`, `inscription` et `notes`, chacun sur sa ligne, son libellé en colonne 0 et son widget en colonne 1

### Requirement: Widget par défaut
Un champ sans widget déclaré SHALL recevoir le widget par défaut du type de son attribut : `TextField` pour un texte, `NumberField` pour un entier ou un décimal, `CheckBox` pour un booléen, `ComboBox` pour un énuméré, `DateField` pour une date et `DateTimeField` pour une date-heure.

#### Scenario: Widgets de l'annuaire déduit
- **WHEN** la présentation de `Personne` ne déclare pas de formulaire
- **THEN** les champs ont pour widgets :
  - `TextField` pour `nom`, `prenom` et `notes` ;
  - `DateField` pour `date_naissance` ;
  - `NumberField` pour `nombre_enfants` et `taille` ;
  - `CheckBox` pour `contact_autorise` ;
  - `ComboBox` pour `situation` ;
  - `DateTimeField` pour `inscription`.

### Requirement: Cellules d'un champ
Un champ SHALL occuper deux cellules de la grille de son conteneur, dans cet ordre : son libellé, puis son widget. Chacune SHALL avoir sa propre mise en page (étendue, nouvelle ligne), d'étendue 1 par défaut.

#### Scenario: Libellé et widget côte à côte
- **WHEN** un champ sans mise en page est le premier élément d'une grille de deux colonnes
- **THEN** son libellé est en ligne 0 colonne 0, et son widget en ligne 0 colonne 1

#### Scenario: Notes sous leur libellé
- **WHEN** la présentation est créée pour `examples/annuaire`
- **THEN** dans la grille du formulaire de `Personne`, le libellé de `notes` occupe les colonnes 0 et 1 d'une ligne, et sa zone de texte les colonnes 0 et 1 de la ligne suivante, la dernière de la grille

### Requirement: Cellule d'un groupe, d'un séparateur, d'un texte
Un groupe, un séparateur et un texte SHALL occuper chacun une cellule de la grille de leur conteneur. Sans étendue déclarée, un groupe ou un séparateur SHALL occuper toute la ligne, et un texte une colonne. Un groupe SHALL avoir sa propre grille pour ses éléments, de colonnes `[0, 1]` (sans poids, puis de poids 1) par défaut.

#### Scenario: Groupe de l'annuaire
- **WHEN** la présentation est créée pour `examples/annuaire`
- **THEN** le groupe `Personne.identite` occupe les colonnes 0 et 1 de la ligne 0 du formulaire, et sa grille, de deux colonnes, place `nom`, `prenom` et `date_naissance` sur ses lignes 0 à 2

#### Scenario: Deux groupes côte à côte
- **WHEN** un formulaire de quatre colonnes a deux groupes d'étendue 2
- **THEN** les deux groupes sont sur la ligne 0, l'un sur les colonnes 0 et 1, l'autre sur les colonnes 2 et 3

#### Scenario: Séparateur
- **WHEN** la présentation est créée pour `examples/annuaire`
- **THEN** le séparateur occupe à lui seul une ligne du formulaire, entre le champ `inscription` et le libellé de `notes`

### Requirement: Libellé d'un champ
Le libellé d'un champ SHALL être celui de son attribut, cherché comme l'en-tête d'une colonne (voir `labels`), ou, si le champ déclare une clé (`label`), le texte de cette clé. Si l'attribut déclare une unité, elle SHALL suivre le libellé entre parenthèses.

#### Scenario: Libellé avec unité
- **WHEN** l'attribut `taille`, d'unité `m`, a pour libellé `Taille`
- **THEN** le libellé de son champ est `Taille (m)`

#### Scenario: Clé déclarée par le champ
- **WHEN** un champ de `nom` déclare la clé `Personne.nom_usage`, de texte français `Nom d'usage`
- **THEN** le libellé du champ est `Nom d'usage`

### Requirement: Titre d'un groupe et texte fixe
Le titre d'un groupe et un texte fixe SHALL être le texte de leur clé dans la langue courante, ou `#clé#` s'il n'en a pas. Un groupe sans clé SHALL NOT avoir de titre.

#### Scenario: Titre du groupe de l'annuaire
- **WHEN** la langue courante est `fr`, et que `Personne.identite` a pour texte `Identité`
- **THEN** le titre du groupe est `Identité`

#### Scenario: Texte sans libellé
- **WHEN** un texte fixe a la clé `Personne.aide`, sans texte en français
- **THEN** il affiche `#Personne.aide#`

### Requirement: Valeurs de l'objet montré
Le widget d'un champ SHALL montrer la valeur de son attribut dans l'objet montré, formatée selon les règles de `value-formats`, avec le format de date du widget s'il en déclare un. Une zone de texte SHALL garder les sauts de ligne de la valeur.

#### Scenario: Personne Dupont
- **WHEN** la vue `proprietes` de l'annuaire montre la personne `Dupont`, la langue courante étant `fr`
- **THEN** les valeurs affichées sont :
  - `Dupont` pour `nom`, et `Marie` pour `prenom` ;
  - `12 mars 1985` pour `date_naissance`, au format `d MMMM yyyy` de son champ ;
  - `2` pour `nombre_enfants`, et `1,68` pour `taille` ;
  - `15/01/2024 09:30:00` pour `inscription`.

#### Scenario: Notes sur plusieurs lignes
- **WHEN** l'objet montré a pour `notes` `Ligne 1\nLigne 2`, dans une zone de texte
- **THEN** la zone de texte affiche deux lignes, `Ligne 1` puis `Ligne 2`

#### Scenario: Texte sur une ligne
- **WHEN** l'objet montré a pour `notes` `Ligne 1\nLigne 2`, dans un `TextField`
- **THEN** le champ affiche `Ligne 1 Ligne 2`

### Requirement: Booléen dans une case
Une case à cocher SHALL être cochée pour vrai et décochée pour faux. Pour un attribut sans valeur, elle SHALL être dans l'état indéterminé.

#### Scenario: Contact autorisé
- **WHEN** la vue `proprietes` de l'annuaire montre la personne `Dupont`
- **THEN** la case de `contact_autorise` est cochée

#### Scenario: Booléen absent
- **WHEN** l'objet montré n'a pas de valeur pour `contact_autorise`
- **THEN** sa case est dans l'état indéterminé

### Requirement: Énuméré en liste ou en boutons radio
Une liste déroulante (`ComboBox`) SHALL montrer le libellé du littéral de la valeur. Des boutons radio (`RadioGroup`) SHALL présenter un bouton par littéral, dans l'ordre de l'énuméré, chacun avec son libellé, et seul le bouton du littéral de la valeur SHALL être choisi. Pour un attribut sans valeur, la liste SHALL être vide, et aucun bouton SHALL NOT être choisi.

#### Scenario: Situation en liste
- **WHEN** la vue `proprietes` de l'annuaire montre la personne `Dupont`, la langue courante étant `fr`
- **THEN** la liste de `situation` montre `Marié(e)`

#### Scenario: Situation en boutons radio
- **WHEN** un champ `RadioGroup` de `situation` montre la personne `Dupont`, la langue courante étant `fr`
- **THEN** il a cinq boutons, `Célibataire`, `Marié(e)`, `Pacsé(e)`, `Divorcé(e)` et `Veuf, veuve`, et seul `Marié(e)` est choisi

### Requirement: Formulaire vide
Sans objet montré, le formulaire SHALL montrer tous ses libellés, titres, séparateurs et textes fixes. Chaque widget SHALL y être comme pour un attribut sans valeur : texte vide, case indéterminée, liste vide, aucun bouton radio choisi.

#### Scenario: Annuaire au lancement
- **WHEN** la présentation est créée pour `examples/annuaire`
- **THEN** le formulaire de la vue `proprietes` a ses libellés (`Nom`, `Prénom`, `Taille (m)`…) et le titre `Identité`, et tous ses widgets sont sans valeur

### Requirement: Objet montré
La présentation SHALL permettre de donner à une vue formulaire l'objet à montrer, ou aucun. Un objet d'une autre classe que celle de la vue ou de ses sous-classes SHALL laisser le formulaire vide. Un objet retiré du document SHALL NOT être montré : le formulaire redevient vide.

#### Scenario: Changement d'objet
- **WHEN** la vue `proprietes` de l'annuaire montre la personne `Dupont`, puis la personne `Martin`
- **THEN** le champ `nom` vaut `Martin`, et le champ `prenom` `Lucas`

#### Scenario: Objet d'une autre classe
- **WHEN** on donne à la vue `proprietes` de l'annuaire l'objet `Annuaire`
- **THEN** le formulaire est vide

#### Scenario: Objet retiré
- **WHEN** la vue `proprietes` de l'annuaire montre la personne `Dupont`, puis la branche `/annuaire/personnes` est remplacée par une autre liste
- **THEN** le formulaire est vide

### Requirement: Valeurs à jour
Les valeurs montrées SHALL être celles de la révision courante du document : une modification de l'objet montré SHALL apparaître à l'image suivante.

#### Scenario: Prénom modifié
- **WHEN** la vue `proprietes` montre la personne `Dupont`, puis une transaction du document change son `prenom` en `Marie-Anne`
- **THEN** à l'image suivante, le champ `prenom` vaut `Marie-Anne`

### Requirement: Affichage du formulaire
L'interface SHALL afficher les libellés, les valeurs, les cases, les listes, les boutons radio, les titres des groupes, les séparateurs et les textes fixes fournis par la présentation. Chaque widget SHALL être relié à son libellé, pour l'accessibilité.

#### Scenario: Formulaire de Dupont affiché
- **WHEN** dans un test sans écran, la vue `proprietes` de l'annuaire montre la personne `Dupont`
- **THEN** le champ de texte de libellé `Nom` vaut `Dupont`, la case de libellé `Contact` est cochée, la liste de libellé `Situation` montre `Marié(e)`, et le titre `Identité` est présent

#### Scenario: Formulaire vide affiché
- **WHEN** dans un test sans écran, l'application de `examples/annuaire` dessine une image
- **THEN** le libellé `Nom` est présent dans la vue de droite, et le champ de texte de libellé `Nom` est vide

### Requirement: Dessin en grille
L'interface SHALL dessiner chaque cellule dans sa colonne et sa ligne : à la position et sur la largeur que donne la grille pour sa colonne, ou pour l'ensemble de ses colonnes, espacements compris. Elle SHALL fournir à la grille la largeur minimale qu'elle mesure pour chaque cellule. Un groupe SHALL être dessiné dans un cadre, avec son titre au-dessus de sa grille.

#### Scenario: Champs alignés
- **WHEN** dans un test sans écran, la vue `proprietes` de l'annuaire montre la personne `Dupont`
- **THEN** les champs de libellés `Enfants` et `Taille (m)` commencent à la même abscisse, et s'étendent jusqu'au bord droit de la vue, à la marge près

#### Scenario: Zone de texte sous son libellé
- **WHEN** dans un test sans écran, la vue `proprietes` de l'annuaire montre la personne `Dupont`
- **THEN** la zone de texte de `notes` est sous le libellé `Notes`, et commence à la même abscisse que lui

#### Scenario: Groupe dans son cadre
- **WHEN** dans un test sans écran, la vue `proprietes` de l'annuaire montre la personne `Dupont`
- **THEN** le titre `Identité` est au-dessus du champ de libellé `Nom`, et le champ de libellé `Enfants`, hors du groupe, est sous le champ de libellé `Né le`

### Requirement: Lecture seule
Les widgets du formulaire SHALL NOT permettre de modifier une valeur : le texte d'un champ peut être sélectionné, mais ni saisi, ni effacé ; les cases, les listes et les boutons radio SHALL être désactivés. L'interface SHALL NOT émettre d'intention depuis le formulaire.

#### Scenario: Saisie dans un champ
- **WHEN** dans un test sans écran, la vue `proprietes` montre la personne `Dupont`, et l'on clique dans le champ de libellé `Nom` puis tape `x`
- **THEN** le champ vaut toujours `Dupont`, et l'interface n'émet aucune intention

#### Scenario: Clic sur une case
- **WHEN** dans un test sans écran, la vue `proprietes` montre la personne `Dupont`, et l'on clique sur la case de libellé `Contact`
- **THEN** la case est toujours cochée, et l'interface n'émet aucune intention

### Requirement: Défilement du formulaire
Quand le formulaire est plus haut que sa vue, l'interface SHALL permettre de le faire défiler verticalement. Quand la vue est plus étroite que la largeur minimale du formulaire, elle SHALL permettre de le faire défiler horizontalement.

#### Scenario: Notes atteintes par défilement
- **WHEN** dans un test sans écran, dans une fenêtre de 640 × 240 points, la vue `proprietes` de l'annuaire montre la personne `Dupont`, et l'on fait défiler le formulaire vers le bas avec la molette
- **THEN** avant le défilement, la zone de texte de `notes` est hors de la vue ; après, elle est dans la vue
