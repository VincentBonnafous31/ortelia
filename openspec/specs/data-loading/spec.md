# data-loading Specification

## Purpose

Remplir un Store à partir d'un fichier de données : une seule API de construction pour tous les lecteurs, présents et futurs, qui convertit les valeurs selon le modèle, et un lecteur JSON en flux qui applique la correspondance identité entre les clés du fichier et le modèle.

## Requirements

### Requirement: Construction des objets dans l'ordre du fichier
Le système SHALL fournir une API de construction, la seule voie par laquelle un lecteur crée des données. Elle remplit un fragment neuf, construit pour une branche : un type et une multiplicité, un objet ou une liste d'objets. Le lecteur l'appelle dans l'ordre du fichier : début d'un objet de tête ou d'un objet enfant dans une référence de contenance, valeurs de ses attributs, fin de l'objet. Le fragment ne contient que ce que le lecteur a créé.

#### Scenario: Arbre construit dans l'ordre des appels
- **WHEN** un lecteur crée, dans un fragment construit pour la racine du réseau, l'objet racine, une station dans `stations`, puis un thermomètre dans `equipements` de cette station
- **THEN** le fragment contient la racine, sa station et le thermomètre de la station, à leur place dans l'arbre de contenance

#### Scenario: Fragment d'une liste
- **WHEN** un lecteur crée trois objets de tête dans un fragment construit pour la branche `personnes`
- **THEN** le fragment contient trois objets de tête de classe `Personne`, dans l'ordre de leur création

#### Scenario: Deux objets pour une branche simple
- **WHEN** un lecteur commence un second objet de tête dans un fragment construit pour la référence simple `annuaire`
- **THEN** la construction signale une erreur qui indique que la référence simple contient plus d'un objet

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
Le lecteur JSON SHALL lire en flux un fichier, sans construire d'arbre intermédiaire. La valeur du fichier est un objet JSON quand la branche visée est la racine ou une référence simple, et un tableau d'objets JSON quand c'est une liste. Une clé porte le nom d'un attribut ou d'une référence de contenance de la classe de l'objet. Une référence multiple est un tableau JSON, une référence simple un objet JSON. Une valeur JSON `null` laisse l'attribut non renseigné.

#### Scenario: Fichier d'exemple du réseau
- **WHEN** le lecteur charge `examples/reseau-meteo/reseau-exemple.json` pour la racine, avec le modèle `examples/reseau-meteo/reseau.model.json`
- **THEN** le fragment contient les stations, équipements, mesures, interventions, zones et mailles du fichier, avec leurs classes et les valeurs de leurs attributs

#### Scenario: Fichier des personnes
- **WHEN** le lecteur charge `examples/annuaire/personnes.json` pour la branche `personnes`
- **THEN** le fragment contient dix objets de tête de classe `Personne`, dans l'ordre du fichier, avec les valeurs de leurs attributs

#### Scenario: Objet au lieu d'un tableau
- **WHEN** le lecteur charge pour la branche `personnes` un fichier dont la valeur est un objet JSON
- **THEN** le chargement est refusé avec une erreur qui indique qu'un tableau était attendu

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
Le chargement MUST être refusé quand le fichier n'est pas du JSON valide, quand une clé n'est ni `_type_` ni le nom d'un attribut ou d'une référence de la classe, ou quand une valeur ne peut pas être convertie vers le type de son attribut. L'erreur SHALL donner le chemin de la donnée depuis la racine de l'application, formé des noms de référence et des positions dans les listes.

#### Scenario: Clé inconnue
- **WHEN** le troisième relevé de la première mesure contient la clé `valuer`
- **THEN** le chargement est refusé avec une erreur dont le message destiné à l'utilisateur cite `valuer`, et qui donne le chemin de ce relevé, par exemple `/stations/0/equipements/0/mesures/2`

#### Scenario: Valeur de mauvais type
- **WHEN** l'attribut `altitude` d'une station vaut `"haute"`
- **THEN** le chargement est refusé avec une erreur de type de valeur et le chemin `/stations/<position>/altitude`

#### Scenario: JSON mal formé
- **WHEN** le fichier s'arrête au milieu d'un objet
- **THEN** le chargement est refusé avec une erreur qui donne la ligne et la colonne

#### Scenario: Chemin depuis la racine de l'application
- **WHEN** la quatrième personne d'un fichier chargé pour la branche `/annuaire/personnes` a la valeur `"deux"` pour `nombre_enfants`
- **THEN** l'erreur donne le chemin `/annuaire/personnes/3/nombre_enfants`

