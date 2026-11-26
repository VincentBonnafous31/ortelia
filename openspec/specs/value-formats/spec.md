# value-formats Specification

## Purpose

Définir le texte affiché pour chaque type de valeur d'un attribut, selon la langue courante et le format éventuel de la colonne, dont les motifs de date LDML, pour que les tables et, plus tard, les formulaires affichent les valeurs de la même façon.

## Requirements

### Requirement: Valeur absente
Un attribut sans valeur SHALL s'afficher comme une cellule vide, quel que soit son type.

#### Scenario: Notes absentes
- **WHEN** une personne n'a pas de `notes`
- **THEN** sa cellule `notes` est vide

### Requirement: Textes
Un texte SHALL s'afficher tel quel, sur une seule ligne : chaque saut de ligne ou tabulation SHALL être remplacé par une espace.

#### Scenario: Texte sur plusieurs lignes
- **WHEN** une note vaut `Ligne 1\nLigne 2`
- **THEN** sa cellule affiche `Ligne 1 Ligne 2`

### Requirement: Nombres
Un entier SHALL s'afficher en chiffres, sans séparateur de milliers, précédé de `-` s'il est négatif. Un décimal SHALL s'afficher avec le moins de chiffres qui le représente exactement, sans exposant, avec pour séparateur décimal la virgule en français et le point en anglais.

#### Scenario: Taille en français
- **WHEN** la langue courante est `fr` et que la taille vaut 1.68
- **THEN** la cellule affiche `1,68`

#### Scenario: Taille en anglais
- **WHEN** la langue courante est `en` et que la taille vaut 1.68
- **THEN** la cellule affiche `1.68`

#### Scenario: Grand entier
- **WHEN** un entier vaut 1234567
- **THEN** la cellule affiche `1234567`

### Requirement: Booléens
Un booléen SHALL s'afficher en case cochée ou non, non modifiable, et SHALL NOT avoir de texte pour le filtre.

#### Scenario: Contact autorisé
- **WHEN** une personne a `contact_autorise` à vrai
- **THEN** sa cellule est une case cochée, non modifiable

### Requirement: Énumérés
Un littéral d'énuméré SHALL s'afficher par son libellé dans la langue courante, ou `#Énuméré.LITTÉRAL#` s'il n'en a pas.

#### Scenario: Situation avec libellé
- **WHEN** le littéral `MARIE` a pour libellé français `Marié(e)`
- **THEN** la cellule d'une personne mariée affiche `Marié(e)`

### Requirement: Dates et dates-heures par défaut
Sans format de colonne, une date SHALL s'afficher selon `dd/MM/yyyy` en français et `yyyy-MM-dd` en anglais, une date-heure selon `dd/MM/yyyy HH:mm:ss` en français et `yyyy-MM-dd HH:mm:ss` en anglais. Les dates-heures SHALL être affichées en UTC.

#### Scenario: Date de naissance
- **WHEN** la langue courante est `fr` et que la date de naissance vaut 1985-03-12
- **THEN** la cellule affiche `12/03/1985`

#### Scenario: Inscription en UTC
- **WHEN** la langue courante est `fr` et que l'inscription vaut `2024-01-15T09:30:00Z`
- **THEN** la cellule affiche `15/01/2024 09:30:00`, quel que soit le fuseau horaire du poste

### Requirement: Motifs de date LDML
Le format d'une colonne de date ou de date-heure SHALL être un motif LDML. Champs reconnus : `yy`, `yyyy` ; `M`, `MM`, `MMM`, `MMMM` ; `d`, `dd` ; `EEE`, `EEEE` ; pour les dates-heures, `H`, `HH`, `m`, `mm`, `s`, `ss` et `S` à `SSSSSS` (1 à 6 chiffres de fraction). Les noms suivent la langue courante. Le texte entre apostrophes et les caractères qui ne sont pas des lettres sont recopiés ; `''` donne une apostrophe.

#### Scenario: Mois en toutes lettres
- **WHEN** une colonne de date a pour format `d MMMM yyyy`, que la langue courante est `fr` et que la date vaut 1985-03-12
- **THEN** la cellule affiche `12 mars 1985`

#### Scenario: Même motif en anglais
- **WHEN** la même colonne est affichée avec la langue courante `en`
- **THEN** la cellule affiche `12 March 1985`

#### Scenario: Jour de la semaine et texte entre apostrophes
- **WHEN** une colonne de date-heure a pour format `EEEE d MMM 'à' HH'h'mm`, que la langue courante est `fr` et que la valeur vaut `2024-01-15T09:30:00Z`
- **THEN** la cellule affiche `lundi 15 janv. à 09h30`

#### Scenario: Microsecondes
- **WHEN** une colonne de date-heure a pour format `HH:mm:ss.SSSSSS` et que la valeur vaut `2026-11-23T10:15:30.123456Z`
- **THEN** la cellule affiche `10:15:30.123456`

### Requirement: Motif invalide refusé
Un motif qui contient une lettre hors des champs reconnus, une apostrophe non fermée, ou un champ d'heure pour une colonne de date MUST empêcher l'ouverture, avec une erreur qui cite la colonne et le motif. Un format pour une colonne qui n'est ni une date ni une date-heure MUST aussi empêcher l'ouverture.

#### Scenario: Lettre inconnue
- **WHEN** une colonne de date a pour format `dd/MM/yyyy Q`
- **THEN** l'ouverture échoue avec une erreur qui cite la colonne et `dd/MM/yyyy Q`

#### Scenario: Heure pour une date
- **WHEN** la colonne `date_naissance`, de type `Date`, a pour format `dd/MM/yyyy HH:mm`
- **THEN** l'ouverture échoue avec une erreur qui cite `date_naissance`

#### Scenario: Format sur un entier
- **WHEN** la colonne `nombre_enfants` a un format
- **THEN** l'ouverture échoue avec une erreur qui cite `nombre_enfants`

### Requirement: Alignement par défaut
Sans alignement de colonne, les nombres SHALL être alignés à droite, les booléens, dates et dates-heures centrés, les textes et énumérés à gauche. L'alignement de la colonne (`left`, `center`, `right`) SHALL remplacer celui du type.

#### Scenario: Alignements de l'annuaire
- **WHEN** l'annuaire est affiché avec les colonnes déduites du modèle
- **THEN** `nombre_enfants` et `taille` sont alignées à droite, `contact_autorise`, `date_naissance` et `inscription` centrées, et les autres à gauche
