# Spec Delta

## Purpose

Remplir un Store à partir d'un fichier de données : une seule API de construction pour tous les lecteurs, présents et futurs, qui convertit les valeurs selon le modèle, et un lecteur JSON en flux qui applique la correspondance identité entre les clés du fichier et le modèle.

## ADDED Requirements

### Requirement: Construction des objets dans l'ordre du fichier
Le système SHALL fournir une API de construction, la seule voie par laquelle un lecteur crée des données. Le lecteur l'appelle dans l'ordre du fichier : début de l'objet racine ou d'un objet enfant dans une référence de contenance, valeurs de ses attributs, fin de l'objet. Le Store construit est neuf : il ne contient que ce que le lecteur a créé.

#### Scenario: Arbre construit dans l'ordre des appels
- **WHEN** un lecteur crée la racine, une station dans `stations`, puis un thermomètre dans `equipements` de cette station
- **THEN** le Store obtenu contient la racine, sa station et le thermomètre de la station, à leur place dans l'arbre de contenance

### Requirement: Conversion des valeurs texte
Une valeur fournie sous forme de texte SHALL être convertie selon le type de l'attribut : nombre pour `Int` et `Float`, `true` ou `false` pour `Boolean`, date ISO 8601 pour `Date`, instant RFC 3339 avec décalage pour `DateTime`, nom d'un littéral pour un énuméré. Un texte non convertible MUST être refusé, comme une date-heure plus précise que la microseconde.

#### Scenario: Conversions réussies
- **WHEN** l'API de construction reçoit les textes `42` pour un `Int`, `12.5` pour un `Float`, `true` pour un `Boolean`, `1985-03-12` pour une `Date`, `2026-03-12T10:15:00.123456Z` pour une `DateTime` et `ORANGE` pour l'énuméré `NiveauAlerte`
- **THEN** les attributs valent respectivement 42, 12,5, vrai, le 12 mars 1985, l'instant 10:15:00,123456 UTC du 12 mars 2026, et le littéral `ORANGE`

#### Scenario: Texte non convertible
- **WHEN** l'API de construction reçoit le texte `1985-13-45` pour un attribut `Date`
- **THEN** la construction échoue avec une erreur de type de valeur

#### Scenario: Date-heure trop précise
- **WHEN** l'API de construction reçoit `2026-03-12T10:15:00.123456789Z` pour un attribut `DateTime`
- **THEN** la construction échoue avec une erreur de type de valeur, au lieu de tronquer l'instant

#### Scenario: Date-heure sans décalage
- **WHEN** l'API de construction reçoit `2026-03-12T10:15:00` pour un attribut `DateTime`
- **THEN** la construction échoue avec une erreur de type de valeur

### Requirement: Valeurs déjà typées
Une valeur déjà typée SHALL être acceptée quand son type est celui de l'attribut, et un entier MUST aussi être accepté pour un `Float`. Toute autre combinaison MUST être refusée : un décimal pour un `Int`, un nombre ou un booléen pour un `String`. Une valeur nulle laisse l'attribut non renseigné.

#### Scenario: Entier pour un décimal
- **WHEN** l'API de construction reçoit l'entier 5 pour l'attribut `valeur`, de type `Float`
- **THEN** `valeur` vaut 5,0

#### Scenario: Décimal pour un entier
- **WHEN** l'API de construction reçoit le décimal 98,5 pour l'attribut `altitude`, de type `Int`
- **THEN** la construction échoue avec une erreur de type de valeur

#### Scenario: Nombre pour un texte
- **WHEN** l'API de construction reçoit le nombre 12 pour l'attribut `nom`, de type `String`
- **THEN** la construction échoue avec une erreur de type de valeur

#### Scenario: Valeur nulle
- **WHEN** l'API de construction reçoit une valeur nulle pour l'attribut `modele`
- **THEN** `modele` n'est pas renseigné

### Requirement: Lecture JSON avec la correspondance identité
Le lecteur JSON SHALL lire en flux un fichier dont la valeur est l'objet racine, sans construire d'arbre intermédiaire. Une clé porte le nom d'un attribut ou d'une référence de contenance de la classe de l'objet. Une référence multiple est un tableau JSON, une référence simple un objet JSON. Une valeur JSON `null` laisse l'attribut non renseigné.

#### Scenario: Fichier d'exemple du réseau
- **WHEN** le lecteur charge `examples/reseau-meteo/reseau-exemple.json` avec le modèle `examples/reseau-meteo/reseau.model.json`
- **THEN** le Store contient les stations, équipements, mesures, interventions, zones et mailles du fichier, avec leurs classes et les valeurs de leurs attributs

### Requirement: Classe concrète donnée par `_type_`
La clé réservée `_type_` SHALL donner le nom de la classe concrète d'un objet, à n'importe quelle position parmi ses clés. Sans `_type_`, l'objet prend le type déclaré de sa référence. Une classe inconnue, abstraite, ou qui n'est ni le type déclaré ni l'une de ses sous-classes MUST être refusée.

#### Scenario: `_type_` après d'autres clés
- **WHEN** un objet de la liste `equipements` s'écrit `{ "numero_serie": "T-17", "_type_": "Thermometre", "plage_min": -40 }`
- **THEN** il est chargé comme un `Thermometre`, avec les mêmes valeurs que si `_type_` avait été la première clé

#### Scenario: `_type_` absent avec un type déclaré concret
- **WHEN** un objet de la liste `coins` d'une maille n'a pas de clé `_type_`
- **THEN** il est chargé comme un `Point`, type déclaré de `coins`

#### Scenario: `_type_` absent avec un type déclaré abstrait
- **WHEN** un objet de la liste `equipements` n'a pas de clé `_type_`
- **THEN** le chargement est refusé, car `Equipement` est abstraite

#### Scenario: Classe non conforme
- **WHEN** un objet de la liste `equipements` a pour `_type_` la valeur `Releve`
- **THEN** le chargement est refusé avec une erreur qui cite `Releve` et `Equipement`

### Requirement: Données non conformes refusées
Le chargement MUST être refusé quand le fichier n'est pas du JSON valide, quand une clé n'est ni `_type_` ni le nom d'un attribut ou d'une référence de la classe, ou quand une valeur ne peut pas être convertie vers le type de son attribut. L'erreur SHALL donner le chemin de la donnée dans le modèle, formé des noms de référence et des positions dans les listes.

#### Scenario: Clé inconnue
- **WHEN** le troisième relevé de la première mesure contient la clé `valuer`
- **THEN** le chargement est refusé avec une erreur qui cite `valuer` et le chemin de ce relevé, par exemple `/stations/0/equipements/0/mesures/2`

#### Scenario: Valeur de mauvais type
- **WHEN** l'attribut `altitude` d'une station vaut `"haute"`
- **THEN** le chargement est refusé avec une erreur de type de valeur et le chemin `/stations/<position>/altitude`

#### Scenario: JSON mal formé
- **WHEN** le fichier s'arrête au milieu d'un objet
- **THEN** le chargement est refusé avec une erreur qui donne la ligne et la colonne

### Requirement: Lecture en flux ou depuis un tampon
Le lecteur JSON SHALL accepter un flux d'octets lu au fur et à mesure, ou un tampon contenant tout le fichier, et MUST donner le même Store dans les deux cas.

#### Scenario: Même résultat dans les deux modes
- **WHEN** le même fichier est chargé une fois en flux et une fois depuis un tampon
- **THEN** les deux Stores ont les mêmes objets, dans le même ordre, avec les mêmes classes et les mêmes valeurs
