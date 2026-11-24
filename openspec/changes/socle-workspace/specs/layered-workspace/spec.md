# Spec Delta

## Purpose

Faire respecter la séparation en couches d'Ortelia : chaque crate du workspace appartient à une couche, ne dépend que des couches inférieures, et aucune couche située sous les interfaces ne dépend d'une bibliothèque d'interface. La vérification est automatique et bloque la CI.

## ADDED Requirements

### Requirement: Couches du workspace
Le workspace SHALL ranger ses crates en couches numérotées : 0 `ortelia-model` ; 1 `ortelia-config` et `ortelia-expr` ; 2 `ortelia-service` ; 3 `ortelia-presentation` ; 4 les crates `ortelia-ui-*` ; 5 l'exécutable `ortelia`. Les crates d'outillage du dépôt, qui ne font pas partie de l'application, sont déclarées comme outils.

#### Scenario: Le workspace livré respecte ses couches
- **WHEN** la vérification des couches est lancée sur le workspace de ce changement
- **THEN** elle ne signale aucune violation et se termine avec le code 0

### Requirement: Dépendances vers les couches inférieures uniquement
Une crate d'une couche MUST NOT dépendre d'une crate du workspace de même niveau ou de niveau supérieur. Elle peut dépendre de n'importe quelle couche inférieure, directement ou non. La règle vaut pour tous les types de dépendance : normale, de développement et de compilation.

#### Scenario: Dépendance vers une couche inférieure acceptée
- **WHEN** `ortelia-presentation` dépend de `ortelia-model`
- **THEN** la vérification ne signale rien pour cette dépendance

#### Scenario: Dépendance vers une couche supérieure refusée
- **WHEN** `ortelia-service` dépend de `ortelia-presentation`
- **THEN** la vérification signale une violation qui nomme les deux crates et leurs niveaux

#### Scenario: Dépendance entre crates de même niveau refusée
- **WHEN** `ortelia-config` dépend de `ortelia-expr`
- **THEN** la vérification signale une violation qui nomme les deux crates

#### Scenario: Dépendance de développement vers une couche supérieure refusée
- **WHEN** `ortelia-model` déclare `ortelia-service` comme dépendance de développement
- **THEN** la vérification signale une violation

### Requirement: Outils hors de l'application
Une crate rangée dans une couche MUST NOT dépendre d'une crate déclarée comme outil.

#### Scenario: Une couche qui dépend d'un outil est refusée
- **WHEN** `ortelia-service` dépend de la crate d'outillage
- **THEN** la vérification signale une violation qui nomme les deux crates

### Requirement: Toute crate du workspace est classée
Chaque crate membre du workspace MUST être rangée dans une couche ou déclarée comme outil. Une crate inconnue de la table des couches est signalée, pour obliger à décider de sa place au moment où on l'ajoute.

#### Scenario: Crate non classée
- **WHEN** le workspace contient une crate `ortelia-geo` absente de la table des couches
- **THEN** la vérification signale que `ortelia-geo` n'est rangée dans aucune couche

### Requirement: Pas de bibliothèque d'interface sous les interfaces
Les crates des couches 0 à 3 MUST NOT dépendre, directement ou transitivement et quel que soit le type de dépendance, d'une bibliothèque d'interface : egui et les crates dont le nom commence par `egui`, `eframe`, `epaint`, `winit`, GPUI et les crates dont le nom commence par `gpui`.

#### Scenario: Dépendance directe à egui refusée
- **WHEN** `ortelia-presentation` dépend directement de `egui`
- **THEN** la vérification signale une violation qui nomme `ortelia-presentation` et `egui`

#### Scenario: Dépendance transitive à egui refusée
- **WHEN** `ortelia-service` dépend d'une crate tierce qui dépend elle-même de `egui`
- **THEN** la vérification signale une violation et affiche le chemin de dépendance de `ortelia-service` jusqu'à `egui`

#### Scenario: Les interfaces peuvent dépendre d'egui
- **WHEN** `ortelia-ui-egui` dépend de `egui` et de `eframe`
- **THEN** la vérification ne signale rien pour ces dépendances

### Requirement: Compte rendu de la vérification
La vérification des couches SHALL se lancer par une commande unique depuis la racine du dépôt (`cargo xtask check-layers`), signaler toutes les violations en une seule exécution, et se terminer avec un code non nul dès qu'il y en a au moins une.

#### Scenario: Plusieurs violations signalées ensemble
- **WHEN** le workspace contient deux violations différentes
- **THEN** la vérification affiche les deux et se termine avec un code non nul
