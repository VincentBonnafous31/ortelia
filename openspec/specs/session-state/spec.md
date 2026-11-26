# session-state Specification

## Purpose

Enregistrer à la fermeture la disposition choisie par l'utilisateur (taille de la fenêtre, disposition des tables) et la restaurer à l'ouverture suivante, dans un fichier propre à l'utilisateur et à l'application, sans jamais modifier `app.json`.

## Requirements

### Requirement: Contenu de l'état de session
L'état de session d'une application SHALL contenir la taille de la fenêtre, en points, et, pour chaque vue table, désignée par son identifiant : l'ordre des colonnes, désignées par leur chemin, la largeur de chacune, le tri (chemin et sens de chaque critère, dans l'ordre) et le texte du filtre. Il SHALL être calculé par la couche de présentation, sans interface.

#### Scenario: Disposition modifiée
- **WHEN** l'annuaire est ouvert, qu'on déplace la colonne `date_naissance` en première position, élargit `nom` à 200 points, trie par `situation` croissant puis `nom` décroissant, filtre sur `dup`, et que la fenêtre mesure 1200 × 800 points
- **THEN** l'état de session donne une fenêtre de 1200 × 800 points, et pour la vue `personnes` : les colonnes `date_naissance`, `nom`, `prenom`, `nombre_enfants`, `taille`, `contact_autorise`, `situation`, `inscription`, `nom` à 200 points et les autres à leur largeur courante, le tri `situation` croissant puis `nom` décroissant, et le filtre `dup`

#### Scenario: Disposition inchangée
- **WHEN** l'annuaire est ouvert sans état de session, puis que son état est calculé sans aucune interaction
- **THEN** l'état donne une fenêtre de 1000 × 700 points, et les colonnes, largeurs et tri de la présentation de `Personne`, avec un filtre vide

### Requirement: Taille de la fenêtre suivie
L'interface SHALL signaler à la présentation, par une intention, chaque nouvelle taille de la fenêtre, en points, à partir de la première action de l'utilisateur dans la fenêtre (pointeur, molette, clavier), tant que la fenêtre n'est ni maximisée, ni en plein écran, ni réduite, et que sa surface n'est pas nulle ; jamais une taille inchangée. La présentation SHALL ignorer une taille non finie, et SHALL ramener la taille enregistrée dans les bornes de `window` (200 à 10 000 points).

#### Scenario: Fenêtre redimensionnée
- **WHEN** la fenêtre de l'annuaire passe à 1200 × 800 points
- **THEN** l'interface émet une seule intention de taille 1200 × 800, et l'état de session donne une fenêtre de 1200 × 800 points

#### Scenario: Fenêtre maximisée
- **WHEN** la fenêtre de l'annuaire est maximisée sur un écran de 1920 × 1080 points
- **THEN** l'interface n'émet aucune intention de taille, et l'état de session garde la dernière taille normale

#### Scenario: Taille provisoire avant toute action
- **WHEN** la fenêtre de l'annuaire mesure d'abord 640 × 480 points, avant que le système applique la taille demandée, puis se ferme sans que l'utilisateur ait agi dans la fenêtre
- **THEN** l'interface n'émet aucune intention de taille, et l'état de session donne la taille initiale, 1000 × 700 points

#### Scenario: Taille hors bornes
- **WHEN** la présentation reçoit une taille de 100 × 20 000 points
- **THEN** l'état de session donne une fenêtre de 200 × 10 000 points, et une nouvelle intention de 100 × 20 000 points n'est pas émise à l'image suivante

### Requirement: Fichier de l'état de session
L'état de session d'une application SHALL être enregistré à côté de son fichier d'application, sous le même nom dont l'extension `.json` devient `.session.json` : `app.session.json` pour `app.json`. Le fichier SHALL être un objet JSON dont la clé `formatVersion` vaut `"1"`, fait de listes d'objets, sans dictionnaires, comme les fichiers de configuration.

#### Scenario: Fichier de l'annuaire
- **WHEN** l'annuaire, copié dans un dossier temporaire `D`, est ouvert avec état de session, puis que son état est enregistré
- **THEN** le fichier `D/app.session.json` existe, et sa clé `formatVersion` vaut `"1"`

#### Scenario: Fichier d'application d'un autre nom
- **WHEN** l'application du fichier `D/annuaire.json` est ouverte avec état de session, puis que son état est enregistré
- **THEN** le fichier d'état est `D/annuaire.session.json`

