# Spec Delta

## MODIFIED Requirements

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

### Requirement: Version du format
La valeur de `formatVersion` MUST être `0.6`. Une autre version MUST être refusée, avec une erreur qui cite la version lue et la version attendue. Il n'y a pas de migration des versions antérieures.

#### Scenario: Ancienne version
- **WHEN** `app.json` déclare `"formatVersion": "0.5"`
- **THEN** la lecture échoue avec une erreur qui cite `0.5` et `0.6`

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

## ADDED Requirements

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
