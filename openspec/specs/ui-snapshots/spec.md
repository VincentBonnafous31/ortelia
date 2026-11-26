# ui-snapshots Specification

## Purpose

Vérifier le rendu réel de l'interface en comparant l'image rendue à une image de référence versionnée, sur une plateforme de rendu unique et reproductible, avec des images de référence que l'on peut régénérer sur le poste de développement.

## Requirements

### Requirement: Comparaison du rendu à une image de référence
Un test de capture SHALL rendre une image de l'interface, à une taille, une échelle et un thème fixés par le test, puis la comparer à une image de référence PNG versionnée dans le dépôt. Il MUST échouer quand le nombre de pixels qui dépassent le seuil de tolérance par pixel est supérieur au nombre de pixels fautifs tolérés, avec un message qui donne ce nombre.

#### Scenario: Rendu conforme
- **WHEN** un test de capture rend une image identique à sa référence
- **THEN** le test réussit

#### Scenario: Rendu différent
- **WHEN** on compare un rendu de la fenêtre principale avec un autre texte central à une référence rendue avec le texte d'origine
- **THEN** la comparaison échoue, et donne un nombre de pixels fautifs supérieur à zéro

### Requirement: Tolérances réglées en un seul endroit
Le seuil de tolérance par pixel et le nombre de pixels fautifs tolérés SHALL être réglés dans un seul fichier, à la racine du dépôt, avec une valeur par système d'exploitation au besoin. Le nombre de pixels fautifs tolérés MUST NOT dépasser 10, pour qu'un trait d'un pixel déplacé ou une petite icône mal rendue fasse échouer le test.

#### Scenario: Tolérances lues à la racine
- **WHEN** on lit le fichier des tolérances à la racine du dépôt
- **THEN** il donne le seuil par pixel et le nombre de pixels fautifs tolérés, et ce nombre ne dépasse 10 pour aucun système

### Requirement: Plateforme de rendu unique
Les tests de capture SHALL être comparés sur une seule plateforme de rendu : Linux, avec le rendu logiciel lavapipe. Sur les autres systèmes, ils MUST être signalés comme ignorés, avec leur raison, sans faire échouer les tests. Une référence produite sur le poste de développement, dans WSL2, MUST être acceptée par la CI.

#### Scenario: Système autre que Linux
- **WHEN** on lance les tests du workspace sous Windows
- **THEN** les tests de capture sont signalés comme ignorés, et les autres tests s'exécutent normalement

#### Scenario: Référence produite dans WSL2
- **WHEN** une image de référence produite dans WSL2 est versionnée, puis que la CI s'exécute
- **THEN** le test de capture correspondant réussit dans la CI

### Requirement: Capture de la fenêtre principale
La fenêtre principale SHALL avoir un test de capture, rendu par l'application egui complète ouverte sur `examples/annuaire`, avec son contenu fourni par la présentation : la table des personnes, avec ses en-têtes, son tri initial et son filtre vide.

#### Scenario: Fenêtre principale conforme
- **WHEN** on rend la fenêtre principale de l'annuaire sur la plateforme de rendu retenue
- **THEN** l'image concorde avec sa référence `main_window`

### Requirement: Capture de la boîte d'erreur d'ouverture
La boîte d'erreur d'une application qui ne s'ouvre pas SHALL avoir un test de capture, rendu par l'application egui complète, avec des erreurs fixées par le test.

#### Scenario: Boîte d'erreur conforme
- **WHEN** on rend la fenêtre d'une application dont l'ouverture a échoué avec deux erreurs, sur la plateforme de rendu retenue
- **THEN** l'image concorde avec sa référence `open_error`