### Requirement: Enregistrement à la fermeture
À la fermeture de la fenêtre principale, l'état de session de l'application ouverte SHALL être écrit dans son fichier d'état. Rien n'est écrit pour une application qui n'a pas pu être ouverte, ni pour une application ouverte sans état de session (tests, captures d'images). `app.json` SHALL NOT être modifié.

#### Scenario: Fermeture puis réouverture
- **WHEN** l'annuaire, copié dans un dossier temporaire, est ouvert dans l'interface avec état de session, qu'on élargit `nom` à 200 points et trie par `prenom`, que la fenêtre se ferme, puis que l'annuaire est rouvert avec état de session
- **THEN** la colonne `nom` a 200 points de large et la table est triée par `prenom` croissant

#### Scenario: Application qui ne s'ouvre pas
- **WHEN** la fenêtre d'une application dont l'ouverture a échoué se ferme
- **THEN** aucun fichier n'est écrit dans le dossier de l'application

#### Scenario: `app.json` intact
- **WHEN** l'annuaire, copié dans un dossier temporaire, est ouvert avec état de session, que sa disposition est modifiée puis son état enregistré
- **THEN** le contenu de son `app.json` est inchangé, octet pour octet

### Requirement: Écriture sûre
L'écriture du fichier d'état SHALL passer par un fichier temporaire du même dossier, renommé ensuite, pour qu'un fichier existant ne soit jamais laissé tronqué. Un échec de l'écriture, par exemple dans un dossier en lecture seule, SHALL être journalisé en `WARN`, avec le chemin du fichier, et SHALL NOT empêcher la fermeture.

#### Scenario: Remplacement d'un état existant
- **WHEN** l'état de session de l'annuaire, copié dans un dossier temporaire, est enregistré deux fois de suite, avec deux tris différents
- **THEN** le fichier d'état contient le second tri, et le dossier ne contient aucun fichier de plus que `app.json`, le modèle, les données et `app.session.json`

#### Scenario: Écriture impossible
- **WHEN** le fichier d'état désigne un chemin situé sous un fichier ordinaire
- **THEN** l'enregistrement journalise un avertissement qui cite le chemin, et ne panique pas

### Requirement: Restauration à l'ouverture
À l'ouverture d'une application avec état de session, si son fichier d'état existe, la présentation SHALL restaurer la taille de la fenêtre et, pour chaque vue table de l'application dont l'identifiant y figure, l'ordre des colonnes, leurs largeurs, le tri et le filtre enregistrés. Sans fichier, elle s'ouvre avec la disposition de `app.json`, sans avertissement. L'exécutable `ortelia` ouvre toujours avec état de session.

#### Scenario: Disposition retrouvée
- **WHEN** l'état de session de l'annuaire enregistre les colonnes `prenom` puis `nom` en tête, `nom` à 180 points, le tri `nom` décroissant, le filtre `dup` et une fenêtre de 1200 × 800 points, et que l'annuaire est ouvert avec état de session
- **THEN** les deux premières colonnes sont `prenom` et `nom`, `nom` a 180 points de large, la table ne montre que Dupont, son compteur vaut `1 / 10`, le filtre vaut `dup` et la fenêtre mesure 1200 × 800 points

#### Scenario: Pas de fichier
- **WHEN** l'annuaire, copié dans un dossier temporaire sans fichier d'état, est ouvert avec état de session
- **THEN** la table a les colonnes, les largeurs et le tri de la présentation de `Personne`, la fenêtre mesure 1000 × 700 points, et il n'y a aucun avertissement de plus

#### Scenario: Ouverture sans état de session
- **WHEN** l'annuaire, copié dans un dossier temporaire avec un fichier d'état qui trie par `prenom`, est ouvert sans état de session
- **THEN** il a la disposition de `app.json`, et l'enregistrement de son état n'écrit rien

### Requirement: Éléments disparus ignorés
`app.json` a pu changer depuis l'enregistrement : une vue, une colonne ou un critère de tri enregistrés qui ne désignent aucune vue table de l'application, ou aucune colonne de sa table, SHALL être ignorés, de même qu'une colonne ou un critère en double (le premier compte). Le reste de l'état SHALL être restauré, sans erreur ni avertissement à l'écran.

#### Scenario: Colonne disparue
- **WHEN** l'état enregistré de la vue `personnes` cite une colonne `surnom`, que la présentation n'a pas
- **THEN** la table s'ouvre sans colonne `surnom`, les autres colonnes enregistrées gardent leur ordre et leur largeur

