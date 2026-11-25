## MODIFIED Requirements

### Requirement: Lecture des fichiers de modèle
Le système SHALL lire un modèle depuis un ou plusieurs fichiers `*.model.json` au format 0.1 : packages, éventuellement imbriqués, contenant des classes, des énumérés et des types de données. Une classe déclare ses attributs (`attributes`) et ses références de contenance (`references`). Un fichier mal formé, ou qui contient une clé inconnue, MUST être refusé avec une erreur qui le dit.

#### Scenario: Modèle de l'annuaire
- **WHEN** le système lit `examples/annuaire/annuaire.model.json`
- **THEN** le modèle compilé contient les classes `Racine`, `Annuaire` et `Personne` ; `Racine` a la référence de contenance simple `annuaire` vers `Annuaire`, `Annuaire` a la référence de contenance multiple `personnes` vers `Personne`, et `Personne` a les attributs `nom`, `prenom`, `date_naissance`, `nombre_enfants`, `taille`, `contact_autorise`, `situation`, `inscription` et `notes`, de types respectifs `String`, `String`, `Date`, `Int`, `Float`, `Boolean`, l'énuméré `SituationFamiliale`, `DateTime`, et le type dérivé `TexteLong`

#### Scenario: Modèle réparti sur deux fichiers
- **WHEN** le système lit deux fichiers de modèle, dont le second déclare une classe qui hérite d'une classe du premier
- **THEN** le modèle compilé contient les classes des deux fichiers, et la sous-classe hérite des attributs de sa super-classe

#### Scenario: Clé inconnue dans un fichier de modèle
- **WHEN** une classe d'un fichier de modèle contient la clé `atributes` (faute de frappe)
- **THEN** la lecture échoue avec une erreur qui cite la clé `atributes`

## ADDED Requirements

### Requirement: Erreurs situées dans leur fichier
Une erreur de lecture d'un fichier de modèle SHALL citer ce fichier, avec la ligne et la colonne. Une erreur de compilation SHALL citer le fichier qui déclare la classe en cause, en plus de la classe et de l'attribut ou de la référence.

#### Scenario: Erreur de syntaxe
- **WHEN** le second de deux fichiers de modèle s'arrête au milieu d'une classe
- **THEN** la lecture échoue avec une erreur qui cite le second fichier, la ligne et la colonne

#### Scenario: Type inconnu dans le second fichier
- **WHEN** une classe du second de deux fichiers de modèle déclare un attribut de type `Entier`
- **THEN** la compilation échoue avec une erreur qui cite le second fichier, la classe, l'attribut et le type `Entier`

### Requirement: Version du format des fichiers de modèle
La valeur de `formatVersion` d'un fichier de modèle MUST être `0.1`. Une autre version MUST être refusée, avec une erreur qui cite le fichier, la version lue et la version attendue.

#### Scenario: Version inconnue
- **WHEN** un fichier de modèle déclare `"formatVersion": "0.2"`
- **THEN** la lecture échoue avec une erreur qui cite le fichier, `0.2` et `0.1`

### Requirement: Clé `$schema` acceptée dans les fichiers de modèle
La clé `$schema` SHALL être acceptée à la racine d'un fichier de modèle, avec un texte pour valeur, et ignorée par le système : elle ne sert qu'à l'éditeur de texte.

#### Scenario: Fichier de modèle avec `$schema`
- **WHEN** un fichier de modèle commence par `"$schema": "../../schemas/model.schema.json"`
- **THEN** la lecture réussit et donne le même modèle que sans cette clé

### Requirement: Bornes cohérentes
La borne inférieure (`lowerBound`) d'un attribut MUST valoir 0 ou 1 : à 1, l'attribut est obligatoire. Pour une référence, la borne inférieure MUST être positive ou nulle, la borne supérieure (`upperBound`) MUST valoir -1 (sans limite) ou au moins 1, et la borne inférieure MUST NOT dépasser une borne supérieure limitée. Sinon, la compilation échoue avec une erreur qui cite la classe, l'attribut ou la référence, et les bornes.

#### Scenario: Attribut obligatoire
- **WHEN** l'attribut `nom` de `Personne` est déclaré avec `lowerBound` à 1
- **THEN** le modèle compilé indique que `nom` est obligatoire, et que `prenom`, déclaré sans `lowerBound`, ne l'est pas

#### Scenario: Borne inférieure négative
- **WHEN** une référence est déclarée avec `lowerBound` à -1
- **THEN** la compilation échoue avec une erreur qui cite la référence et sa borne inférieure

#### Scenario: Bornes inversées
- **WHEN** une référence est déclarée avec `lowerBound` à 3 et `upperBound` à 2
- **THEN** la compilation échoue avec une erreur qui cite la référence et ses deux bornes

#### Scenario: Borne supérieure nulle
- **WHEN** une référence est déclarée avec `upperBound` à 0
- **THEN** la compilation échoue avec une erreur qui cite la référence et sa borne supérieure

#### Scenario: Attribut de borne inférieure 2
- **WHEN** un attribut est déclaré avec `lowerBound` à 2
- **THEN** la compilation échoue avec une erreur qui cite l'attribut et sa borne inférieure

### Requirement: Littéraux uniques dans un énuméré
Deux littéraux d'un même énuméré MUST NOT porter le même nom. Sinon, la compilation échoue avec une erreur qui cite l'énuméré et le littéral.

#### Scenario: Littéral en double
- **WHEN** l'énuméré `Qualite` déclare les littéraux `BONNE`, `DOUTEUSE`, `BONNE`
- **THEN** la compilation échoue avec une erreur qui cite `Qualite` et `BONNE`
