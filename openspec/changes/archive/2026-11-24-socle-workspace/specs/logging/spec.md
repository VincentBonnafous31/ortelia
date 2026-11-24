# Spec Delta

## Purpose

Produire un journal de l'application dans des fichiers, pour diagnostiquer un problème après coup chez l'utilisateur : emplacement prévisible, rotation quotidienne, niveau réglable, et trace des paniques.

## ADDED Requirements

### Requirement: Emplacement des journaux
Les journaux SHALL être écrits dans un dossier `logs` propre à Ortelia : sous Linux `$XDG_STATE_HOME/ortelia/logs` (par défaut `~/.local/state/ortelia/logs`), sous Windows `%LOCALAPPDATA%\ortelia\logs`. La variable d'environnement `ORTELIA_LOG_DIR`, si elle est définie et non vide, remplace ce dossier. Le dossier est créé s'il n'existe pas.

#### Scenario: Dossier imposé par la variable d'environnement
- **WHEN** l'application démarre avec `ORTELIA_LOG_DIR` désignant un dossier qui n'existe pas encore
- **THEN** ce dossier est créé et le fichier journal du jour y est écrit

#### Scenario: Dossier par défaut sous Linux
- **WHEN** le dossier des journaux est calculé sous Linux avec `XDG_STATE_HOME=/tmp/etat` et sans `ORTELIA_LOG_DIR`
- **THEN** le dossier obtenu est `/tmp/etat/ortelia/logs`

### Requirement: Un fichier par jour, en nombre limité
L'application SHALL écrire dans un fichier par jour, nommé `ortelia.<AAAA-MM-JJ>.log` d'après la date en temps universel (UTC), et MUST conserver au plus les 7 fichiers les plus récents. Les horodatages des lignes sont eux aussi en UTC.

#### Scenario: Nom du fichier du jour
- **WHEN** un événement est journalisé le 24 novembre 2026 (date UTC)
- **THEN** il est écrit dans `ortelia.2026-11-24.log`

#### Scenario: Anciens fichiers supprimés
- **WHEN** le dossier contient déjà 7 fichiers journaux de jours précédents et qu'un fichier est créé pour un nouveau jour
- **THEN** le dossier ne contient plus que les 7 fichiers les plus récents

### Requirement: Niveau de journalisation réglable
Le niveau par défaut SHALL être `info`. La variable d'environnement `ORTELIA_LOG` MUST permettre de le changer, globalement (`debug`) ou par crate (`ortelia_service=trace`) ; un réglage par crate laisse le niveau `info` aux autres crates. Une valeur invalide est ignorée : le niveau par défaut s'applique et un avertissement est journalisé.

#### Scenario: Niveau par défaut
- **WHEN** `ORTELIA_LOG` n'est pas définie
- **THEN** un événement `info` est écrit dans le journal et un événement `debug` ne l'est pas

#### Scenario: Niveau demandé
- **WHEN** `ORTELIA_LOG=debug`
- **THEN** un événement `debug` est écrit dans le journal

#### Scenario: Niveau par crate
- **WHEN** `ORTELIA_LOG=une_autre_crate=trace`
- **THEN** un événement `info` d'Ortelia est encore écrit dans le journal, et un événement `debug` d'Ortelia ne l'est pas

#### Scenario: Valeur invalide
- **WHEN** `ORTELIA_LOG` contient une valeur qui n'est pas un filtre valide
- **THEN** le niveau `info` s'applique et le journal contient un avertissement qui signale la valeur ignorée

### Requirement: Avertissements des bibliothèques tierces
Les avertissements et les erreurs que les bibliothèques tierces (eframe, winit, wgpu…) émettent par la crate `log` SHALL être écrits dans le même journal, avec le nom de leur module d'origine. Leurs messages de niveau inférieur à `warn` ne le sont pas.

#### Scenario: Messages d'une bibliothèque tierce
- **WHEN** une bibliothèque tierce émet par `log` un avertissement et un message `info`
- **THEN** le journal contient l'avertissement, au niveau `WARN` et avec la cible de la bibliothèque, mais pas le message `info`

### Requirement: Démarrage et arrêt journalisés
Au démarrage, avant l'ouverture de la fenêtre, l'application SHALL journaliser au niveau `info` sa version, le système d'exploitation et l'architecture. À la fin normale du programme, elle SHALL journaliser son arrêt. Les options `--version` et `--help` ne journalisent rien.

#### Scenario: Ligne de démarrage
- **WHEN** l'application démarre sans argument
- **THEN** le fichier journal contient une ligne de niveau `INFO` avec la version `0.1.0`, le système d'exploitation et l'architecture

### Requirement: Paniques journalisées
Si l'application panique, le message de la panique et son emplacement dans le code MUST être écrits dans le journal au niveau `ERROR`, en plus du comportement habituel de la panique.

#### Scenario: Panique dans l'application
- **WHEN** une panique se produit après l'initialisation du journal
- **THEN** le fichier journal contient une ligne `ERROR` avec le message de la panique et son fichier source

### Requirement: Journal indisponible
Si le dossier des journaux ne peut pas être créé ou si le fichier ne peut pas être ouvert, l'application MUST continuer sans journal fichier et écrire un avertissement sur la sortie d'erreur, qui cite le dossier concerné.

#### Scenario: Dossier impossible à créer
- **WHEN** `ORTELIA_LOG_DIR` désigne un chemin situé sous un fichier ordinaire
- **THEN** l'application poursuit son démarrage, et la sortie d'erreur contient un avertissement qui cite ce chemin