### Requirement: Lecture en flux ou depuis un tampon
Le lecteur JSON SHALL accepter un flux d'octets lu au fur et à mesure, ou un tampon contenant tout le fichier, et MUST donner le même fragment dans les deux cas.

#### Scenario: Même résultat dans les deux modes
- **WHEN** le même fichier est chargé une fois en flux et une fois depuis un tampon
- **THEN** les deux fragments ont les mêmes objets, dans le même ordre, avec les mêmes classes et les mêmes valeurs

### Requirement: Erreurs rassemblées
Une erreur de données SHALL ne pas interrompre la lecture : la valeur fautive est ignorée, ou l'objet entier quand sa classe est refusée, et la lecture continue. Le chargement est refusé à la fin, avec les 100 premières erreurs, dans l'ordre du fichier, et leur nombre total. Une erreur de syntaxe ou de lecture du fichier MUST arrêter la lecture, et s'ajoute aux erreurs déjà trouvées.

#### Scenario: Trois erreurs dans trois objets
- **WHEN** un fichier de personnes contient une valeur de mauvais type dans la deuxième personne, une clé inconnue dans la cinquième et une classe inconnue pour la huitième
- **THEN** le chargement est refusé avec trois erreurs, dans l'ordre du fichier, chacune avec son chemin, et un total de trois

#### Scenario: Plafond des erreurs
- **WHEN** un fichier contient 150 personnes qui ont chacune une valeur de mauvais type
- **THEN** le chargement est refusé avec les 100 premières erreurs et un total de 150

#### Scenario: Erreur de syntaxe après une erreur de données
- **WHEN** un fichier contient une clé inconnue dans la première personne, puis s'arrête au milieu de la troisième
- **THEN** le chargement est refusé avec deux erreurs : la clé inconnue, puis l'erreur de syntaxe avec sa ligne et sa colonne

### Requirement: Attribut obligatoire absent
Quand un objet n'a pas de valeur pour un attribut obligatoire, le chargement SHALL le garder, sans refuser le fichier, et produire un diagnostic qui cite l'attribut et le chemin de l'objet. Les diagnostics sont rendus avec le fragment : les 100 premiers, dans l'ordre du fichier, et leur nombre total.

#### Scenario: Personne sans nom
- **WHEN** la deuxième personne d'un fichier de personnes n'a pas de clé `nom`, et une autre a `"nom": null`
- **THEN** le chargement réussit, le fragment contient toutes les personnes, et deux diagnostics citent `nom`, chacun avec le chemin de sa personne

#### Scenario: Diagnostic et erreur dans le même fichier
- **WHEN** un fichier a une personne sans `nom` et une autre avec une clé inconnue
- **THEN** le chargement est refusé à cause de la clé inconnue, et le diagnostic de la personne sans nom est rendu avec l'erreur

### Requirement: Valeur fautive absente du journal
Une erreur de type de valeur SHALL garder la valeur fautive, réduite à ses 40 premiers caractères suivis de « … » quand elle est plus longue, pour le message destiné à l'utilisateur. Sa forme destinée au journal MUST NOT contenir cette valeur. Il en va de même pour une clé inconnue et pour le nom de classe inconnu d'un `_type_`, textes tirés du fichier qui peuvent être des données (un dictionnaire employé par erreur) : seul le message destiné à l'utilisateur les cite.

#### Scenario: Les deux formes d'une erreur
- **WHEN** l'attribut `nombre_enfants` d'une personne vaut `"deux"`
- **THEN** le message destiné à l'utilisateur cite `deux`, et la forme destinée au journal cite l'attribut, le type attendu et le chemin, mais pas `deux`

#### Scenario: Clé inconnue absente du journal
- **WHEN** une personne contient la clé `Dupont Jean, 12 rue X`
- **THEN** le message destiné à l'utilisateur cite cette clé, et la forme destinée au journal indique une clé inconnue et son chemin, sans la citer

#### Scenario: Valeur longue
- **WHEN** une valeur fautive fait 1 000 caractères
- **THEN** le message destiné à l'utilisateur n'en cite que les 40 premiers, suivis de « … »

### Requirement: Fragment transférable entre threads
Un fragment SHALL pouvoir être transféré à un autre thread que celui qui l'a construit, pour qu'une source puisse plus tard être lue en tâche de fond puis intégrée par le thread principal.

#### Scenario: Fragment lu dans un autre thread
- **WHEN** un fragment est lu dans un thread, puis rendu au thread principal qui l'intègre dans un Document
- **THEN** le Document contient les objets du fragment, avec leurs valeurs
