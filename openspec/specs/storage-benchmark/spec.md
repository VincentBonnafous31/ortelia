# storage-benchmark Specification

## Purpose

Mesurer de façon reproductible les performances du stockage dynamique (mémoire, chargement, accès aux attributs et navigation, modification) sur des données représentatives, pour choisir sa disposition et en surveiller l'évolution.

## Requirements

### Requirement: Modèle d'essai représentatif
Le dépôt SHALL contenir un modèle d'essai (`examples/reseau-meteo/reseau.model.json`) qui couvre : un arbre de contenance de profondeur 4 sous la racine ; des listes polymorphes ; un héritage sur trois niveaux ; tous les types de base ; des énumérés ; des textes longs ; une référence simple dont le type est abstrait. Un petit fichier de données conforme (`reseau-exemple.json`) l'accompagne.

#### Scenario: Modèle et exemple chargés
- **WHEN** on compile le modèle d'essai, puis qu'on charge le fichier d'exemple avec ce modèle
- **THEN** la compilation et le chargement réussissent, et le Store contient des objets à la profondeur 4 sous la racine

### Requirement: Générateur de données reproductible
La commande `cargo xtask gen-data` SHALL écrire un fichier JSON conforme au modèle d'essai, d'une taille au moins égale à la taille demandée. On y règle aussi la graine, la part de texte long et la part d'objets dont `_type_` n'est pas la première clé. Les mêmes paramètres MUST produire un fichier identique, octet pour octet.

#### Scenario: Même graine, même fichier
- **WHEN** on génère deux fichiers de 200 Ko avec la même graine et les mêmes paramètres
- **THEN** les deux fichiers sont identiques

#### Scenario: Taille demandée atteinte
- **WHEN** on génère un fichier de 1 Mo
- **THEN** le fichier fait au moins 1 Mo, et il dépasse cette taille de moins de 10 %

#### Scenario: Fichier conforme au modèle
- **WHEN** on charge avec le modèle d'essai un fichier généré avec une part d'objets à `_type_` tardif de 10 %
- **THEN** le chargement réussit

#### Scenario: Sans texte long
- **WHEN** on génère un fichier avec une part de texte long nulle
- **THEN** aucun attribut de texte long (description, compte rendu, texte d'annotation) n'y figure

### Requirement: Programme de mesure
Un programme de mesure SHALL charger un fichier généré avec chaque implémentation du Store, et rendre un rapport des mesures : mémoire, chargement, accès aux attributs et navigation, modification. Chaque cas MUST tourner dans un processus séparé, pour que le pic de mémoire d'un cas ne fausse pas le suivant. Une mesure indisponible sur le système est signalée comme telle.

#### Scenario: Toutes les mesures sur un petit fichier
- **WHEN** on exécute chaque mesure sur le fichier d'exemple
- **THEN** chaque mesure rend une valeur, et le rapport contient une ligne par mesure et par implémentation

#### Scenario: Mémoire indisponible
- **WHEN** le programme tourne sur un système où le pic de mémoire du processus ne peut pas être lu
- **THEN** le rapport indique « non disponible » pour les mesures de mémoire, et donne les autres mesures
