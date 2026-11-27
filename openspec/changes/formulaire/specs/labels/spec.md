# Spec Delta

## MODIFIED Requirements

### Requirement: Forme des clés
Une clé SHALL désigner une classe (`Personne`), un attribut d'une classe (`Personne.nom`) ou un littéral d'un énuméré (`SituationFamiliale.MARIE`). Une clé citée par un formulaire, comme titre de groupe, texte fixe ou libellé d'un champ, SHALL pouvoir être libre, sans rien désigner du modèle (`Personne.identite`).

#### Scenario: Libellé d'un littéral
- **WHEN** le groupe `fr` contient la clé `SituationFamiliale.MARIE` de texte `Marié(e)`
- **THEN** le libellé français du littéral `MARIE` est `Marié(e)`

#### Scenario: Clé libre d'un formulaire
- **WHEN** le groupe `fr` contient la clé `Personne.identite` de texte `Identité`, citée comme titre d'un groupe du formulaire de `Personne`
- **THEN** le titre français de ce groupe est `Identité`

### Requirement: Clé qui ne désigne rien
Au lancement, une clé de `labels` qui ne désigne aucune classe, aucun attribut ni aucun littéral du modèle, et qu'aucun formulaire ne cite, SHALL produire un avertissement qui cite la clé et la langue, sans empêcher l'ouverture.

#### Scenario: Faute de frappe
- **WHEN** le groupe `fr` contient la clé `Personne.nmo`
- **THEN** l'application s'ouvre, avec un avertissement qui cite `Personne.nmo` et `fr`

#### Scenario: Clé citée par un formulaire
- **WHEN** le groupe `fr` contient la clé `Personne.identite`, citée comme titre d'un groupe d'un formulaire
- **THEN** l'application s'ouvre sans avertissement pour `Personne.identite`

### Requirement: Avertissement des libellés manquants
Au lancement, chaque élément que la vue affichera sans libellé dans la langue courante SHALL produire un avertissement, une seule fois par clé, qui cite la clé et la langue. C'est le cas d'un en-tête de colonne ou d'un littéral d'un énuméré affiché, et, dans un formulaire affiché, d'un libellé de champ, d'un titre de groupe ou d'un texte fixe. Ces avertissements SHALL être journalisés au niveau `WARN`.

#### Scenario: Deux libellés manquants
- **WHEN** l'annuaire est ouvert, et que les clés `Personne.prenom` et `SituationFamiliale.VEUF` manquent en français
- **THEN** il y a exactement deux avertissements de libellé manquant, qui citent ces deux clés et `fr`, même après plusieurs images

#### Scenario: Titre de groupe manquant
- **WHEN** l'annuaire est ouvert, et que la clé `Personne.identite` manque en français
- **THEN** il y a un avertissement de libellé manquant qui cite `Personne.identite` et `fr`, et le groupe a pour titre `#Personne.identite#`

#### Scenario: Libellé manquant dans la table et le formulaire
- **WHEN** l'annuaire est ouvert, et que la clé `Personne.nom`, montrée par la table et par le formulaire, manque en français
- **THEN** il y a un seul avertissement de libellé manquant pour `Personne.nom`
