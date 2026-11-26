# Spec Delta

## ADDED Requirements

### Requirement: Avertissements de la configuration
L'ouverture SHALL rapporter, avec l'application ouverte, les avertissements de la vérification de `app.json` contre le modèle, comme les clés de libellés qui ne désignent rien. Ces avertissements MUST NOT empêcher l'ouverture.

#### Scenario: Ouverture avec avertissement
- **WHEN** on ouvre une application dont un libellé a la clé `Personne.nmo`
- **THEN** l'ouverture réussit, les sources sont chargées, et l'application ouverte porte un avertissement qui cite `Personne.nmo`

#### Scenario: Annuaire sans avertissement de configuration
- **WHEN** on ouvre `examples/annuaire/app.json`
- **THEN** l'application ouverte ne porte aucun avertissement de configuration
