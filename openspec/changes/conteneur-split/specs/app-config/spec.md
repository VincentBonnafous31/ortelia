# Spec Delta

## MODIFIED Requirements

### Requirement: Contenu de `app.json`
Le système SHALL lire un fichier `app.json` au format 0.5, qui contient : `formatVersion`, `id` (identifiant de l'application), `name` (son nom), `languages`, `labels` (optionnel), `model` (liste des fichiers de modèle, `files`, et classe de la racine, `root`), `sources`, `presentations` (optionnel) et `window`. Une source a un identifiant (`id`), un format (`json`), un fichier (`file`) et une cible (`target`). Les chemins de fichiers sont relatifs au dossier de `app.json`.

#### Scenario: Application de l'annuaire
- **WHEN** le système lit `examples/annuaire/app.json`
- **THEN** il obtient l'identifiant `annuaire`, les langues `fr` et `en`, le fichier de modèle `annuaire.model.json`, la racine `Racine`, une source `personnes` au format `json`, de fichier `personnes.json` et de cible `/annuaire/personnes`, une présentation de la classe `Personne` et une fenêtre dont le contenu est le split horizontal `principal`, de poids `[65, 35]`, qui contient une vue table de données `/annuaire/personnes` puis la vue vide `proprietes`

### Requirement: Version du format
La valeur de `formatVersion` MUST être `0.5`. Une autre version MUST être refusée, avec une erreur qui cite la version lue et la version attendue. Il n'y a pas de migration des versions antérieures.

#### Scenario: Ancienne version
- **WHEN** `app.json` déclare `"formatVersion": "0.4"`
- **THEN** la lecture échoue avec une erreur qui cite `0.4` et `0.5`

### Requirement: Contenu de la fenêtre
Le contenu de la fenêtre SHALL être un objet polymorphe dont `_type_` vaut `View` ou `Split`. Une vue a un identifiant (`id`), un widget (`widget`, objet polymorphe dont `_type_` vaut `Table` ou `Empty`) et, selon son widget, un chemin de données (`data`). Un split a un identifiant (`id`), une direction (`direction`), des poids optionnels (`weights`) et des enfants (`children`), chacun une vue ou un split. Un `_type_` inconnu MUST être refusé, avec une erreur qui le cite.

#### Scenario: Vue table
- **WHEN** `window.content` vaut `{ "_type_": "View", "id": "personnes", "widget": { "_type_": "Table" }, "data": "/annuaire/personnes" }`
- **THEN** la lecture réussit, et le contenu est la vue `personnes`, une table de données `/annuaire/personnes`

#### Scenario: Split de deux vues
- **WHEN** `window.content` est un split `principal` de direction `horizontal`, de poids `[65, 35]`, dont les enfants sont une vue table `personnes` et une vue `proprietes` de widget `{ "_type_": "Empty" }`
- **THEN** la lecture réussit, et le contenu est le split `principal`, avec ses deux vues dans cet ordre

#### Scenario: Conteneur inconnu
- **WHEN** `window.content` a pour `_type_` `Tabs`
- **THEN** la lecture échoue avec une erreur qui cite `Tabs`

#### Scenario: Widget inconnu
- **WHEN** le widget d'une vue a pour `_type_` `Form`
- **THEN** la lecture échoue avec une erreur qui cite `Form`

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

## ADDED Requirements

### Requirement: Données selon le widget
Une vue table MUST avoir un chemin de données (`data`), et une vue vide MUST NOT en avoir. Sinon, la lecture échoue avec une erreur qui cite la vue et `data`.

#### Scenario: Table sans données
- **WHEN** une vue table `personnes` n'a pas de clé `data`
- **THEN** la lecture échoue avec une erreur qui cite `personnes` et `data`

#### Scenario: Vue vide avec des données
- **WHEN** une vue vide `proprietes` a pour données `/annuaire/personnes`
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
