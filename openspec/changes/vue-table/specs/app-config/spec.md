# Spec Delta

## MODIFIED Requirements

### Requirement: Contenu de `app.json`
Le système SHALL lire un fichier `app.json` au format 0.4, qui contient : `formatVersion`, `id` (identifiant de l'application), `name` (son nom), `languages`, `labels` (optionnel), `model` (liste des fichiers de modèle, `files`, et classe de la racine, `root`), `sources`, `presentations` (optionnel) et `window`. Une source a un identifiant (`id`), un format (`json`), un fichier (`file`) et une cible (`target`). Les chemins de fichiers sont relatifs au dossier de `app.json`.

#### Scenario: Application de l'annuaire
- **WHEN** le système lit `examples/annuaire/app.json`
- **THEN** il obtient l'identifiant `annuaire`, les langues `fr` et `en`, le fichier de modèle `annuaire.model.json`, la racine `Racine`, une source `personnes` au format `json`, de fichier `personnes.json` et de cible `/annuaire/personnes`, une présentation de la classe `Personne` et une fenêtre dont le contenu est une vue table de données `/annuaire/personnes`

### Requirement: Version du format
La valeur de `formatVersion` MUST être `0.4`. Une autre version MUST être refusée, avec une erreur qui cite la version lue et la version attendue. Il n'y a pas de migration des versions antérieures.

#### Scenario: Ancienne version
- **WHEN** `app.json` déclare `"formatVersion": "0.3"`
- **THEN** la lecture échoue avec une erreur qui cite `0.3` et `0.4`

## ADDED Requirements

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
Le contenu de la fenêtre SHALL être un objet polymorphe dont `_type_` vaut `View` : une vue, avec un identifiant (`id`), un widget (`widget`, objet polymorphe dont `_type_` vaut `Table`) et un chemin de données (`data`). Un `_type_` inconnu MUST être refusé, avec une erreur qui le cite.

#### Scenario: Vue table
- **WHEN** `window.content` vaut `{ "_type_": "View", "id": "personnes", "widget": { "_type_": "Table" }, "data": "/annuaire/personnes" }`
- **THEN** la lecture réussit, et le contenu est la vue `personnes`, une table de données `/annuaire/personnes`

#### Scenario: Conteneur inconnu
- **WHEN** `window.content` a pour `_type_` `Split`
- **THEN** la lecture échoue avec une erreur qui cite `Split`

### Requirement: Données d'une vue vérifiées contre le modèle
Le chemin `data` d'une vue table SHALL suivre la forme d'une cible de source : `/r1/…/rn`, depuis la classe de la racine. Les références intermédiaires MUST être des contenances simples, et la dernière MUST être une contenance multiple ; la classe affichée est son type. Une erreur cite la vue et le segment.

#### Scenario: Données de l'annuaire
- **WHEN** une vue table a pour données `/annuaire/personnes`
- **THEN** la vérification réussit, et la classe affichée est `Personne`

#### Scenario: Données qui ne sont pas une liste
- **WHEN** une vue table a pour données `/annuaire`
- **THEN** la vérification échoue avec une erreur qui cite la vue et indique que `annuaire` n'est pas une liste

### Requirement: Présentations vérifiées contre le modèle
Chaque présentation SHALL désigner par `class` une classe du modèle, et une classe MUST NOT avoir deux présentations. Sa `table`, optionnelle, a une liste de colonnes (`columns`) et un tri (`sort`, optionnel). Une erreur cite la présentation et la classe.

#### Scenario: Classe inconnue
- **WHEN** une présentation désigne la classe `Persone`
- **THEN** la vérification échoue avec une erreur qui cite `Persone`

#### Scenario: Deux présentations pour une classe
- **WHEN** deux présentations désignent `Personne`
- **THEN** la vérification échoue avec une erreur qui cite `Personne`

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
