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
Avec l'option `--help`, l'exécutable SHALL écrire sur la sortie standard un court texte d'usage qui liste les options reconnues et l'argument de chemin de l'application, puis se terminer avec le code 0 sans ouvrir de fenêtre.

#### Scenario: Aide demandée
- **WHEN** on lance `ortelia --help`
- **THEN** la sortie standard mentionne `--version`, `--help` et le chemin de l'application, et le code de sortie est 0

### Requirement: Argument non reconnu
Un argument qui commence par `-` et n'est pas une option reconnue, ou un argument de plus après le chemin de l'application ou après une option, MUST provoquer un message d'erreur sur la sortie d'erreur, qui cite l'argument et rappelle `--help`, et la fin du programme avec le code 2, sans ouvrir de fenêtre.

#### Scenario: Option inconnue
- **WHEN** on lance `ortelia --frobnicate`
- **THEN** la sortie d'erreur contient `--frobnicate` et `--help`, et le code de sortie est 2

#### Scenario: Deux chemins
- **WHEN** on lance `ortelia examples/annuaire autre`
- **THEN** la sortie d'erreur contient `autre` et `--help`, et le code de sortie est 2

### Requirement: Contenu de la fenêtre principale fourni par la présentation
La couche de présentation SHALL fournir le titre de la fenêtre principale, sa taille initiale et son contenu, calculés depuis l'application ouverte : titre de `window`, taille de l'état de session s'il en donne une (voir `session-state`), sinon taille de `window`, et arbre de disposition de son contenu (voir `window-layout`). L'interface ne calcule pas ces éléments.

#### Scenario: Textes de la fenêtre principale
- **WHEN** la présentation est créée, sans interface et sans état de session, pour l'application `examples/annuaire` ouverte
- **THEN** le titre de la fenêtre vaut `Annuaire`, sa taille initiale 1000 × 700 points, et son contenu est le split `principal`, qui contient la table des personnes et la vue formulaire `proprietes`

#### Scenario: Taille de l'état de session
- **WHEN** la présentation est créée pour `examples/annuaire`, avec un état de session qui donne une fenêtre de 1200 × 800 points
- **THEN** le titre de la fenêtre vaut `Annuaire` et sa taille initiale 1200 × 800 points

#### Scenario: Application qui ne s'ouvre pas
- **WHEN** la présentation est créée pour une application dont l'ouverture a échoué
- **THEN** le titre de la fenêtre vaut `Ortelia`, et la fenêtre ne contient que la boîte d'erreur d'ouverture

### Requirement: Affichage de la fenêtre principale
L'exécutable SHALL ouvrir l'application désignée par la ligne de commande, puis une fenêtre native dont le titre, la taille initiale et le contenu sont ceux fournis par la présentation.

#### Scenario: Titre de la fenêtre native
- **WHEN** l'interface prépare les options de la fenêtre native à partir de la présentation de `examples/annuaire`
- **THEN** le titre demandé au système est `Annuaire`, et la taille intérieure 1000 × 700

#### Scenario: Texte central affiché
- **WHEN** l'application egui complète de `examples/annuaire` dessine une image, dans un test sans écran
- **THEN** l'en-tête `Nom` et la cellule `Dupont` sont présents

#### Scenario: Démarrage réel
- **WHEN** on lance `ortelia examples/annuaire`, avec un affichage réel ou virtuel
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

### Requirement: Chemin de l'application
L'exécutable SHALL accepter un argument facultatif, le chemin de l'application. Un dossier désigne le fichier `app.json` qu'il contient, un fichier se désigne lui-même, et sans argument, c'est le fichier `app.json` du dossier courant.

#### Scenario: Dossier
- **WHEN** on lance `ortelia examples/annuaire`
- **THEN** l'application ouverte est celle de `examples/annuaire/app.json`

#### Scenario: Fichier
- **WHEN** on lance `ortelia examples/annuaire/app.json`
- **THEN** l'application ouverte est celle de ce fichier

#### Scenario: Sans argument
- **WHEN** on lance `ortelia` depuis le dossier `examples/annuaire`
- **THEN** l'application ouverte est celle de `examples/annuaire/app.json`

### Requirement: Application qui ne s'ouvre pas
Si l'application ne peut pas s'ouvrir (fichier introuvable, `app.json` ou modèle fautif), la fenêtre SHALL afficher une boîte modale « Impossible d'ouvrir l'application » qui liste les erreurs, vingt au plus, suivies du nombre des autres, avec un bouton « Quitter » qui ferme la fenêtre. Les erreurs SHALL aussi être écrites sur la sortie d'erreur et journalisées au niveau `ERROR`, et le processus MUST se terminer avec le code 1, quelle que soit la façon dont la fenêtre est fermée.

#### Scenario: Dossier sans `app.json`
- **WHEN** on lance `ortelia` sur un dossier qui ne contient pas de `app.json`
- **THEN** la boîte d'erreur d'ouverture cite le fichier `app.json` attendu

#### Scenario: Bouton Quitter
- **WHEN** dans un test sans écran, on clique sur « Quitter » dans la boîte d'erreur d'ouverture
- **THEN** l'interface émet l'intention de quitter, et demande la fermeture de la fenêtre

#### Scenario: Erreurs nombreuses
- **WHEN** l'ouverture échoue avec 25 erreurs
- **THEN** la boîte en liste 20, puis indique qu'il y en a 5 autres

#### Scenario: Code de sortie
- **WHEN** on lance `ortelia` sous Linux sans affichage, sur un dossier qui ne contient pas de `app.json`
- **THEN** la sortie d'erreur cite le fichier `app.json` attendu, et le code de sortie est 1

### Requirement: Source en échec
Si l'application s'ouvre mais qu'une source n'a pas pu être chargée, la fenêtre SHALL afficher l'application, et par-dessus une boîte modale « Certaines données n'ont pas pu être chargées » qui liste les erreurs de chaque source en échec, avec un bouton « Continuer » qui ferme la boîte.

#### Scenario: Fichier de données fautif
- **WHEN** l'annuaire est ouvert avec un `personnes.json` qui n'est pas du JSON valide
- **THEN** la table est vide, et la boîte cite la source `personnes` et son erreur

#### Scenario: Bouton Continuer
- **WHEN** dans un test sans écran, on clique sur « Continuer »
- **THEN** l'interface émet l'intention de fermer la boîte, qui n'est plus affichée à l'image suivante

### Requirement: Barre d'état des avertissements
S'il y a des avertissements (configuration, libellés, diagnostics des sources), la fenêtre SHALL afficher en bas une barre d'état qui en donne le nombre. Un clic sur ce nombre SHALL ouvrir une boîte modale qui les liste, avec un bouton « Fermer ». Sans avertissement, la barre d'état n'est pas affichée.

#### Scenario: Libellés manquants
- **WHEN** l'annuaire est ouvert avec deux libellés manquants et aucun autre avertissement
- **THEN** la barre d'état indique 2 avertissements, et un clic ouvre la liste des deux clés

#### Scenario: Aucun avertissement
- **WHEN** l'application `examples/annuaire` est affichée
- **THEN** il n'y a pas de barre d'état
