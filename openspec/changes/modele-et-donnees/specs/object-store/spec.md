## ADDED Requirements

### Requirement: Parcours d'un sous-arbre
Le Store SHALL rendre tous les descendants d'un objet, l'objet compris : chaque objet une seule fois, le parent avant ses enfants, et les enfants d'une référence dans leur ordre. Le parcours MUST fonctionner quelle que soit la profondeur de l'arbre.

#### Scenario: Descendants d'une station
- **WHEN** on parcourt le sous-arbre d'une station qui a deux équipements, dont un capteur avec trois mesures
- **THEN** on obtient la station, ses deux équipements et les trois mesures, soit six objets, chacun une fois, la station en premier et chaque équipement avant ses mesures

#### Scenario: Arbre très profond
- **WHEN** on parcourt le sous-arbre d'un objet qui a 100 000 générations de descendants
- **THEN** le parcours rend les 100 001 objets sans dépasser la pile d'appels

### Requirement: Suppression en lot
Le Store SHALL supprimer d'un coup plusieurs objets, avec leurs descendants. Chaque liste concernée MUST n'être parcourue qu'une fois, quel que soit le nombre d'objets qu'on y supprime, et les enfants restants gardent leur ordre.

#### Scenario: Plusieurs objets d'une même liste
- **WHEN** on supprime d'un coup la première, la troisième et la cinquième de cinq stations
- **THEN** la liste contient la deuxième puis la quatrième station, et les trois stations supprimées et leurs descendants ne désignent plus aucun objet

#### Scenario: Grand lot
- **WHEN** on supprime d'un coup 50 000 personnes d'une liste de 100 000
- **THEN** la liste contient les 50 000 autres dans leur ordre, et la suppression prend moins d'une seconde, même dans une compilation de test non optimisée (une suppression objet par objet décalerait la liste 50 000 fois)

#### Scenario: Objets de listes différentes
- **WHEN** on supprime d'un coup une station et une zone
- **THEN** la liste `stations` et la liste `zones` ont chacune perdu cet objet, et les autres gardent leur ordre
