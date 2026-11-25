# change-tracking Specification

## Purpose

Faire passer toute modification des données d'une application par un seul point, le Document, qui la trace : une révision qui augmente à chaque transaction, et un journal qui dit ce qui a changé, pour que la présentation sache quels caches sont périmés et que les incréments suivants puissent bâtir l'indicateur « modifié » et l'enregistrement.

## Requirements

### Requirement: Seule voie de modification
Hors de la bibliothèque du modèle, les données d'une application SHALL ne pouvoir être modifiées que par le Document : création, modification, suppression et intégration d'un fragment. Le Store que tient le Document MUST rester lisible par tous, sans pouvoir être modifié directement.

#### Scenario: Modification directe du Store impossible
- **WHEN** un code extérieur à la bibliothèque du modèle tente de modifier un attribut directement par le Store d'un Document
- **THEN** ce code ne compile pas, ce que vérifie un test de non-compilation

#### Scenario: Lecture par le Store
- **WHEN** un code extérieur à la bibliothèque du modèle lit les attributs et parcourt les enfants des objets d'un Document
- **THEN** il obtient les valeurs et les enfants que le Document a enregistrés

### Requirement: Racine du Document
Un Document SHALL être créé pour une classe de racine, qui MUST être concrète, et il contient dès sa création un objet racine de cette classe, sans aucun enfant. Sa révision vaut alors 0 et son journal est vide. La racine MUST NOT être supprimée : on la remplace en intégrant un fragment à la racine.

#### Scenario: Document neuf
- **WHEN** on crée un Document pour la classe `Racine` du modèle de l'annuaire
- **THEN** le Document a une racine de classe `Racine`, dont la référence `annuaire` est vide, sa révision vaut 0 et son journal est vide

#### Scenario: Racine abstraite refusée
- **WHEN** on crée un Document pour une classe abstraite
- **THEN** la création échoue avec une erreur qui cite la classe

#### Scenario: Suppression de la racine refusée
- **WHEN** on demande au Document de supprimer sa racine
- **THEN** la suppression échoue, la racine existe toujours et le journal ne reçoit rien

### Requirement: Transactions
Toute modification SHALL être faite dans une transaction, qui regroupe des opérations appliquées tout de suite, dans l'ordre. À la fin d'une transaction qui a changé quelque chose, la révision augmente de un et le journal reçoit la transaction, avec ses changements dans l'ordre. Une transaction qui n'a rien changé MUST laisser la révision et le journal tels quels.

#### Scenario: Trois modifications dans une transaction
- **WHEN** une transaction change le nom, le prénom et la date de naissance d'une personne, dans un Document de révision 4
- **THEN** la révision vaut 5, et le journal reçoit une seule transaction, de révision 5, avec les trois changements dans cet ordre

#### Scenario: Transaction vide
- **WHEN** une transaction se termine sans aucune opération
- **THEN** la révision et le journal sont inchangés

### Requirement: Changement de valeur enregistré
Un changement de valeur SHALL être enregistré avec l'objet, l'attribut, l'ancienne valeur et la nouvelle, textes compris ; une valeur absente est notée comme telle. Donner à un attribut la valeur qu'il a déjà MUST NOT rien enregistrer. Une valeur refusée (mauvais type, littéral inconnu) MUST NOT rien enregistrer, et l'attribut garde sa valeur.

#### Scenario: Ancienne et nouvelle valeur
- **WHEN** une transaction remplace le nom « Dupont » d'une personne par « Dupond »
- **THEN** le journal contient un changement de valeur sur cette personne et l'attribut `nom`, d'ancienne valeur « Dupont » et de nouvelle valeur « Dupond »

#### Scenario: Valeur inchangée
- **WHEN** une transaction donne à l'attribut `nom` d'une personne la valeur « Dupont » qu'il a déjà, et ne fait rien d'autre
- **THEN** la révision et le journal sont inchangés

#### Scenario: Valeur refusée
- **WHEN** une transaction donne un texte à l'attribut `nombre_enfants`, de type `Int`, qui valait 2
- **THEN** l'opération échoue, `nombre_enfants` vaut toujours 2, et le journal ne reçoit aucun changement pour cette opération