#### Scenario: Tri sur une colonne disparue
- **WHEN** l'état enregistré trie par `surnom` croissant puis `nom` décroissant
- **THEN** la table est triée par `nom` décroissant seulement

#### Scenario: Vue inconnue
- **WHEN** l'état enregistré décrit une vue `contacts`, que l'application n'a pas, et la vue `personnes`
- **THEN** la vue `personnes` est restaurée, et la vue `contacts` est ignorée

#### Scenario: Colonne en double
- **WHEN** l'état enregistré cite la colonne `nom` deux fois, à 180 puis à 300 points
- **THEN** la table n'a qu'une colonne `nom`, de 180 points

### Requirement: Colonnes nouvelles ajoutées
Une colonne de la table absente de l'état enregistré de sa vue SHALL être ajoutée après les colonnes restaurées, dans l'ordre de la présentation, avec la largeur de la présentation.

#### Scenario: Colonne nouvelle
- **WHEN** l'état enregistré de la vue `personnes` ne cite que les colonnes `prenom` et `nom`
- **THEN** la table a d'abord `prenom` et `nom`, puis `date_naissance`, `nombre_enfants`, `taille`, `contact_autorise`, `situation` et `inscription`, avec les largeurs de la présentation

### Requirement: Valeurs ramenées dans leurs bornes
Une largeur enregistrée SHALL être ramenée dans les bornes d'une colonne (20 à 2 000 points), et une taille de fenêtre dans celles de la fenêtre (200 à 10 000 points). Une valeur qui n'est pas un nombre fini, ou qui manque, est ignorée : la valeur de `app.json` s'applique.

#### Scenario: Largeur hors bornes
- **WHEN** l'état enregistré donne 5 points à la colonne `nom` et 9 000 à `prenom`
- **THEN** la colonne `nom` a 20 points de large et `prenom` 2 000

### Requirement: Fichier d'état illisible ignoré
Un fichier d'état qui n'est pas du JSON valide, dont `formatVersion` n'est pas `"1"`, ou dont une valeur n'a pas le type attendu SHALL être ignoré en entier : l'application s'ouvre avec la disposition de `app.json`, et un avertissement est journalisé en `WARN`, avec le chemin du fichier et, pour une erreur de syntaxe ou de type, sa ligne et sa colonne. Le fichier ignoré SHALL être remplacé à la fermeture suivante.

#### Scenario: JSON invalide
- **WHEN** le fichier d'état de l'annuaire contient `{ "formatVersion": "1", `
- **THEN** l'annuaire s'ouvre avec la disposition de `app.json`, et le journal contient un avertissement qui cite le chemin du fichier, la ligne et la colonne

#### Scenario: Autre version de format
- **WHEN** le fichier d'état de l'annuaire a pour `formatVersion` `"2"`
- **THEN** l'annuaire s'ouvre avec la disposition de `app.json`, et le journal contient un avertissement qui cite le chemin du fichier et la version `2`

#### Scenario: Fichier ignoré puis remplacé
- **WHEN** le fichier d'état de l'annuaire n'est pas du JSON valide, que l'annuaire est ouvert puis fermé
- **THEN** le fichier contient un état valide, de `formatVersion` `"1"`

### Requirement: Clés inconnues ignorées
Les clés inconnues du fichier d'état SHALL être ignorées, à tous les niveaux, pour qu'un fichier écrit par une version plus récente d'Ortelia, qui enregistre davantage, garde ce que la version courante comprend.

#### Scenario: Clé inconnue
- **WHEN** le fichier d'état de l'annuaire contient, en plus d'un état valide, une clé `language` à la racine et une clé `split` dans la vue `personnes`
- **THEN** l'état valide est restauré

### Requirement: Filtre absent du journal
Le texte du filtre est une donnée saisie par l'utilisateur : il SHALL être enregistré dans le fichier d'état, mais SHALL NOT apparaître dans le journal, ni à l'enregistrement, ni à la restauration, ni dans le message d'un fichier ignoré.

#### Scenario: Filtre enregistré puis restauré
- **WHEN** l'annuaire est fermé avec le filtre `dup`, puis rouvert, au niveau de journalisation `trace`
- **THEN** le fichier d'état contient `dup`, et aucune ligne du journal ne contient `dup`

#### Scenario: Filtre d'un type faux
- **WHEN** le fichier d'état de l'annuaire donne au filtre la valeur `["dup"]` au lieu d'un texte
- **THEN** le fichier est ignoré, et l'avertissement du journal ne contient pas `dup`
