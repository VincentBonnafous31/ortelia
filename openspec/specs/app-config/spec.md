# app-config Specification

## Purpose

Lire la définition d'une application, `app.json`, et la vérifier contre son modèle avant toute lecture de données : fichiers du modèle, classe de la racine, et sources de données avec la branche que chacune alimente, avec des erreurs claires quand le fichier est fautif.

## Requirements

### Requirement: Contenu de `app.json`
Le système SHALL lire un fichier `app.json` au format 0.6, qui contient : `formatVersion`, `id` (identifiant de l'application), `name` (son nom), `languages`, `labels` (optionnel), `model` (liste des fichiers de modèle, `files`, et classe de la racine, `root`), `sources`, `presentations` (optionnel) et `window`. Une source a un identifiant (`id`), un format (`json`), un fichier (`file`) et une cible (`target`). Les chemins de fichiers sont relatifs au dossier de `app.json`.

#### Scenario: Application de l'annuaire
- **WHEN** le système lit `examples/annuaire/app.json`
- **THEN** il obtient :
  - l'identifiant `annuaire`, les langues `fr` et `en` ;
  - le fichier de modèle `annuaire.model.json` et la racine `Racine` ;
  - une source `personnes` au format `json`, de fichier `personnes.json` et de cible `/annuaire/personnes` ;
  - une présentation de la classe `Personne`, avec une table et un formulaire ;
  - une fenêtre dont le contenu est le split horizontal `principal`, de poids `[65, 35]`, qui contient une vue table de données `/annuaire/personnes`, puis la vue formulaire `proprietes` de la classe `Personne`.

### Requirement: Fichier `app.json` mal formé refusé
Un `app.json` qui n'est pas du JSON valide, qui contient une clé inconnue, auquel il manque une clé obligatoire, ou dont une valeur n'a pas le type attendu MUST être refusé, avec une erreur qui cite la clé quand il y en a une, la ligne et la colonne. Un format de source autre que `json` MUST être refusé.

#### Scenario: Clé inconnue
- **WHEN** `app.json` contient la clé `sourcse` au lieu de `sources`
- **THEN** la lecture échoue avec une erreur qui cite `sourcse`, la ligne et la colonne

#### Scenario: Clé obligatoire absente
- **WHEN** une source de `app.json` n'a pas de clé `target`
- **THEN** la lecture échoue avec une erreur qui cite `target`

#### Scenario: Format de source inconnu
- **WHEN** une source déclare le format `xml`
- **THEN** la lecture échoue avec une erreur qui cite `xml`

### Requirement: Version du format
La valeur de `formatVersion` MUST être `0.6`. Une autre version MUST être refusée, avec une erreur qui cite la version lue et la version attendue. Il n'y a pas de migration des versions antérieures.

#### Scenario: Ancienne version
- **WHEN** `app.json` déclare `"formatVersion": "0.5"`
- **THEN** la lecture échoue avec une erreur qui cite `0.5` et `0.6`

### Requirement: Clé `$schema` acceptée
La clé `$schema` SHALL être acceptée à la racine de `app.json`, avec un texte pour valeur, et ignorée par le système : elle ne sert qu'à l'éditeur de texte.

#### Scenario: Fichier avec `$schema`
- **WHEN** `app.json` commence par `"$schema": "../../schemas/app.schema.json"`
- **THEN** la lecture réussit et donne la même application que sans cette clé

### Requirement: Racine vérifiée contre le modèle
La classe désignée par `model.root` MUST exister dans le modèle et MUST être concrète. Sinon, la vérification échoue avec une erreur qui cite la classe.

#### Scenario: Racine introuvable
- **WHEN** `model.root` vaut `Racinne`, absente du modèle
- **THEN** la vérification échoue avec une erreur qui cite `Racinne`

#### Scenario: Racine abstraite
- **WHEN** `model.root` désigne une classe abstraite
- **THEN** la vérification échoue avec une erreur qui cite cette classe

### Requirement: Cible d'une source vérifiée contre le modèle
Une cible SHALL être `/`, la racine elle-même, ou un chemin `/r1/r2/…/rn` de noms de références de contenance. Depuis la classe de la racine, chaque nom MUST être une référence de contenance de la classe atteinte. Les références intermédiaires MUST être simples, de type concret ; la dernière peut être simple ou multiple. Une erreur cite la source, le segment et la classe.

#### Scenario: Cible de l'annuaire
- **WHEN** la cible `/annuaire/personnes` est vérifiée contre le modèle de l'annuaire
- **THEN** la vérification réussit : la branche est la référence multiple `personnes` d'un `Annuaire`, qui est lui-même dans la référence simple `annuaire` de la racine

#### Scenario: Segment inconnu
- **WHEN** une source a pour cible `/annuaire/persones`
- **THEN** la vérification échoue avec une erreur qui cite la source, `persones` et la classe `Annuaire`

#### Scenario: Segment intermédiaire multiple
- **WHEN** une source a pour cible `/stations/equipements` dans le modèle du réseau, où `stations` est une liste
- **THEN** la vérification échoue avec une erreur qui indique que `stations` est une liste et ne peut pas être traversée

#### Scenario: Segment intermédiaire de type abstrait
- **WHEN** une source a pour cible un chemin qui traverse une référence simple dont le type est une classe abstraite
- **THEN** la vérification échoue avec une erreur qui cite la référence et sa classe abstraite

#### Scenario: Cible à la racine
- **WHEN** une source a pour cible `/`
- **THEN** la vérification réussit : la source alimente toute l'application, à partir de la racine

### Requirement: Un fichier, une branche
Chaque source SHALL alimenter sa propre branche. Deux sources MUST NOT avoir le même identifiant, MUST NOT viser la même branche, et la cible de l'une MUST NOT être à l'intérieur de celle d'une autre ; la cible `/` exclut donc toute autre source. Une erreur cite les deux sources.

#### Scenario: Même branche
- **WHEN** deux sources ont pour cible `/stations`
- **THEN** la vérification échoue avec une erreur qui cite les deux sources

#### Scenario: Branches imbriquées
- **WHEN** une source a pour cible `/annuaire` et une autre `/annuaire/personnes`
- **THEN** la vérification échoue avec une erreur qui cite les deux sources

#### Scenario: Deux branches distinctes
- **WHEN** une source a pour cible `/stations` et une autre `/zones`
- **THEN** la vérification réussit

#### Scenario: Identifiant en double
- **WHEN** deux sources ont l'identifiant `personnes`
- **THEN** la vérification échoue avec une erreur qui cite `personnes`

### Requirement: Erreurs de vérification rassemblées
La vérification d'`app.json` contre le modèle SHALL signaler toutes les erreurs qu'elle trouve, et non seulement la première.

#### Scenario: Deux cibles fautives
- **WHEN** deux sources ont chacune une cible dont un segment est inconnu
- **THEN** la vérification échoue avec deux erreurs, une par source

### Requirement: Langues déclarées
`languages` MUST être une liste non vide, sans doublon. Sinon, la lecture échoue avec une erreur qui cite `languages`.

#### Scenario: Liste vide
- **WHEN** `app.json` déclare `"languages": []`
- **THEN** la lecture échoue avec une erreur qui cite `languages`

#### Scenario: Langue en double
- **WHEN** `app.json` déclare `"languages": ["fr", "fr"]`
- **THEN** la lecture échoue avec une erreur qui cite `fr`

### Requirement: Fenêtre
`window` SHALL donner le titre de la fenêtre (`title`, optionnel : `name` à défaut), sa largeur et sa hauteur initiales en points (`width`, `height`, optionnelles), et son contenu (`content`). Une largeur ou une hauteur MUST être comprise entre 200 et 10 000 points.

#### Scenario: Fenêtre de l'annuaire
- **WHEN** `window` déclare le titre `Annuaire`, la largeur 1000 et la hauteur 700
- **THEN** l'application a ces titre et taille initiale

#### Scenario: Titre par défaut
- **WHEN** `window` ne déclare pas de titre
- **THEN** le titre de la fenêtre est le `name` de l'application

#### Scenario: Taille hors limites
- **WHEN** `window` déclare la largeur 50
- **THEN** la lecture échoue avec une erreur qui cite `width` et 50

### Requirement: Contenu de la fenêtre
Le contenu de la fenêtre SHALL être un objet polymorphe dont `_type_` vaut `View` ou `Split`. Une vue a un identifiant (`id`), un widget (`widget`, objet polymorphe dont `_type_` vaut `Table`, `Empty` ou `Form`) et, selon son widget, un chemin de données (`data`). Un split a un identifiant (`id`), une direction (`direction`), des poids optionnels (`weights`) et des enfants (`children`), chacun une vue ou un split. Un `_type_` inconnu MUST être refusé, avec une erreur qui le cite.

#### Scenario: Vue table
- **WHEN** `window.content` vaut `{ "_type_": "View", "id": "personnes", "widget": { "_type_": "Table" }, "data": "/annuaire/personnes" }`
- **THEN** la lecture réussit, et le contenu est la vue `personnes`, une table de données `/annuaire/personnes`

#### Scenario: Split de deux vues
- **WHEN** `window.content` est un split `principal` de direction `horizontal`, de poids `[65, 35]`, dont les enfants sont une vue table `personnes` et une vue `proprietes` de widget `{ "_type_": "Empty" }`
- **THEN** la lecture réussit, et le contenu est le split `principal`, avec ses deux vues dans cet ordre

#### Scenario: Vue formulaire
- **WHEN** une vue `proprietes` a pour widget `{ "_type_": "Form", "class": "Personne" }`
- **THEN** la lecture réussit, et la vue `proprietes` est une vue formulaire de la classe `Personne`

#### Scenario: Conteneur inconnu
- **WHEN** `window.content` a pour `_type_` `Tabs`
- **THEN** la lecture échoue avec une erreur qui cite `Tabs`

#### Scenario: Widget inconnu
- **WHEN** le widget d'une vue a pour `_type_` `Chart`
- **THEN** la lecture échoue avec une erreur qui cite `Chart`

### Requirement: Données d'une vue vérifiées contre le modèle
Le chemin `data` d'une vue table SHALL suivre la forme d'une cible de source : `/r1/…/rn`, depuis la classe de la racine. Les références intermédiaires MUST être des contenances simples, et la dernière MUST être une contenance multiple ; la classe affichée est son type. Une erreur cite la vue et le segment. Chaque vue table de la fenêtre, à toute profondeur dans les splits, SHALL être vérifiée.

#### Scenario: Données de l'annuaire
- **WHEN** une vue table a pour données `/annuaire/personnes`
- **THEN** la vérification réussit, et la classe affichée est `Personne`

#### Scenario: Données qui ne sont pas une liste
- **WHEN** une vue table a pour données `/annuaire`
- **THEN** la vérification échoue avec une erreur qui cite la vue et indique que `annuaire` n'est pas une liste

#### Scenario: Vue table dans un split imbriqué
- **WHEN** une vue table `b`, enfant d'un split lui-même enfant du split de la fenêtre, a pour données `/annuaire/persones`
- **THEN** la vérification échoue avec une erreur qui cite la vue `b` et `persones`

### Requirement: Présentations vérifiées contre le modèle
Chaque présentation SHALL désigner par `class` une classe du modèle, et une classe MUST NOT avoir deux présentations. Sa `table`, optionnelle, a une liste de colonnes (`columns`) et un tri (`sort`, optionnel). Son formulaire (`form`), optionnel, a une grille (`columns`, optionnelle) et des éléments (`elements`). Une erreur cite la présentation et la classe.

#### Scenario: Classe inconnue
- **WHEN** une présentation désigne la classe `Persone`
- **THEN** la vérification échoue avec une erreur qui cite `Persone`

#### Scenario: Deux présentations pour une classe
- **WHEN** deux présentations désignent `Personne`
- **THEN** la vérification échoue avec une erreur qui cite `Personne`

#### Scenario: Présentation avec formulaire seul
- **WHEN** la présentation de `Personne` déclare un formulaire, sans table
- **THEN** la vérification réussit

### Requirement: Colonnes vérifiées contre le modèle
Une colonne SHALL désigner par `path` un attribut monovalué de la classe de la présentation, hérité compris, et peut donner une largeur en points (`width`, entre 20 et 2 000), un alignement (`align` : `left`, `center` ou `right`) et un format (`format`). Une liste de colonnes MUST NOT être vide, ni désigner deux fois le même attribut. Une erreur cite la classe et la colonne.

#### Scenario: Attribut inconnu
- **WHEN** une colonne de la présentation de `Personne` a pour chemin `prenon`
- **THEN** la vérification échoue avec une erreur qui cite `Personne` et `prenon`

#### Scenario: Colonne en double
- **WHEN** la présentation de `Personne` a deux colonnes de chemin `nom`
- **THEN** la vérification échoue avec une erreur qui cite `nom`

#### Scenario: Alignement inconnu
- **WHEN** une colonne déclare `"align": "middle"`
- **THEN** la lecture échoue avec une erreur qui cite `middle`

### Requirement: Tri initial vérifié
Chaque critère du `sort` d'une table SHALL désigner par `path` une colonne de cette table, avec un sens (`order` : `asc`, par défaut, ou `desc`). Une colonne MUST NOT figurer deux fois dans le tri. Une erreur cite la classe et le chemin.

#### Scenario: Tri sur une colonne absente
- **WHEN** la table de `Personne` a les colonnes `nom` et `prenom`, et trie sur `date_naissance`
- **THEN** la vérification échoue avec une erreur qui cite `Personne` et `date_naissance`

#### Scenario: Sens par défaut
- **WHEN** un critère de tri ne déclare pas de sens
- **THEN** il trie par ordre croissant

### Requirement: Données selon le widget
Une vue table MUST avoir un chemin de données (`data`), et une vue vide ou une vue formulaire MUST NOT en avoir. Sinon, la lecture échoue avec une erreur qui cite la vue et `data`.

#### Scenario: Table sans données
- **WHEN** une vue table `personnes` n'a pas de clé `data`
- **THEN** la lecture échoue avec une erreur qui cite `personnes` et `data`

#### Scenario: Vue vide avec des données
- **WHEN** une vue vide `proprietes` a pour données `/annuaire/personnes`
- **THEN** la lecture échoue avec une erreur qui cite `proprietes` et `data`

#### Scenario: Vue formulaire avec des données
- **WHEN** une vue formulaire `proprietes` a pour données `/annuaire/personnes`
- **THEN** la lecture échoue avec une erreur qui cite `proprietes` et `data`

### Requirement: Split vérifié
Un split MUST avoir au moins deux enfants, et sa direction MUST être `horizontal` (enfants côte à côte, de gauche à droite) ou `vertical` (enfants l'un au-dessus de l'autre, de haut en bas). S'il déclare des poids, il MUST en déclarer autant qu'il a d'enfants, et chacun MUST être un nombre fini strictement positif. Sinon, la lecture échoue avec une erreur qui cite le split.

#### Scenario: Split d'un seul enfant
- **WHEN** le split `principal` n'a qu'un enfant
- **THEN** la lecture échoue avec une erreur qui cite `principal` et `children`

#### Scenario: Direction inconnue
- **WHEN** un split déclare la direction `diagonal`
- **THEN** la lecture échoue avec une erreur qui cite `diagonal`

#### Scenario: Nombre de poids faux
- **WHEN** le split `principal`, de deux enfants, déclare les poids `[50, 30, 20]`
- **THEN** la lecture échoue avec une erreur qui cite `principal` et `weights`

#### Scenario: Poids nul
- **WHEN** le split `principal` déclare les poids `[100, 0]`
- **THEN** la lecture échoue avec une erreur qui cite `principal` et `weights`

#### Scenario: Poids absents
- **WHEN** un split de deux enfants ne déclare pas de poids
- **THEN** la lecture réussit

### Requirement: Identifiants uniques dans la fenêtre
Les identifiants des vues et des splits de la fenêtre, à toute profondeur, SHALL former un seul espace de noms : deux d'entre eux MUST NOT être égaux, même si l'un est une vue et l'autre un split. Sinon, la lecture échoue avec une erreur qui cite l'identifiant.

#### Scenario: Deux vues de même identifiant
- **WHEN** deux vues de la fenêtre ont l'identifiant `personnes`
- **THEN** la lecture échoue avec une erreur qui cite `personnes`

#### Scenario: Une vue et un split de même identifiant
- **WHEN** le split de la fenêtre et l'une de ses vues ont l'identifiant `principal`
- **THEN** la lecture échoue avec une erreur qui cite `principal`

### Requirement: Classe d'une vue formulaire vérifiée
Un widget `Form` SHALL déclarer une classe (`class`), qui MUST être une classe du modèle, abstraite ou non. Une classe absente fait échouer la lecture, avec une erreur qui cite `class` ; une classe inconnue fait échouer la vérification, avec une erreur qui cite la vue et la classe.

#### Scenario: Formulaire sans classe
- **WHEN** le widget d'une vue est `{ "_type_": "Form" }`
- **THEN** la lecture échoue avec une erreur qui cite `class`

#### Scenario: Classe inconnue
- **WHEN** la vue formulaire `proprietes` déclare la classe `Persone`
- **THEN** la vérification échoue avec une erreur qui cite `proprietes` et `Persone`

### Requirement: Éléments d'un formulaire
Les éléments d'un formulaire ou d'un groupe (`elements`) SHALL être des objets polymorphes dont `_type_` vaut `Field` (un champ), `Group` (un groupe), `Separator` (un séparateur) ou `Text` (un texte fixe). Un `_type_` inconnu MUST être refusé, avec une erreur qui le cite.

#### Scenario: Formulaire de l'annuaire
- **WHEN** le système lit la présentation de `Personne` de `examples/annuaire/app.json`
- **THEN** son formulaire a pour éléments un groupe de clé `Personne.identite` avec trois champs, cinq champs, un séparateur et un champ `notes`

#### Scenario: Élément inconnu
- **WHEN** un élément de formulaire a pour `_type_` `Button`
- **THEN** la lecture échoue avec une erreur qui cite `Button`

### Requirement: Clés d'un champ
Un champ SHALL avoir un attribut (`path`) et, optionnels, un widget (`widget`), une clé de libellé (`label`), la cellule de son libellé (`labelCell`) et celle de son widget (`cell`).

#### Scenario: Champ sans attribut
- **WHEN** un élément `Field` n'a pas de clé `path`
- **THEN** la lecture échoue avec une erreur qui cite `path`

#### Scenario: Champ minimal
- **WHEN** un élément vaut `{ "_type_": "Field", "path": "nom" }`
- **THEN** la lecture réussit, et le champ n'a ni widget, ni clé de libellé, ni cellule déclarés

### Requirement: Clés d'un groupe, d'un séparateur et d'un texte
Un groupe SHALL avoir, optionnels, une clé de titre (`label`), une grille (`columns`), des éléments (`elements`, aucun par défaut) et une cellule (`cell`). Un séparateur SHALL avoir une cellule optionnelle (`cell`). Un texte fixe SHALL avoir une clé de libellé (`label`) et une cellule optionnelle (`cell`).

#### Scenario: Groupe imbriqué
- **WHEN** un groupe a pour éléments un champ et un autre groupe, qui contient lui-même un champ
- **THEN** la lecture réussit, avec les deux groupes et leurs champs

#### Scenario: Texte sans clé
- **WHEN** un élément `Text` n'a pas de clé `label`
- **THEN** la lecture échoue avec une erreur qui cite `label`

### Requirement: Grille d'un formulaire
La grille d'un formulaire ou d'un groupe (`columns`) SHALL être une liste non vide de colonnes, chacune avec un poids optionnel (`grow`), un nombre fini positif ou nul. Sans `columns`, la grille SHALL avoir deux colonnes : la première sans poids, la seconde de poids 1. Une cellule (`labelCell`, `cell`) a une étendue optionnelle (`span`) et une demande de nouvelle ligne optionnelle (`newRow`, faux par défaut).

#### Scenario: Grille par défaut
- **WHEN** un formulaire ne déclare pas de `columns`
- **THEN** sa grille a deux colonnes, de poids 0 et 1

#### Scenario: Grille vide
- **WHEN** un formulaire déclare `"columns": []`
- **THEN** la vérification échoue avec une erreur qui cite `columns`

#### Scenario: Poids négatif
- **WHEN** une colonne de la grille d'un groupe déclare `"grow": -1`
- **THEN** la vérification échoue avec une erreur qui cite `grow`

### Requirement: Étendue d'une cellule vérifiée
L'étendue d'une cellule MUST être au moins 1 et au plus le nombre de colonnes de la grille qui la contient. Sinon, la vérification échoue avec une erreur qui cite l'étendue et le nombre de colonnes.

#### Scenario: Étendue trop grande
- **WHEN** dans un formulaire de deux colonnes, la cellule du widget d'un champ déclare `"span": 3`
- **THEN** la vérification échoue avec une erreur qui cite 3 et 2

#### Scenario: Étendue nulle
- **WHEN** la cellule d'un séparateur déclare `"span": 0`
- **THEN** la vérification échoue avec une erreur qui cite `span`

### Requirement: Champs vérifiés contre le modèle
Le `path` d'un champ SHALL désigner un attribut de la classe de la présentation, hérité compris. Un même attribut MAY figurer dans plusieurs champs d'un formulaire. Sinon, la vérification échoue avec une erreur qui cite la classe et le chemin.

#### Scenario: Attribut inconnu
- **WHEN** un champ du formulaire de `Personne` a pour chemin `prenon`
- **THEN** la vérification échoue avec une erreur qui cite `Personne` et `prenon`

#### Scenario: Attribut montré deux fois
- **WHEN** le formulaire de `Personne` a deux champs de chemin `nom`, l'un hors de tout groupe, l'autre dans un groupe
- **THEN** la vérification réussit

### Requirement: Widgets d'un champ
Le widget d'un champ SHALL être un objet polymorphe dont `_type_` vaut `TextField`, `TextArea`, `NumberField`, `CheckBox`, `ComboBox`, `RadioGroup`, `DateField` ou `DateTimeField`. `TextArea` accepte un nombre de lignes (`rows`, de 1 à 50, 4 par défaut), `DateField` et `DateTimeField` un motif de date (`format`), vérifié comme celui d'une colonne de même type (voir `value-formats`). Un `_type_` inconnu MUST être refusé.

#### Scenario: Zone de texte
- **WHEN** le champ `notes` a pour widget `{ "_type_": "TextArea", "rows": 6 }`
- **THEN** la lecture réussit, et la zone de texte a six lignes

#### Scenario: Nombre de lignes hors limites
- **WHEN** un widget `TextArea` déclare `"rows": 0`
- **THEN** la vérification échoue avec une erreur qui cite `rows` et 0

#### Scenario: Motif de date invalide
- **WHEN** le widget `DateField` du champ `date_naissance` déclare le format `dd/QQ/yyyy`
- **THEN** la vérification échoue avec une erreur qui cite `date_naissance` et `dd/QQ/yyyy`

#### Scenario: Heure dans un champ de date
- **WHEN** le widget `DateField` du champ `date_naissance` déclare le format `dd/MM/yyyy HH:mm`
- **THEN** la vérification échoue avec une erreur qui cite `date_naissance`

### Requirement: Widget compatible avec son attribut
Le widget d'un champ MUST convenir au type de son attribut : `TextField` et `TextArea` pour un texte, `NumberField` pour un entier ou un décimal, `CheckBox` pour un booléen, `ComboBox` et `RadioGroup` pour un énuméré, `DateField` pour une date, `DateTimeField` pour une date-heure. Sinon, la vérification échoue avec une erreur qui cite le chemin, le widget et le type.

#### Scenario: Case pour un texte
- **WHEN** le champ `nom`, de type `String`, a pour widget `CheckBox`
- **THEN** la vérification échoue avec une erreur qui cite `nom`, `CheckBox` et `String`

#### Scenario: Boutons radio pour un énuméré
- **WHEN** le champ `situation`, de type `SituationFamiliale`, a pour widget `RadioGroup`
- **THEN** la vérification réussit

### Requirement: Erreurs d'un formulaire situées
Une erreur d'un formulaire SHALL citer la classe de la présentation et la position de l'élément fautif dans le formulaire, par les indices de ses éléments à partir de 0 (`form.elements[0].elements[2]`). Toutes les erreurs d'un formulaire SHALL être signalées, et non seulement la première.

#### Scenario: Erreur dans un groupe
- **WHEN** le troisième champ du premier élément du formulaire de `Personne`, un groupe, a pour chemin `prenon`
- **THEN** la vérification échoue avec une erreur qui cite `Personne`, `form.elements[0].elements[2]` et `prenon`

#### Scenario: Deux erreurs
- **WHEN** le formulaire de `Personne` a un champ de chemin inconnu et un séparateur d'étendue 5 dans une grille de deux colonnes
- **THEN** la vérification échoue avec deux erreurs
