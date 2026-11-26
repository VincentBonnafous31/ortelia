# app-context Specification

## Purpose

Ouvrir une application à partir de son `app.json` : compiler son modèle, créer l'arbre de ses objets avec sa racine et le squelette de ses branches, puis alimenter chaque branche par sa source, en gardant pour chaque source son état et le lien avec sa branche.

## Requirements

### Requirement: Ouverture d'une application
L'ouverture d'une application SHALL lire `app.json`, lire et compiler les fichiers du modèle, vérifier `app.json` contre le modèle, puis créer le Document de l'application et l'alimenter par ses sources. Une erreur dans `app.json` ou dans le modèle MUST empêcher l'ouverture, avec une erreur qui cite le fichier en cause.

#### Scenario: Annuaire ouvert
- **WHEN** on ouvre `examples/annuaire/app.json`
- **THEN** l'ouverture réussit, la racine est de classe `Racine`, son annuaire contient les dix personnes de `personnes.json` dans l'ordre du fichier, avec leurs valeurs, et la source `personnes` est chargée sans diagnostic

#### Scenario: Fichier de modèle introuvable
- **WHEN** `app.json` déclare un fichier de modèle qui n'existe pas
- **THEN** l'ouverture échoue avec une erreur qui cite ce fichier

#### Scenario: Modèle qui ne compile pas
- **WHEN** un fichier de modèle déclare un attribut de type inconnu
- **THEN** l'ouverture échoue avec une erreur qui cite le fichier de modèle, la classe et l'attribut

### Requirement: Racine et squelette créés à l'ouverture
Avant toute lecture de données, l'ouverture SHALL créer la racine, de la classe de `model.root`, puis, pour chaque cible, les objets intermédiaires de son chemin, chacun du type déclaré de sa référence. Cette création forme la première transaction du Document.

#### Scenario: Squelette de l'annuaire
- **WHEN** on ouvre une application dont l'unique source a pour cible `/annuaire/personnes`
- **THEN** la première transaction du journal crée un `Annuaire` dans la référence `annuaire` de la racine, avant l'intégration des personnes

#### Scenario: Objet intermédiaire commun à deux cibles
- **WHEN** deux sources ont pour cibles `/a/b` et `/a/c`, où `a` est une référence simple
- **THEN** l'ouverture ne crée qu'un objet dans `a`, et les deux sources alimentent ses références `b` et `c`

### Requirement: Alimentation des branches par les sources
L'ouverture SHALL lire chaque source dans l'ordre de `app.json`, puis intégrer ce qu'elle a lu dans sa branche, en remplaçant son contenu, en une transaction par source. Le fichier d'une source contient un objet JSON quand sa branche est une référence simple ou la racine, et un tableau JSON quand c'est une liste.

#### Scenario: Deux sources dans deux branches
- **WHEN** une application du réseau déclare une source pour `/stations` et une autre pour `/zones`
- **THEN** après l'ouverture, la racine contient les stations du premier fichier et les zones du second, et le journal contient, après la création du squelette, une transaction par source, dans l'ordre de `app.json`

#### Scenario: Source à la racine
- **WHEN** une application a une seule source, de cible `/`, dont le fichier contient l'objet racine
- **THEN** après l'ouverture, la racine du Document est l'objet lu dans ce fichier

### Requirement: Une source en échec n'empêche pas l'ouverture
Quand le fichier d'une source est introuvable, illisible, ou que ses données sont refusées, l'ouverture SHALL réussir quand même. La branche de cette source reste telle que le squelette l'a créée, les autres sources sont lues normalement, et l'état de la source donne son erreur.

#### Scenario: Fichier de données absent
- **WHEN** on ouvre l'annuaire alors que `personnes.json` n'existe pas
- **THEN** l'ouverture réussit, l'annuaire existe et ne contient aucune personne, et l'état de la source `personnes` est en échec, avec une erreur qui cite le fichier

#### Scenario: Données refusées dans une source sur deux
- **WHEN** une application du réseau a une source pour `/stations` dont le fichier contient une clé inconnue, et une source correcte pour `/zones`
- **THEN** l'ouverture réussit, la branche `stations` est vide, la branche `zones` contient les zones du second fichier, et l'état de la source des stations donne l'erreur, avec son chemin

### Requirement: État de chaque source
Après l'ouverture, le système SHALL donner, pour chaque source : son identifiant, son fichier, la branche qu'elle alimente, et son état. Une source chargée donne ses diagnostics, avec leur nombre total, et la révision du Document à laquelle elle a été intégrée. Une source en échec donne ses erreurs, avec leur nombre total.

#### Scenario: Personne sans nom
- **WHEN** on ouvre l'annuaire avec un `personnes.json` dont la troisième personne n'a pas de `nom`, attribut obligatoire
- **THEN** la source `personnes` est chargée avec un diagnostic qui cite `nom` et le chemin `/annuaire/personnes/2`, et l'annuaire contient toutes les personnes du fichier

#### Scenario: Lien entre source et branche
- **WHEN** on ouvre l'annuaire
- **THEN** l'état de la source `personnes` donne pour branche la référence `personnes` de l'annuaire du Document, et pour révision d'intégration celle de la transaction qui a intégré les personnes

### Requirement: Valeurs des données absentes du journal
La journalisation de l'ouverture SHALL donner l'application, les fichiers lus, le nombre d'objets intégrés par source, et le nombre d'erreurs et de diagnostics. Elle MUST NOT contenir aucune valeur lue dans un fichier de données, même quand cette valeur est fautive.

#### Scenario: Valeur fautive non journalisée
- **WHEN** on ouvre une application dont une source contient la valeur `"haute"` pour un attribut `Int`
- **THEN** le journal signale l'échec de la source avec le chemin de la donnée, et ne contient nulle part le texte `haute`

### Requirement: Avertissements de la configuration
L'ouverture SHALL rapporter, avec l'application ouverte, les avertissements de la vérification de `app.json` contre le modèle, comme les clés de libellés qui ne désignent rien. Ces avertissements MUST NOT empêcher l'ouverture.

#### Scenario: Ouverture avec avertissement
- **WHEN** on ouvre une application dont un libellé a la clé `Personne.nmo`
- **THEN** l'ouverture réussit, les sources sont chargées, et l'application ouverte porte un avertissement qui cite `Personne.nmo`

#### Scenario: Annuaire sans avertissement de configuration
- **WHEN** on ouvre `examples/annuaire/app.json`
- **THEN** l'application ouverte ne porte aucun avertissement de configuration
