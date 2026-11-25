# metamodel Specification

## Purpose

Décrire les données d'une application par un modèle lu à l'exécution (packages, classes, attributs, contenance, énumérés, types de données), et le compiler sous une forme où chaque attribut a une position fixe et chaque nom est résolu une seule fois.

## Requirements

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

### Requirement: Types de base
Le métamodèle SHALL fournir les types de base `String`, `Int` (entier signé de 64 bits), `Float` (flottant de 64 bits), `Boolean`, `Date` (date civile, sans heure ni fuseau) et `DateTime` (instant UTC, à la microseconde). Ces noms sont disponibles dans tout modèle sans déclaration.

#### Scenario: Attribut d'un type de base
- **WHEN** un modèle déclare l'attribut `altitude` de type `Int` dans la classe `Station`
- **THEN** le modèle compilé donne `Int` comme type de cet attribut

#### Scenario: Type inconnu
- **WHEN** un attribut est déclaré avec le type `Entier`, qui n'est ni un type de base, ni un énuméré, ni un type de données du modèle
- **THEN** la compilation échoue avec une erreur qui cite le type `Entier` et l'attribut concerné

### Requirement: Énumérés et types dérivés
Un énuméré SHALL déclarer une liste ordonnée de littéraux, et un attribut de ce type prend l'un de ces littéraux pour valeur. Un type de données déclaré avec un type de base (`baseType`) SHALL être stocké comme ce type de base.

#### Scenario: Littéraux d'un énuméré
- **WHEN** un modèle déclare l'énuméré `Qualite` avec les littéraux `BONNE`, `DOUTEUSE`, `FAUSSE`
- **THEN** le modèle compilé donne ces trois littéraux dans cet ordre

#### Scenario: Type dérivé d'un type de base
- **WHEN** un modèle déclare le type de données `Identifier` de type de base `String`, et un attribut de type `Identifier`
- **THEN** cet attribut est stocké comme un `String`

### Requirement: Héritage simple
Une classe SHALL pouvoir déclarer une super-classe (`superClass`) au plus ; elle hérite alors de tous les attributs et de toutes les références de ses ancêtres. Une classe déclarée abstraite (`abstract`) MUST NOT être instanciable. Une super-classe introuvable ou un cycle d'héritage MUST faire échouer la compilation.

#### Scenario: Héritage sur trois niveaux
- **WHEN** `Thermometre` hérite de `Capteur`, qui hérite de `Equipement`
- **THEN** `Thermometre` a les attributs de `Equipement`, puis ceux de `Capteur`, puis les siens, et sa référence `mesures` déclarée dans `Capteur`

#### Scenario: Super-classe introuvable
- **WHEN** une classe déclare la super-classe `Equipment`, absente du modèle
- **THEN** la compilation échoue avec une erreur qui cite `Equipment`

#### Scenario: Cycle d'héritage
- **WHEN** la classe `A` hérite de `B` et la classe `B` hérite de `A`
- **THEN** la compilation échoue avec une erreur qui cite le cycle

### Requirement: Position fixe des attributs
Chaque attribut et chaque référence de contenance SHALL recevoir une position fixe dans sa classe. Cette position MUST être la même dans toutes les sous-classes. Les attributs hérités viennent d'abord, dans l'ordre de leurs ancêtres.

#### Scenario: Attribut hérité dans deux sous-classes
- **WHEN** l'attribut `numero_serie` est déclaré dans `Equipement`, dont héritent `Thermometre` et `Camera`
- **THEN** l'identifiant de `numero_serie` obtenu depuis `Thermometre` est le même que celui obtenu depuis `Camera`

### Requirement: Résolution des noms
Le système SHALL résoudre une seule fois un nom de classe, d'attribut ou de référence en un identifiant, utilisé ensuite pour tous les accès. Un attribut hérité MUST se trouver par son nom depuis une sous-classe. Deux classes du même nom, ou deux attributs ou références du même nom dans une classe et ses ancêtres, MUST faire échouer la compilation.

#### Scenario: Attribut hérité trouvé depuis la sous-classe
- **WHEN** on recherche l'attribut `instant` dans la classe `Releve`, qui hérite de `Mesure` où il est déclaré
- **THEN** la recherche donne l'identifiant de `instant`

#### Scenario: Nom inconnu
- **WHEN** on recherche l'attribut `temperature` dans la classe `Releve`
- **THEN** la recherche ne donne aucun identifiant

#### Scenario: Classe en double
- **WHEN** deux packages déclarent chacun une classe `Point`
- **THEN** la compilation échoue avec une erreur qui cite `Point`

#### Scenario: Attribut qui redéclare un attribut hérité
- **WHEN** `Thermometre` déclare un attribut `modele` déjà déclaré dans son ancêtre `Equipement`
- **THEN** la compilation échoue avec une erreur qui cite `modele` et les deux classes

### Requirement: Attributs multivalués refusés
Un attribut dont la borne supérieure (`upperBound`) n'est pas 1 MUST faire échouer la compilation, avec une erreur qui indique que les attributs multivalués ne sont pas encore pris en charge. Les références de contenance, elles, peuvent être multiples.

#### Scenario: Attribut multivalué
- **WHEN** un modèle déclare un attribut `mots_cles` de type `String` avec `upperBound` à -1
- **THEN** la compilation échoue avec une erreur qui cite `mots_cles` et indique que les attributs multivalués ne sont pas pris en charge

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
