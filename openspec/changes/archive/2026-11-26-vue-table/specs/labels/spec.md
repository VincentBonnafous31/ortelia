# Spec Delta

## Purpose

Donner aux éléments affichés d'une application (classes, attributs, littéraux d'énumérés) des libellés traduisibles, déclarés dans `app.json`, et rendre visibles les libellés manquants ou fautifs dès le lancement.

## ADDED Requirements

### Requirement: Déclaration des libellés
`app.json` SHALL déclarer ses langues dans `languages`, une liste non vide de codes de langue, et ses libellés dans `labels`, une liste optionnelle de groupes, un par langue, chacun avec sa langue (`lang`) et ses entrées (`entries`), des paires `key` / `text`.

#### Scenario: Libellés de l'annuaire
- **WHEN** on lit `examples/annuaire/app.json`
- **THEN** les langues sont `fr` puis `en`, et la clé `Personne.nom` a pour texte `Nom` en français et `Last name` en anglais

### Requirement: Forme des clés
Une clé SHALL désigner une classe (`Personne`), un attribut d'une classe (`Personne.nom`) ou un littéral d'un énuméré (`SituationFamiliale.MARIE`).

#### Scenario: Libellé d'un littéral
- **WHEN** le groupe `fr` contient la clé `SituationFamiliale.MARIE` de texte `Marié(e)`
- **THEN** le libellé français du littéral `MARIE` est `Marié(e)`

### Requirement: Langue courante
La langue courante SHALL être la première de `languages`. Les libellés SHALL être cherchés dans cette seule langue.

#### Scenario: Première langue
- **WHEN** `languages` vaut `["fr", "en"]`
- **THEN** l'en-tête de la colonne `nom` est `Nom`

### Requirement: Libellé d'un attribut hérité
Pour un attribut affiché avec une classe, le libellé SHALL être cherché d'abord sous la clé de cette classe, puis sous celles de ses classes mères, en remontant jusqu'à la classe qui déclare l'attribut.

#### Scenario: Sous-classe qui renomme
- **WHEN** `Employe` hérite de `Personne`, que `Personne.nom` a pour texte `Nom` et `Employe.nom` pour texte `Nom d'usage`
- **THEN** la colonne `nom` d'une table d'employés a pour en-tête `Nom d'usage`, et celle d'une table de personnes `Nom`

#### Scenario: Libellé de la classe mère
- **WHEN** `Employe` hérite de `Personne`, et seul `Personne.nom` a un libellé
- **THEN** la colonne `nom` d'une table d'employés a pour en-tête ce libellé

### Requirement: Libellé manquant visible
Si aucun libellé n'est trouvé dans la langue courante, l'élément SHALL s'afficher sous la forme `#clé#`, avec la clé de la classe affichée, même si l'autre langue a un libellé.

#### Scenario: Aucun libellé
- **WHEN** aucun groupe ne contient de clé pour l'attribut `prenom`
- **THEN** l'en-tête de la colonne est `#Personne.prenom#`

#### Scenario: Libellé dans l'autre langue seulement
- **WHEN** la langue courante est `fr`, et seul le groupe `en` contient `Personne.prenom`
- **THEN** l'en-tête de la colonne est `#Personne.prenom#`

### Requirement: Avertissement des libellés manquants
Au lancement, chaque élément que la vue affichera sans libellé dans la langue courante (en-tête de colonne, littéral d'un énuméré affiché) SHALL produire un avertissement, une seule fois par clé, qui cite la clé et la langue. Ces avertissements SHALL être journalisés au niveau `WARN`.

#### Scenario: Deux libellés manquants
- **WHEN** l'annuaire est ouvert, et que les clés `Personne.prenom` et `SituationFamiliale.VEUF` manquent en français
- **THEN** il y a exactement deux avertissements de libellé manquant, qui citent ces deux clés et `fr`, même après plusieurs images

### Requirement: Clé qui ne désigne rien
Au lancement, une clé de `labels` qui ne désigne aucune classe, aucun attribut ni aucun littéral du modèle SHALL produire un avertissement qui cite la clé et la langue, sans empêcher l'ouverture.

#### Scenario: Faute de frappe
- **WHEN** le groupe `fr` contient la clé `Personne.nmo`
- **THEN** l'application s'ouvre, avec un avertissement qui cite `Personne.nmo` et `fr`

### Requirement: Clé en double
Une clé présente deux fois dans le groupe d'une même langue MUST empêcher l'ouverture, avec une erreur qui cite la clé et la langue.

#### Scenario: Doublon
- **WHEN** le groupe `fr` contient deux fois la clé `Personne.nom`
- **THEN** l'ouverture échoue avec une erreur qui cite `Personne.nom` et `fr`

### Requirement: Langue non déclarée
Un groupe de libellés dont la langue n'est pas dans `languages`, ou deux groupes de la même langue, MUST empêcher l'ouverture, avec une erreur qui cite la langue.

#### Scenario: Langue absente de la liste
- **WHEN** `languages` vaut `["fr", "en"]` et qu'un groupe a pour langue `de`
- **THEN** l'ouverture échoue avec une erreur qui cite `de`

#### Scenario: Deux groupes pour une langue
- **WHEN** deux groupes ont pour langue `fr`
- **THEN** l'ouverture échoue avec une erreur qui cite `fr`