### Requirement: Création et suppression enregistrées
La création d'un objet SHALL être enregistrée avec l'objet, sa classe, son conteneur, la référence et sa position dans la liste. Sa suppression SHALL être enregistrée avec l'objet, son conteneur, la référence et la position qu'il y occupait ; l'objet et ses descendants ne désignent alors plus aucun objet. Une suppression en lot enregistre un changement par objet supprimé.

#### Scenario: Création enregistrée
- **WHEN** une transaction crée une personne en fin de la liste `personnes`, qui contient déjà dix personnes
- **THEN** le journal contient une création de cette personne, de classe `Personne`, dans la référence `personnes` de l'annuaire, à la position 10

#### Scenario: Suppression enregistrée
- **WHEN** une transaction supprime la troisième personne de l'annuaire
- **THEN** le journal contient une suppression de cette personne, de la référence `personnes` de l'annuaire, à la position 2, et cette personne ne désigne plus aucun objet

#### Scenario: Suppression en lot
- **WHEN** une transaction supprime d'un coup la première, la troisième et la cinquième de cinq personnes
- **THEN** la liste contient la deuxième puis la quatrième, la révision n'augmente que de un, et la transaction contient trois suppressions

### Requirement: Consultation des changements
Le Document SHALL rendre sa révision courante, et les transactions dont la révision est postérieure à une révision donnée, dans l'ordre. Seule une transaction qui change quelque chose fait évoluer la révision : la lecture des données, même répétée, MUST la laisser telle quelle.

#### Scenario: Changements depuis une révision
- **WHEN** un consommateur a noté la révision 3, puis deux transactions modifient le Document
- **THEN** les changements depuis la révision 3 sont ces deux transactions, dans l'ordre, et les changements depuis la révision courante sont vides

#### Scenario: Lectures sans effet
- **WHEN** on lit mille fois des attributs et des enfants d'un Document
- **THEN** sa révision n'a pas changé

### Requirement: Intégration d'un fragment dans une branche
Le Document SHALL remplacer le contenu d'une branche, c'est-à-dire d'une référence de contenance d'un objet, ou de la racine, par les objets d'un fragment, dans l'ordre du fragment. La transaction ne contient qu'un changement : la branche et ses nouveaux objets. Les anciens objets de la branche et leurs descendants ne désignent plus aucun objet ; ceux des autres branches gardent leurs identifiants.

#### Scenario: Remplacement d'une liste
- **WHEN** on intègre dans la branche `stations` d'un réseau qui a deux stations et une zone un fragment de trois stations
- **THEN** la branche `stations` contient les trois stations du fragment, dans leur ordre, les deux anciennes stations et leurs équipements ne désignent plus aucun objet, la zone garde son identifiant, et le journal reçoit une transaction qui ne contient qu'un changement : la branche `stations` et ses trois nouveaux objets

#### Scenario: Valeurs intactes après intégration
- **WHEN** on intègre un fragment dont les objets ont des valeurs de tous les types de base et un texte plus long qu'un bloc de l'arène des textes
- **THEN** les objets intégrés ont exactement les mêmes valeurs et les mêmes enfants que dans le fragment, et leurs identifiants diffèrent de tous ceux que le Document a déjà attribués

#### Scenario: Remplacement de la racine
- **WHEN** on intègre à la racine un fragment qui contient un objet de la classe de la racine
- **THEN** cet objet devient la racine, l'ancienne racine et tous ses descendants ne désignent plus aucun objet, et le journal reçoit un seul changement, qui porte sur la racine

### Requirement: Fragment non conforme refusé
Un fragment est construit pour un type et une multiplicité : un objet, ou une liste d'objets. L'intégration MUST être refusée, et le Document laissé tel quel, quand ce type ou cette multiplicité n'est pas celui de la branche, quand le fragment ne contient aucun objet pour la racine, ou quand il a été construit avec un autre modèle.

#### Scenario: Fragment d'un autre type
- **WHEN** on intègre dans la branche `stations` un fragment construit pour la branche `zones`
- **THEN** l'intégration échoue, la branche `stations` est inchangée, et ni la révision ni le journal ne changent

#### Scenario: Liste pour une référence simple
- **WHEN** on intègre dans la référence simple `annuaire` de la racine un fragment construit pour une liste d'annuaires
- **THEN** l'intégration échoue et la racine garde son annuaire

#### Scenario: Racine vide
- **WHEN** on intègre à la racine un fragment construit pour la racine mais qui ne contient aucun objet
- **THEN** l'intégration échoue et la racine est inchangée
