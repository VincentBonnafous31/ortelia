# app-launch Specification

## Purpose

Définir le lancement de l'exécutable `ortelia` : les options de la ligne de commande, et l'ouverture de la fenêtre principale dont le contenu est calculé par la couche de présentation puis dessiné par l'interface.

## Requirements

### Requirement: Affichage de la version
Avec l'option `--version`, l'exécutable SHALL écrire `ortelia <version>` sur la sortie standard, suivi d'un saut de ligne, puis se terminer avec le code 0 sans ouvrir de fenêtre. `<version>` est la version du workspace.

#### Scenario: Version demandée
- **WHEN** on lance `ortelia --version`
- **THEN** la sortie standard contient exactement `ortelia 0.1.0` suivi d'un saut de ligne, et le code de sortie est 0

### Requirement: Aide de la ligne de commande
Avec l'option `--help`, l'exécutable SHALL écrire sur la sortie standard un court texte d'usage qui liste les options reconnues, puis se terminer avec le code 0 sans ouvrir de fenêtre.

#### Scenario: Aide demandée
- **WHEN** on lance `ortelia --help`
- **THEN** la sortie standard mentionne `--version` et `--help`, et le code de sortie est 0

### Requirement: Argument non reconnu
Un argument non reconnu MUST provoquer un message d'erreur sur la sortie d'erreur, qui cite l'argument et rappelle `--help`, et la fin du programme avec le code 2, sans ouvrir de fenêtre.

#### Scenario: Option inconnue
- **WHEN** on lance `ortelia --frobnicate`
- **THEN** la sortie d'erreur contient `--frobnicate` et `--help`, et le code de sortie est 2

### Requirement: Contenu de la fenêtre principale fourni par la présentation
La couche de présentation SHALL fournir le titre de la fenêtre principale, `Ortelia`, et le texte affiché en son centre, `Ortelia <version>`. L'interface ne calcule pas ces textes.

#### Scenario: Textes de la fenêtre principale
- **WHEN** la présentation est créée, sans interface
- **THEN** le titre de la fenêtre vaut `Ortelia` et le texte central vaut `Ortelia 0.1.0`

### Requirement: Affichage de la fenêtre principale
Sans argument, l'exécutable SHALL ouvrir une fenêtre native dont le titre et le texte central sont ceux fournis par la présentation.

#### Scenario: Titre de la fenêtre native
- **WHEN** l'interface prépare les options de la fenêtre native à partir de la présentation
- **THEN** le titre demandé au système est `Ortelia`

#### Scenario: Texte central affiché
- **WHEN** l'application egui complète dessine une image, dans un test sans écran
- **THEN** un libellé `Ortelia 0.1.0` est présent

#### Scenario: Démarrage réel
- **WHEN** on lance `ortelia` sans argument, avec un affichage réel ou virtuel
- **THEN** dans les 20 secondes, le journal indique que la fenêtre principale est affichée, le processus tourne encore, et le journal ne contient aucune ligne `ERROR` émise par Ortelia (les lignes relayées des bibliothèques tierces ne comptent pas)

### Requirement: Première image journalisée
Quand la fenêtre principale a dessiné sa première image, l'application SHALL journaliser au niveau `INFO`, une seule fois, que la fenêtre principale est affichée.

#### Scenario: Événement unique
- **WHEN** l'application egui complète dessine plusieurs images, dans un test sans écran
- **THEN** l'événement « fenêtre principale affichée » est journalisé exactement une fois

### Requirement: Échec de l'ouverture de la fenêtre
Si la fenêtre ne peut pas être ouverte (par exemple, aucun affichage disponible), l'exécutable MUST écrire un message d'erreur compréhensible sur la sortie d'erreur et dans le journal, puis se terminer avec le code 1, sans paniquer.

#### Scenario: Aucun affichage disponible sous Linux
- **WHEN** on lance `ortelia` sous Linux sans `DISPLAY`, `WAYLAND_DISPLAY` ni `WAYLAND_SOCKET`
- **THEN** la sortie d'erreur contient un message d'échec de l'ouverture de la fenêtre, et le code de sortie est 1
