# Spec Delta

## Purpose

Ranger en mémoire les objets d'une application, dont le modèle n'est connu qu'à l'exécution, et permettre de les créer, de lire et modifier leurs attributs, de naviguer dans leur arbre de contenance et de les supprimer, rapidement et avec peu de mémoire.

## ADDED Requirements

### Requirement: Identité des objets
Chaque objet SHALL recevoir à sa création un identifiant, stable pendant toute la session. Un identifiant MUST NOT être réutilisé, même après la suppression de l'objet. Le Store MUST indiquer si un identifiant désigne un objet encore présent.

#### Scenario: Identifiant d'un objet supprimé
- **WHEN** un objet est supprimé, puis un autre objet est créé
- **THEN** le nouvel objet reçoit un identifiant différent de celui de l'objet supprimé, et l'identifiant de l'objet supprimé est signalé comme ne désignant plus aucun objet

### Requirement: Création des objets
Le Store SHALL permettre de créer l'objet racine, puis des objets enfants dans une référence de contenance d'un objet existant, en fin de liste ou à une position donnée. La classe d'un nouvel objet MUST être concrète et MUST être le type de la référence ou l'une de ses sous-classes.

#### Scenario: Enfant ajouté en fin de liste
- **WHEN** on crée trois stations dans la référence `stations` de la racine
- **THEN** la référence `stations` contient ces trois objets, dans l'ordre de leur création

#### Scenario: Enfant inséré à une position
- **WHEN** on crée une station à la position 1 de la référence `stations`, qui contient déjà deux stations
- **THEN** la nouvelle station est la deuxième de la liste, et les deux autres gardent leur ordre

#### Scenario: Classe abstraite refusée
- **WHEN** on crée un objet de la classe abstraite `Equipement` dans la référence `equipements`
- **THEN** la création échoue et la liste reste inchangée

#### Scenario: Classe non conforme refusée
- **WHEN** on crée un objet `Releve` dans la référence `equipements`, de type `Equipement`
- **THEN** la création échoue et la liste reste inchangée

#### Scenario: Référence simple déjà occupée
- **WHEN** on crée un `Cercle` dans la référence simple `emprise` d'une zone qui contient déjà un `Polygone`
- **THEN** la création échoue et la zone garde son `Polygone`

### Requirement: Lecture des attributs
Le Store SHALL rendre la valeur d'un attribut d'un objet, désigné par l'identifiant de l'attribut : une valeur du type de l'attribut, ou l'indication qu'il n'est pas renseigné. La lecture MUST fonctionner de la même façon pour un attribut hérité.

#### Scenario: Attribut jamais renseigné
- **WHEN** on lit l'attribut `nom` d'une station créée sans valeur
- **THEN** la lecture indique que l'attribut n'est pas renseigné

#### Scenario: Attribut hérité dans une liste polymorphe
- **WHEN** la liste `equipements` contient un `Thermometre` et une `Camera`, et qu'on lit `numero_serie` sur chacun
- **THEN** on obtient le numéro de série de chacun

### Requirement: Modification des attributs
Le Store SHALL permettre de donner une valeur à un attribut, ou de le remettre à l'état non renseigné. Une valeur dont le type ne correspond pas à celui de l'attribut, ou un littéral étranger à son énuméré, MUST être refusé, et l'attribut garde alors sa valeur.

#### Scenario: Valeur modifiée
- **WHEN** on donne la valeur 12,5 à l'attribut `valeur` d'un relevé qui valait 11,0
- **THEN** la lecture de `valeur` donne 12,5

#### Scenario: Texte remplacé
- **WHEN** on remplace le nom « Brest » d'une station par « Brest-Guipavas »
- **THEN** la lecture de `nom` donne « Brest-Guipavas »

#### Scenario: Valeur de mauvais type refusée
- **WHEN** on donne un texte à l'attribut `altitude`, de type `Int`, qui valait 98
- **THEN** la modification échoue et `altitude` vaut toujours 98

#### Scenario: Attribut remis à l'état non renseigné
- **WHEN** on retire la valeur de l'attribut `modele` d'un thermomètre
- **THEN** la lecture de `modele` indique que l'attribut n'est pas renseigné

### Requirement: Navigation dans l'arbre de contenance
Le Store SHALL rendre, pour un objet, la classe de cet objet, les enfants d'une référence de contenance dans leur ordre, et son conteneur avec la référence qui le contient. L'objet racine n'a pas de conteneur.

#### Scenario: Remontée jusqu'à la racine
- **WHEN** on part d'un `Point`, coin d'une `Maille` d'une `ZoneAlerte`, et qu'on remonte de conteneur en conteneur
- **THEN** on passe par la maille, la zone, puis la racine, chaque fois avec le nom de la référence qui contient l'objet, et la racine n'a pas de conteneur

#### Scenario: Liste polymorphe
- **WHEN** on crée dans `equipements` un `Thermometre`, une `Camera`, puis un `Anemometre`
- **THEN** les enfants de `equipements` sont ces trois objets dans cet ordre, et la classe de chacun est celle de sa création

### Requirement: Retrait et suppression
Le Store SHALL permettre de supprimer un objet : il est retiré de la liste de son conteneur, sans changer l'ordre des autres enfants, et il ne désigne plus aucun objet, de même que tous ses descendants.

#### Scenario: Suppression au milieu d'une liste
- **WHEN** on supprime la deuxième de trois stations
- **THEN** la référence `stations` contient la première et la troisième, dans cet ordre

#### Scenario: Suppression d'un sous-arbre
- **WHEN** on supprime une station qui contient des équipements, des mesures et des annotations
- **THEN** la station, ses équipements, leurs mesures et leurs annotations ne désignent plus aucun objet, et les autres stations sont intactes

### Requirement: Transfert entre threads
Un Store SHALL pouvoir être transféré à un autre thread que celui qui l'a construit, pour qu'un chargement puisse plus tard se faire en tâche de fond.

#### Scenario: Store construit dans un autre thread
- **WHEN** un Store est construit et rempli dans un thread, puis rendu au thread principal
- **THEN** le thread principal lit les objets et leurs attributs comme s'il avait lui-même construit le Store
