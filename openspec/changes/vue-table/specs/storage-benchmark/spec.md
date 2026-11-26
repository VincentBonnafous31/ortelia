# Spec Delta

## ADDED Requirements

### Requirement: Générateur de l'annuaire
La commande `cargo xtask gen-data --model annuaire --count <n> --out <dossier>` SHALL écrire dans le dossier une application de l'annuaire complète : `app.json` et `annuaire.model.json`, recopiés de `examples/annuaire`, et un `personnes.json` de `n` personnes conformes au modèle, avec des valeurs absentes pour les attributs facultatifs. Les mêmes paramètres MUST produire des fichiers identiques. Sans `--model`, le générateur reste celui du modèle d'essai.

#### Scenario: Application ouverte
- **WHEN** on génère une application de 1 000 personnes, puis qu'on l'ouvre
- **THEN** l'ouverture réussit sans erreur ni diagnostic, et l'annuaire contient 1 000 personnes

#### Scenario: Même graine, mêmes fichiers
- **WHEN** on génère deux applications de 500 personnes avec la même graine
- **THEN** leurs fichiers `personnes.json` sont identiques

#### Scenario: Noms variés
- **WHEN** on génère une application de 1 000 personnes
- **THEN** des noms comportent des accents ou des majuscules à l'intérieur (`Émile`, `de La Tour`…), et des personnes n'ont pas de situation
