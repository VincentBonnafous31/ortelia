## Purpose

Lire la définition d'une application, `app.json`, et la vérifier contre son modèle avant toute lecture de données : fichiers du modèle, classe de la racine, et sources de données avec la branche que chacune alimente, avec des erreurs claires quand le fichier est fautif.

## ADDED Requirements

### Requirement: Contenu de `app.json`
Le système SHALL lire un fichier `app.json` au format 0.3, qui contient : `formatVersion`, `id` (identifiant de l'application), `name` (son nom), `model` (liste des fichiers de modèle, `files`, et classe de la racine, `root`) et `sources`. Une source a un identifiant (`id`), un format (`json`), un fichier (`file`) et une cible (`target`). Les chemins de fichiers sont relatifs au dossier de `app.json`.

#### Scenario: Application de l'annuaire
- **WHEN** le système lit `examples/annuaire/app.json`
- **THEN** il obtient l'identifiant `annuaire`, le fichier de modèle `annuaire.model.json`, la racine `Racine`, et une source `personnes` au format `json`, de fichier `personnes.json` et de cible `/annuaire/personnes`

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
La valeur de `formatVersion` MUST être `0.3`. Une autre version MUST être refusée, avec une erreur qui cite la version lue et la version attendue. Il n'y a pas de migration des versions antérieures.

#### Scenario: Ancienne version
- **WHEN** `app.json` déclare `"formatVersion": "0.2"`
- **THEN** la lecture échoue avec une erreur qui cite `0.2` et `0.3`

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
